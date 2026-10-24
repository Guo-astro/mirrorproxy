//! Credentials stay local and are never MCP tool arguments or results.
use anyhow::{ensure, Context};
use clap::{Args, Subcommand};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    time::Duration,
};

#[derive(Debug, Args)]
pub struct DownloadArgs {
    #[arg(long)]
    pub base_url: String,
    #[command(subcommand)]
    pub command: DownloadCommand,
}
#[derive(Debug, Subcommand)]
pub enum DownloadCommand {
    /// Save a download-only token read from stdin, or remove it with --logout.
    Auth {
        #[arg(long, conflicts_with = "logout", required_unless_present = "logout")]
        token_stdin: bool,
        #[arg(long)]
        logout: bool,
    },
    /// Check server permissions, DNS and redirect policy using HEAD.
    Check { url: String },
    /// Download into a new file; existing files are never overwritten.
    Fetch {
        url: String,
        #[arg(short, long)]
        output: PathBuf,
        #[arg(long, default_value_t = 2_147_483_648)]
        max_bytes: u64,
    },
}
fn base(value: &str) -> anyhow::Result<String> {
    let base = super::system::validated_base_url(value)?;
    let u = url::Url::parse(&base)?;
    ensure!(
        u.scheme() == "https" || matches!(u.host_str(), Some("localhost" | "127.0.0.1" | "[::1]")),
        "download authentication requires HTTPS (except loopback development)"
    );
    Ok(base)
}
fn credential_path(base: &str) -> anyhow::Result<PathBuf> {
    let home = super::user_home().context("user home is unavailable")?;
    Ok(home.join(".mirrorproxy/credentials").join(format!(
        "{}.token",
        hex::encode(Sha256::digest(base.as_bytes()))
    )))
}
fn valid_token(token: &str) -> bool {
    token.len() == 52
        && token.starts_with("mpd_")
        && token[4..].bytes().all(|b| b.is_ascii_alphanumeric())
}
fn token(base: &str) -> anyhow::Result<String> {
    if let Ok(token) = std::env::var("MIRRORPROXY_DOWNLOAD_TOKEN") {
        ensure!(valid_token(&token), "invalid download token in environment");
        return Ok(token);
    }
    let path = credential_path(base)?;
    super::reject_symlink(&path)?;
    let file=fs::File::open(&path).map_err(|_|anyhow::anyhow!("download token unavailable; configure it locally with mirrorproxy download --base-url ORIGIN auth --token-stdin"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        ensure!(
            file.metadata()?.permissions().mode() & 0o077 == 0,
            "download credential permissions must be 0600"
        );
    }
    let mut text = String::new();
    file.take(256).read_to_string(&mut text)?;
    let text = text.trim().to_owned();
    ensure!(valid_token(&text), "invalid stored download token");
    Ok(text)
}
fn client() -> anyhow::Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .no_proxy()
        // Workspace builds may enable reqwest compression through the server.
        // Preserve artifact bytes identically in standalone and workspace builds.
        .no_gzip()
        .no_brotli()
        .no_deflate()
        .no_zstd()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(10))
        .read_timeout(Duration::from_secs(30))
        .timeout(Duration::from_secs(86400))
        .build()?)
}
fn validate_url(value: &str) -> anyhow::Result<()> {
    ensure!(value.len() <= 8192, "download URL is too long");
    let u = url::Url::parse(value).map_err(|_| anyhow::anyhow!("invalid download URL"))?;
    ensure!(
        u.scheme() == "https"
            && u.port_or_known_default() == Some(443)
            && u.host_str().is_some()
            && u.username().is_empty()
            && u.password().is_none()
            && u.fragment().is_none(),
        "download target must be credential-free HTTPS on port 443"
    );
    Ok(())
}
async fn request(base: &str, url: &str, check: bool) -> anyhow::Result<reqwest::Response> {
    validate_url(url)?;
    let base = self::base(base)?;
    let token = token(&base)?;
    let response = client()?
        .get(format!(
            "{base}/api/download{}",
            if check { "/check" } else { "" }
        ))
        .bearer_auth(token)
        .query(&[("url", url)])
        .send()
        .await
        .map_err(|_| {
            anyhow::anyhow!("download service request failed; check instance connectivity and TLS")
        })?;
    ensure!(
        response.status().is_success(),
        "download service rejected request (HTTP {}); check permission, domain policy and quota",
        response.status().as_u16()
    );
    Ok(response)
}
pub async fn check(base: &str, url: &str) -> anyhow::Result<Value> {
    let mut response = request(base, url, true).await?;
    ensure!(
        response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| v.starts_with("application/json")),
        "download service returned non-JSON policy response"
    );
    let mut bytes = vec![];
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| anyhow::anyhow!("policy response interrupted"))?
    {
        ensure!(
            bytes.len() + chunk.len() <= 65536,
            "policy response too large"
        );
        bytes.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&bytes).map_err(|_| anyhow::anyhow!("invalid policy response"))
}
struct Temporary(PathBuf);
impl Drop for Temporary {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}
pub async fn fetch(base: &str, url: &str, output: &Path, max_bytes: u64) -> anyhow::Result<Value> {
    ensure!(
        (1..=64u64 << 30).contains(&max_bytes),
        "max_bytes must be 1 to 64 GiB"
    );
    super::reject_symlink(output)?;
    ensure!(!output.exists(), "output already exists; choose a new file");
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let parent = fs::canonicalize(parent).context("output directory is unavailable")?;
    let name = output.file_name().context("output must name a file")?;
    let output = parent.join(name);
    let mut response = request(base, url, false).await?;
    ensure!(
        response.status() == reqwest::StatusCode::OK,
        "partial download responses are not supported by this command"
    );
    ensure!(
        response.content_length().is_none_or(|n| n <= max_bytes),
        "download exceeds local file size limit"
    );
    let expected = response.content_length();
    let temporary_path = parent.join(format!(
        ".mirrorproxy-download-{}-{}",
        std::process::id(),
        super::TEMPORARY_FILE_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(&temporary_path)
        .context("cannot create download temporary file")?;
    let temp = Temporary(temporary_path);
    let mut size = 0u64;
    let mut hash = Sha256::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| anyhow::anyhow!("download interrupted; partial file removed"))?
    {
        size = size.saturating_add(chunk.len() as u64);
        ensure!(size <= max_bytes, "download exceeds local file size limit");
        file.write_all(&chunk)?;
        hash.update(&chunk);
    }
    ensure!(expected.is_none_or(|n| n == size), "incomplete download");
    file.sync_all()?;
    drop(file);
    // Hard-link creation publishes a complete file without any overwrite race.
    fs::hard_link(&temp.0, &output).context(
        "cannot publish output; filesystem must support hard links and destination must not exist",
    )?;
    Ok(json!({"path":output,"bytes":size,"sha256":hex::encode(hash.finalize())}))
}
pub fn run(args: DownloadArgs) -> anyhow::Result<()> {
    let base = base(&args.base_url)?;
    if let DownloadCommand::Auth {
        token_stdin: _,
        logout,
    } = args.command
    {
        let path = credential_path(&base)?;
        super::reject_symlink(&path)?;
        if logout {
            if path.exists() {
                fs::remove_file(path)?;
            }
            println!("Download credential removed.");
            return Ok(());
        }
        let mut token = String::new();
        std::io::stdin().take(256).read_to_string(&mut token)?;
        let token = token.trim();
        ensure!(
            valid_token(token),
            "expected a download-only token from the account page"
        );
        let parent = path.parent().unwrap();
        fs::create_dir_all(parent)?;
        super::reject_symlink(&path)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(parent, fs::Permissions::from_mode(0o700))?;
        }
        // Existing atomic writer creates private files and rejects symlink ancestors.
        super::write_atomic(&path, token)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
        }
        println!("Download credential saved for this instance.");
        return Ok(());
    }
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let result = runtime.block_on(async {
        match args.command {
            DownloadCommand::Check { url } => check(&base, &url).await,
            DownloadCommand::Fetch {
                url,
                output,
                max_bytes,
            } => fetch(&base, &url, &output, max_bytes).await,
            _ => unreachable!(),
        }
    })?;
    serde_json::to_writer_pretty(std::io::stdout().lock(), &result)?;
    println!();
    Ok(())
}
