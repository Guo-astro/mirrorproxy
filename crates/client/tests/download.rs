//! Shipped CLI/MCP against a deterministic local download service.
use serde_json::{json, Value};
use std::{
    fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::PathBuf,
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};
const TOKEN: &str = "mpd_012345678901234567890123456789012345678901234567";
struct Server {
    base: String,
    requests: Arc<Mutex<Vec<String>>>,
    stop: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
}
impl Server {
    fn new(mode: &'static str) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(vec![]));
        let stop = Arc::new(AtomicBool::new(false));
        let captures = requests.clone();
        let stopped = stop.clone();
        let worker = thread::spawn(move || {
            for stream in listener.incoming() {
                if stopped.load(Ordering::SeqCst) {
                    break;
                }
                let mut stream = stream.unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut bytes = vec![];
                let mut b = [0];
                while bytes.len() < 32768 && !bytes.ends_with(b"\r\n\r\n") {
                    if stream.read(&mut b).unwrap_or(0) == 0 {
                        break;
                    }
                    bytes.push(b[0]);
                }
                let request = String::from_utf8(bytes).unwrap();
                captures.lock().unwrap().push(request.clone());
                let response=match mode {
    "encoded"=>"HTTP/1.1 200 OK\r\nContent-Encoding: gzip\r\nContent-Length: 3\r\nConnection: close\r\n\r\nabc".to_owned(),
    "redirect"=>"HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:1/stolen\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_owned(),
    "interrupted"=>"HTTP/1.1 200 OK\r\nContent-Length: 100\r\nConnection: close\r\n\r\npartial".to_owned(),
    "denied"=>"HTTP/1.1 403 Forbidden\r\nContent-Length: 6\r\nConnection: close\r\n\r\nsecret".to_owned(),
    _ if request.starts_with("GET /api/download/check?")=>{let body=r#"{"allowed":true,"domain":"example.org"}"#;format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len())},
    _=>"HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: 3\r\nConnection: close\r\n\r\nabc".to_owned(),
   };
                let _ = stream.write_all(response.as_bytes());
            }
        });
        Self {
            base,
            requests,
            stop,
            worker: Some(worker),
        }
    }
    fn cmd(&self) -> Command {
        let mut c = Command::new(env!("CARGO_BIN_EXE_mirrorproxy"));
        c.env("MIRRORPROXY_DOWNLOAD_TOKEN", TOKEN);
        for k in [
            "HTTP_PROXY",
            "HTTPS_PROXY",
            "ALL_PROXY",
            "http_proxy",
            "https_proxy",
            "all_proxy",
        ] {
            c.env(k, "http://127.0.0.1:1");
        }
        c.env("NO_PROXY", "");
        c
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(self.base.trim_start_matches("http://"));
        self.worker.take().unwrap().join().unwrap();
    }
}
struct Root(PathBuf);
static ROOT_SEQUENCE: AtomicU64 = AtomicU64::new(0);
impl Root {
    fn new() -> Self {
        Self::with_timestamp(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
        )
    }

    fn with_timestamp(timestamp: u128) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "mirrorproxy-download-test-{}-{}-{}",
            std::process::id(),
            ROOT_SEQUENCE.fetch_add(1, Ordering::Relaxed),
            timestamp
        ));
        fs::create_dir(&dir).unwrap();
        // macOS temporary directories may have system symlink ancestors;
        // exercise downloads using the real fixture path while retaining the
        // production refusal of symlinked output/credential paths.
        Self(fs::canonicalize(dir).unwrap())
    }
}
impl Drop for Root {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn download_fixtures_with_identical_clock_ticks_have_independent_lifetimes() {
    let roots: Vec<_> = (0..32).map(|_| Root::with_timestamp(0)).collect();
    let paths: std::collections::HashSet<_> = roots.iter().map(|root| root.0.clone()).collect();
    assert_eq!(paths.len(), roots.len());
    for root in &roots {
        fs::write(root.0.join("retained"), b"owned by this fixture").unwrap();
    }
    let mut roots = roots;
    drop(roots.pop());
    for root in roots {
        assert_eq!(
            fs::read(root.0.join("retained")).unwrap(),
            b"owned by this fixture"
        );
    }
}
#[test]
fn complete_download_has_correct_digest_preserves_existing_files_and_uses_only_instance() {
    let s = Server::new("ok");
    let dir = Root::new();
    let output = dir.0.join("artifact with spaces.xz");
    let args = [
        "download",
        "--base-url",
        &s.base,
        "fetch",
        "https://example.org/source.xz",
        "--output",
        output.to_str().unwrap(),
    ];
    let result = s.cmd().args(args).output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let value: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(value["bytes"], 3);
    assert_eq!(
        value["sha256"],
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(fs::read(&output).unwrap(), b"abc");
    assert!(!s.cmd().args(args).output().unwrap().status.success());
    assert_eq!(fs::read(&output).unwrap(), b"abc");
    assert_eq!(s.requests.lock().unwrap().len(), 1);
    let request = s.requests.lock().unwrap()[0].to_lowercase();
    assert!(request.contains(&format!("authorization: bearer {}", TOKEN)));
    assert!(request.starts_with("get /api/download?url=https%3a%2f%2fexample.org%2fsource.xz"));
    assert!(!request.contains("cookie:"));
}
#[test]
fn failures_clean_partial_files_refuse_redirects_and_redact_service_bodies() {
    for mode in ["interrupted", "redirect", "denied"] {
        let s = Server::new(mode);
        let dir = Root::new();
        let output = dir.0.join("artifact");
        let r = s
            .cmd()
            .args([
                "download",
                "--base-url",
                &s.base,
                "fetch",
                "https://example.org/a",
                "--output",
                output.to_str().unwrap(),
            ])
            .output()
            .unwrap();
        assert!(!r.status.success(), "{mode}");
        assert!(!output.exists());
        assert_eq!(fs::read_dir(&dir.0).unwrap().count(), 0);
        assert!(!String::from_utf8_lossy(&r.stderr).contains("secret"));
        assert!(!String::from_utf8_lossy(&r.stderr).contains(TOKEN));
    }
}
#[test]
fn limits_and_unsafe_inputs_do_not_leave_outputs() {
    let s = Server::new("ok");
    let dir = Root::new();
    let output = dir.0.join("artifact");
    let r = s
        .cmd()
        .args([
            "download",
            "--base-url",
            &s.base,
            "fetch",
            "https://example.org/a",
            "--output",
            output.to_str().unwrap(),
            "--max-bytes",
            "2",
        ])
        .output()
        .unwrap();
    assert!(!r.status.success());
    assert!(!output.exists());
    for url in [
        "http://example.org/a",
        "https://user:secret@example.org/a",
        "https://example.org:444/a",
    ] {
        let r = s
            .cmd()
            .args(["download", "--base-url", &s.base, "check", url])
            .output()
            .unwrap();
        assert!(!r.status.success());
        assert!(!String::from_utf8_lossy(&r.stderr).contains("secret"));
    }
    assert_eq!(s.requests.lock().unwrap().len(), 1);
}
#[test]
fn mcp_checks_and_downloads_using_local_credential_without_returning_it() {
    let s = Server::new("ok");
    let dir = Root::new();
    let output = dir.0.join("artifact");
    let mut child = s
        .cmd()
        .args(["mcp", "--base-url", &s.base])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    for frame in [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"test","version":"1"}}}),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"check_download","arguments":{"url":"https://example.org/a"}}}),
        json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"download_file","arguments":{"url":"https://example.org/a","output":output}}}),
    ] {
        writeln!(stdin, "{frame}").unwrap();
    }
    drop(stdin);
    let r = child.wait_with_output().unwrap();
    assert!(r.status.success());
    let text = String::from_utf8(r.stdout).unwrap();
    assert!(!text.contains(TOKEN));
    let replies: Vec<Value> = text
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(replies[1]["result"]["isError"], false);
    assert_eq!(replies[2]["result"]["isError"], false);
    assert_eq!(fs::read(output).unwrap(), b"abc");
}
#[test]
fn credentials_are_origin_scoped_private_and_removable() {
    let s = Server::new("ok");
    let root = Root::new();
    let mut c = s.cmd();
    c.env_remove("MIRRORPROXY_DOWNLOAD_TOKEN")
        .env("HOME", &root.0)
        .env("USERPROFILE", &root.0);
    let mut child = c
        .args(["download", "--base-url", &s.base, "auth", "--token-stdin"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(TOKEN.as_bytes())
        .unwrap();
    assert!(child.wait().unwrap().success());
    let path = fs::read_dir(root.0.join(".mirrorproxy/credentials"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    assert_eq!(fs::read_to_string(&path).unwrap(), TOKEN);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    let mut c = s.cmd();
    c.env_remove("MIRRORPROXY_DOWNLOAD_TOKEN")
        .env("HOME", &root.0)
        .env("USERPROFILE", &root.0);
    assert!(c
        .args([
            "download",
            "--base-url",
            &s.base,
            "check",
            "https://example.org/a"
        ])
        .output()
        .unwrap()
        .status
        .success());
    let mut c = s.cmd();
    c.env_remove("MIRRORPROXY_DOWNLOAD_TOKEN")
        .env("HOME", &root.0)
        .env("USERPROFILE", &root.0);
    assert!(c
        .args(["download", "--base-url", &s.base, "auth", "--logout"])
        .output()
        .unwrap()
        .status
        .success());
    assert!(!path.exists());
}

#[test]
fn encoded_payload_bytes_are_preserved_even_when_workspace_enables_compression() {
    let s = Server::new("encoded");
    let root = Root::new();
    let output = root.0.join("artifact");
    let r = s
        .cmd()
        .args([
            "download",
            "--base-url",
            &s.base,
            "fetch",
            "https://example.org/a",
            "--output",
            output.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(r.status.success(), "{}", String::from_utf8_lossy(&r.stderr));
    assert_eq!(fs::read(output).unwrap(), b"abc");
}

#[cfg(unix)]
#[test]
fn symlinked_output_paths_are_refused_without_contacting_the_instance() {
    use std::os::unix::fs::symlink;
    let s = Server::new("ok");
    let root = Root::new();
    let real = root.0.join("real");
    fs::create_dir(&real).unwrap();
    let link = root.0.join("linked");
    symlink(&real, &link).unwrap();
    let output = link.join("artifact");
    let r = s
        .cmd()
        .args([
            "download",
            "--base-url",
            &s.base,
            "fetch",
            "https://example.org/a",
            "--output",
            output.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!r.status.success());
    assert!(!real.join("artifact").exists());
    assert!(s.requests.lock().unwrap().is_empty());
}
