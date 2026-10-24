//! Local, reversible Skill/MCP setup. Never runs commands from instance metadata.
use std::{
    collections::BTreeSet,
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{bail, ensure, Context};
use clap::{Args, ValueEnum};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use toml_edit::{DocumentMut, Item, Table};

const SKILL: &str = include_str!("../assets/mirrorproxy/SKILL.md");
const AGENTS: [Agent; 3] = [Agent::Codex, Agent::ClaudeCode, Agent::Cursor];

#[derive(Args, Debug)]
pub(super) struct SetupArgs {
    #[arg(
        long,
        required_unless_present = "rollback",
        conflicts_with = "rollback"
    )]
    base_url: Option<String>,
    /// Select agents explicitly for unattended use; repeat or separate with commas.
    #[arg(long, value_enum, value_delimiter = ',')]
    agent: Vec<Agent>,
    /// Omit to ask. Global (current user) is strongly recommended and the default.
    #[arg(long, value_enum)]
    scope: Option<Scope>,
    #[arg(long)]
    project_dir: Option<PathBuf>,
    /// Explicitly allow replacing conflicting MirrorProxy entries after backup.
    #[arg(long)]
    replace_existing: bool,
    #[arg(long)]
    dry_run: bool,
    /// Restore one setup's files; refuses files edited since installation.
    #[arg(long, conflicts_with_all = ["agent", "replace_existing", "dry_run"])]
    rollback: Option<String>,
}

#[derive(Clone, Copy, Debug, ValueEnum, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
enum Agent {
    Codex,
    ClaudeCode,
    Cursor,
}

impl Agent {
    fn name(self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::ClaudeCode => "claude-code",
            Self::Cursor => "cursor",
        }
    }
    fn directory(self) -> &'static str {
        match self {
            Self::Codex => ".codex",
            Self::ClaudeCode => ".claude",
            Self::Cursor => ".cursor",
        }
    }
    fn executable(self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::ClaudeCode => "claude",
            Self::Cursor => "cursor",
        }
    }
}

fn on_path(name: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|paths| {
        std::env::split_paths(&paths).any(|path| {
            if cfg!(windows) {
                ["exe", "cmd", "bat"]
                    .into_iter()
                    .any(|extension| path.join(format!("{name}.{extension}")).is_file())
            } else {
                path.join(name).is_file()
            }
        })
    })
}

#[derive(Clone, Copy, Debug, ValueEnum, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum Scope {
    Global,
    Project,
}

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
enum Kind {
    Skill,
    Mcp,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Change {
    agent: Agent,
    kind: Kind,
    before: Option<String>,
    installed: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    version: u32,
    scope: Scope,
    root: PathBuf,
    changes: Vec<Change>,
}

struct Locations {
    home: PathBuf,
    root: PathBuf,
    scope: Scope,
    codex_home: PathBuf,
}

impl Locations {
    fn path(&self, agent: Agent, kind: Kind) -> PathBuf {
        if kind == Kind::Skill {
            let directory = if agent == Agent::Codex {
                ".agents"
            } else {
                agent.directory()
            };
            return self
                .root
                .join(directory)
                .join("skills/mirrorproxy/SKILL.md");
        }
        match agent {
            Agent::Codex if self.scope == Scope::Global => self.codex_home.join("config.toml"),
            Agent::Codex => self.root.join(".codex/config.toml"),
            Agent::ClaudeCode if self.scope == Scope::Global => self.home.join(".claude.json"),
            Agent::ClaudeCode => self.root.join(".mcp.json"),
            Agent::Cursor => self.root.join(".cursor/mcp.json"),
        }
    }
    fn record(&self, id: &str) -> anyhow::Result<PathBuf> {
        ensure!(
            !id.is_empty()
                && id.len() <= 100
                && id.bytes().all(|b| b.is_ascii_digit() || b == b'-'),
            "invalid rollback ID"
        );
        // Backups can contain credentials from other MCP entries. Keep them
        // outside the project, including for project-scoped installations.
        Ok(self
            .home
            .join(".mirrorproxy/agent-setups")
            .join(id)
            .join("manifest.json"))
    }
}

struct Terminal {
    input: BufReader<File>,
    output: File,
}

impl Terminal {
    fn open() -> anyhow::Result<Self> {
        #[cfg(unix)]
        let (input, output) = ("/dev/tty", "/dev/tty");
        #[cfg(windows)]
        let (input, output) = ("CONIN$", "CONOUT$");
        let input = File::open(input).context("no interactive terminal; specify --agent and --scope (conflicts also require --replace-existing)")?;
        let output = OpenOptions::new().write(true).open(output)?;
        Ok(Self {
            input: BufReader::new(input),
            output,
        })
    }
    fn ask(&mut self, message: &str) -> anyhow::Result<String> {
        write!(self.output, "{message} ")?;
        self.output.flush()?;
        let mut answer = String::new();
        ensure!(
            self.input.read_line(&mut answer)? != 0,
            "installation cancelled: terminal closed"
        );
        Ok(answer.trim().to_string())
    }
}

fn scope_answer(answer: &str) -> anyhow::Result<Scope> {
    match answer.trim() {
        "" | "1" | "global" => Ok(Scope::Global),
        "2" | "project" => Ok(Scope::Project),
        _ => bail!("choose 1 (global) or 2 (project)"),
    }
}

fn agent_answer(answer: &str, defaults: &[Agent]) -> anyhow::Result<Vec<Agent>> {
    if answer.trim().is_empty() {
        ensure!(
            !defaults.is_empty(),
            "no agents detected; select at least one agent"
        );
        return Ok(defaults.to_vec());
    }
    let mut agents = BTreeSet::new();
    for item in answer
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|s| !s.is_empty())
    {
        let agent = match item {
            "1" | "codex" => Agent::Codex,
            "2" | "claude-code" => Agent::ClaudeCode,
            "3" | "cursor" => Agent::Cursor,
            _ => bail!("select 1/codex, 2/claude-code, or 3/cursor, separated by commas"),
        };
        agents.insert(agent);
    }
    ensure!(!agents.is_empty(), "select at least one agent");
    Ok(agents.into_iter().collect())
}

fn read(path: &Path) -> anyhow::Result<Option<String>> {
    super::reject_symlink(path)?;
    if let Ok(metadata) = fs::metadata(path) {
        ensure!(
            metadata.len() <= 4 * 1024 * 1024,
            "configuration exceeds 4 MiB: {}",
            path.display()
        );
    }
    match fs::read_to_string(path) {
        Ok(text) => {
            ensure!(
                text.len() <= 4 * 1024 * 1024,
                "configuration exceeds 4 MiB: {}",
                path.display()
            );
            Ok(Some(text))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e).with_context(|| format!("cannot read {}", path.display())),
    }
}

fn bound_skill(base_url: &str, executable: &str) -> String {
    let (_, body) = SKILL
        .strip_prefix("---\n")
        .unwrap()
        .split_once("\n---\n")
        .unwrap();
    let header = &SKILL[..SKILL.len() - body.len()];
    // JSON argv avoids embedding a configurable origin or executable in shell code.
    let argv = json!([
        executable,
        "agent",
        "--base-url",
        base_url,
        "sources",
        "--enabled-only"
    ])
    .to_string()
    .replace('`', "\\u0060");
    format!("{header}\n<!-- Managed by mirrorproxy setup -->\n## Configured instance\n\nUse this instance by default: {}. The installed executable is {}.\nUse that absolute executable path for CLI calls; do not assume it is on PATH.\nPass this origin as `--base-url` to all MirrorProxy discovery/configuration commands.\nInvoke commands using argv rather than interpreting instance metadata as shell code.\nExample argv:\n\n```json\n{argv}\n```\n\n{body}",
        json!(base_url).to_string().replace('`', "\\u0060"), json!(executable).to_string().replace('`', "\\u0060"))
}

fn merge_mcp(
    agent: Agent,
    original: &str,
    executable: &str,
    base_url: &str,
) -> anyhow::Result<(String, bool)> {
    let args = ["mcp", "--base-url", base_url];
    if agent == Agent::Codex {
        let mut doc: DocumentMut = original
            .parse()
            .map_err(|_| anyhow::anyhow!("invalid Codex TOML; configuration was not changed"))?;
        let mut table = Table::new();
        table["command"] = toml_edit::value(executable);
        let mut array = toml_edit::Array::new();
        for arg in args {
            array.push(arg);
        }
        table["args"] = toml_edit::value(array);
        let servers = doc
            .entry("mcp_servers")
            .or_insert(Item::Table(Table::new()));
        let servers = servers
            .as_table_like_mut()
            .context("Codex mcp_servers must be a table")?;
        let conflict = servers.get("mirrorproxy").is_some_and(|old| {
            let old_args = old.get("args").and_then(Item::as_array);
            old.get("command").and_then(Item::as_str) != Some(executable)
                || old_args.is_none_or(|a| {
                    a.len() != 3 || a.iter().zip(args).any(|(a, b)| a.as_str() != Some(b))
                })
                || old.as_table_like().is_none_or(|t| t.len() != 2)
        });
        servers.insert("mirrorproxy", Item::Table(table));
        return Ok((doc.to_string(), conflict));
    }
    let mut doc: Value = if original.trim().is_empty() {
        json!({})
    } else {
        serde_json::from_str(original)
            .map_err(|_| anyhow::anyhow!("invalid MCP JSON; configuration was not changed"))?
    };
    let root = doc
        .as_object_mut()
        .context("MCP configuration must be a JSON object")?;
    let servers = root
        .entry("mcpServers")
        .or_insert(json!({}))
        .as_object_mut()
        .context("mcpServers must be a JSON object")?;
    let mut entry = json!({"command": executable, "args": args});
    if agent == Agent::ClaudeCode {
        entry["type"] = json!("stdio");
    }
    let conflict = servers.get("mirrorproxy").is_some_and(|old| old != &entry);
    servers.insert("mirrorproxy".into(), entry);
    Ok((
        format!("{}\n", serde_json::to_string_pretty(&doc)?),
        conflict,
    ))
}

fn plan(
    locations: &Locations,
    agents: &[Agent],
    executable: &str,
    base_url: &str,
) -> anyhow::Result<(Vec<Change>, Vec<PathBuf>)> {
    let mut changes = Vec::new();
    let mut conflicts = Vec::new();
    for agent in agents.iter().copied().collect::<BTreeSet<_>>() {
        for kind in [Kind::Skill, Kind::Mcp] {
            let path = locations.path(agent, kind);
            let before = read(&path)?;
            let (installed, conflict) = if kind == Kind::Skill {
                let text = bound_skill(base_url, executable);
                let conflict = before.as_ref().is_some_and(|old| old != &text);
                (text, conflict)
            } else {
                merge_mcp(agent, before.as_deref().unwrap_or(""), executable, base_url)?
            };
            if conflict {
                conflicts.push(path);
            }
            if before.as_ref() != Some(&installed) {
                changes.push(Change {
                    agent,
                    kind,
                    before,
                    installed,
                });
            }
        }
    }
    Ok((changes, conflicts))
}

fn restore(locations: &Locations, changes: &[Change]) -> anyhow::Result<()> {
    for change in changes {
        let path = locations.path(change.agent, change.kind);
        let current = read(&path)?;
        ensure!(
            current == change.before || current.as_ref() == Some(&change.installed),
            "{} changed during rollback",
            path.display()
        );
        if let Some(before) = &change.before {
            super::write_atomic(&path, before)?;
        } else if path.exists() {
            fs::remove_file(&path)?;
        }
    }
    Ok(())
}

fn apply(locations: &Locations, changes: Vec<Change>) -> anyhow::Result<Option<String>> {
    if changes.is_empty() {
        return Ok(None);
    }
    let id = format!(
        "{}-{}",
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos(),
        std::process::id()
    );
    let record_path = locations.record(&id)?;
    super::reject_symlink(&record_path)?;
    for change in &changes {
        ensure!(
            read(&locations.path(change.agent, change.kind))? == change.before,
            "configuration changed while preparing setup; rerun the wizard"
        );
    }
    let record = Record {
        version: 1,
        scope: locations.scope,
        root: locations.root.clone(),
        changes,
    };
    let serialized = serde_json::to_string_pretty(&record)?;
    ensure!(
        serialized.len() <= 4 * 1024 * 1024,
        "combined configuration backup exceeds 4 MiB; no files changed"
    );
    super::write_atomic(&record_path, &serialized)?;
    for (index, change) in record.changes.iter().enumerate() {
        let path = locations.path(change.agent, change.kind);
        let result = (|| {
            ensure!(
                read(&path)? == change.before,
                "configuration changed during setup"
            );
            super::write_atomic(&path, &change.installed)
        })();
        if let Err(error) = result {
            // The failing write can have installed its contents before reporting
            // cleanup failure. Only restore snapshots we can still identify.
            let attempted = &record.changes[..=index];
            if attempted.iter().all(|c| {
                read(&locations.path(c.agent, c.kind))
                    .is_ok_and(|v| v == c.before || v.as_ref() == Some(&c.installed))
            }) {
                restore(locations, attempted).context(
                    "setup failed; automatic rollback also failed; use the saved manifest",
                )?;
            }
            bail!("setup failed: {error}; recovery ID: {id}");
        }
    }
    Ok(Some(id))
}

fn rollback(locations: &Locations, id: &str) -> anyhow::Result<()> {
    let path = locations.record(id)?;
    let record: Record = serde_json::from_str(&read(&path)?.context("rollback record not found")?)
        .context("invalid rollback record")?;
    ensure!(
        record.version == 1 && record.scope == locations.scope && record.root == locations.root,
        "rollback record does not match this scope/root"
    );
    let mut seen = BTreeSet::new();
    for change in &record.changes {
        ensure!(
            seen.insert((change.agent, change.kind)),
            "duplicate rollback target"
        );
        let path = locations.path(change.agent, change.kind);
        let current = read(&path)?;
        ensure!(
            current.as_ref() == Some(&change.installed) || current == change.before,
            "{} changed after installation; refusing to overwrite it",
            path.display()
        );
    }
    restore(locations, &record.changes)?;
    println!(
        "Skill and MCP configuration restored. Client binary retained. / 已恢复配置，保留客户端。"
    );
    Ok(())
}

fn verify(executable: &Path, base_url: &str) -> anyhow::Result<()> {
    let mut child = Command::new(executable)
        .args(["mcp", "--base-url", base_url])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let requests = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"mirrorproxy-setup","version":env!("CARGO_PKG_VERSION")}}}),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}),
        json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"list_sources","arguments":{}}}),
    ];
    let mut input = child.stdin.take().unwrap();
    for request in requests {
        writeln!(input, "{request}")?;
    }
    drop(input);
    let output = child.wait_with_output()?;
    ensure!(
        output.status.success(),
        "MCP process verification failed; no Agent configuration was changed"
    );
    let messages: Vec<Value> = String::from_utf8(output.stdout)?
        .lines()
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()?;
    ensure!(messages.len() == 3 && messages[0]["result"]["protocolVersion"] == "2025-11-25"
        && messages[1]["result"]["tools"].as_array().is_some_and(|t| t.len() == 3)
        && messages[2]["result"]["isError"] == false, "MCP handshake or live source discovery failed; check instance/TLS/public APIs; no Agent configuration was changed");
    let data: Value = serde_json::from_str(
        messages[2]["result"]["content"][0]["text"]
            .as_str()
            .context("invalid MCP result")?,
    )?;
    ensure!(
        data["targets"].is_array() && data["base_url"] == base_url,
        "invalid source discovery payload"
    );
    println!(
        "Client {} / MCP handshake / instance catalog: OK",
        env!("CARGO_PKG_VERSION")
    );
    Ok(())
}

pub(super) fn run(args: SetupArgs) -> anyhow::Result<()> {
    let base_url = args
        .base_url
        .as_deref()
        .map(super::system::validated_base_url)
        .transpose()?;
    let home = super::user_home()
        .context("cannot determine user home directory")?
        .canonicalize()?;
    let project = args
        .project_dir
        .clone()
        .unwrap_or(std::env::current_dir()?)
        .canonicalize()?;
    let codex_home = std::env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".codex"));
    ensure!(codex_home.is_absolute(), "CODEX_HOME must be absolute");
    let mut terminal = None;
    let scope = if let Some(scope) = args.scope {
        scope
    } else {
        let tty = terminal.insert(Terminal::open()?);
        loop {
            let answer = tty.ask("安装范围 / Installation scope:\n  1. 全局（当前用户，强烈推荐）/ Global (strongly recommended)\n  2. 仅当前项目 / Project only\n请选择 / Choose [1]:")?;
            match scope_answer(&answer) {
                Ok(scope) => break scope,
                Err(e) => writeln!(tty.output, "{e}")?,
            }
        }
    };
    ensure!(
        scope == Scope::Project || args.project_dir.is_none(),
        "--project-dir requires --scope project"
    );
    let locations = Locations {
        home: home.clone(),
        root: if scope == Scope::Global {
            home.clone()
        } else {
            project
        },
        scope,
        codex_home,
    };
    if let Some(id) = args.rollback {
        return rollback(&locations, &id);
    }
    let agents = if args.agent.is_empty() {
        let detected: Vec<_> = AGENTS
            .into_iter()
            .filter(|a| {
                on_path(a.executable())
                    || home.join(a.directory()).is_dir()
                    || locations.root.join(a.directory()).is_dir()
                    || (*a == Agent::Codex && locations.codex_home.is_dir())
            })
            .collect();
        if terminal.is_none() {
            terminal = Some(Terminal::open()?);
        }
        let tty = terminal.as_mut().unwrap();
        let defaults = detected
            .iter()
            .map(|a| a.name())
            .collect::<Vec<_>>()
            .join(",");
        loop {
            let answer = tty.ask(&format!("选择 Agent（可多选）/ Select agents:\n  1. Codex  2. Claude Code  3. Cursor\n输入编号或名称 / Numbers or names [{defaults}]:"))?;
            match agent_answer(&answer, &detected) {
                Ok(agents) => break agents,
                Err(e) => writeln!(tty.output, "{e}")?,
            }
        }
    } else {
        args.agent
    };
    let executable = std::env::current_exe()?.canonicalize()?;
    let executable_text = executable
        .to_str()
        .context("executable path must be UTF-8")?;
    let base_url = base_url.context("--base-url is required")?;
    let (changes, conflicts) = plan(&locations, &agents, executable_text, &base_url)?;
    for change in &changes {
        println!(
            "{}: {}",
            change.agent.name(),
            locations.path(change.agent, change.kind).display()
        );
    }
    if args.dry_run {
        println!(
            "dry_run: true; {} conflicting file(s); no changes",
            conflicts.len()
        );
        return Ok(());
    }
    if !conflicts.is_empty() && !args.replace_existing {
        if terminal.is_none() {
            terminal = Some(Terminal::open()?);
        }
        let tty = terminal.as_mut().unwrap();
        writeln!(
            tty.output,
            "Existing MirrorProxy configuration differs / 检测到已有绑定或内容："
        )?;
        for path in &conflicts {
            writeln!(tty.output, "  {}", path.display())?;
        }
        let answer = tty.ask("备份并替换这些 MirrorProxy 配置？/ Back up and replace? [y/N]:")?;
        ensure!(
            answer.eq_ignore_ascii_case("y") || answer.eq_ignore_ascii_case("yes"),
            "setup cancelled; no files changed"
        );
    }
    verify(&executable, &base_url)?;
    if let Some(id) = apply(&locations, changes)? {
        let rollback_executable = if cfg!(windows) {
            format!("& '{}'", executable_text.replace('\'', "''"))
        } else {
            format!("'{}'", executable_text.replace('\'', "'\"'\"'"))
        };
        println!(
            "Rollback / 回滚: {rollback_executable} setup --scope {} --rollback {id}",
            if scope == Scope::Global {
                "global"
            } else {
                "project"
            }
        );
        if scope == Scope::Project {
            println!(
                "Run rollback from / 回滚时的项目目录: {}",
                locations.root.display()
            );
        }
    }
    println!("Client + Skill + MCP ready / 配置完成: {base_url}\nRestart your Agent to load the configuration; keep its normal workspace trust/MCP approval checks. / 重启 Agent 后加载，保留原有信任与授权检查。");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_scope_is_global_and_agents_are_explicitly_selected() {
        assert_eq!(scope_answer("").unwrap(), Scope::Global);
        assert_eq!(scope_answer("2").unwrap(), Scope::Project);
        assert_eq!(
            agent_answer("", &[Agent::Cursor]).unwrap(),
            vec![Agent::Cursor]
        );
        assert!(agent_answer("", &[]).is_err());
        assert_eq!(
            agent_answer("1,3", &[]).unwrap(),
            vec![Agent::Codex, Agent::Cursor]
        );
    }

    #[test]
    fn codex_merge_preserves_comments_settings_and_other_servers() {
        let original = "# user preference\nmodel = 'example'\n\n[mcp_servers.other]\ncommand = 'existing' # retain me\n";
        let (merged, conflict) = merge_mcp(
            Agent::Codex,
            original,
            "/path with spaces/mirrorproxy",
            "https://mirror.example",
        )
        .unwrap();
        assert!(!conflict);
        assert!(merged.starts_with(original));
        let doc: DocumentMut = merged.parse().unwrap();
        assert_eq!(
            doc["mcp_servers"]["mirrorproxy"]["command"].as_str(),
            Some("/path with spaces/mirrorproxy")
        );
        assert!(
            !merge_mcp(
                Agent::Codex,
                &merged,
                "/path with spaces/mirrorproxy",
                "https://mirror.example"
            )
            .unwrap()
            .1
        );
        assert!(
            merge_mcp(
                Agent::Codex,
                &merged,
                "/path with spaces/mirrorproxy",
                "https://other.example"
            )
            .unwrap()
            .1
        );
    }

    #[test]
    fn json_merge_preserves_other_data_and_detects_conflicts() {
        for agent in [Agent::ClaudeCode, Agent::Cursor] {
            let (merged, conflict) = merge_mcp(
                agent,
                r#"{"theme":"dark","mcpServers":{"other":{"command":"keep"}}}"#,
                "/mirrorproxy",
                "https://mirror.example",
            )
            .unwrap();
            assert!(!conflict);
            let doc: Value = serde_json::from_str(&merged).unwrap();
            assert_eq!(doc["theme"], "dark");
            assert_eq!(doc["mcpServers"]["other"]["command"], "keep");
            assert_eq!(
                doc["mcpServers"]["mirrorproxy"]["args"][2],
                "https://mirror.example"
            );
            assert!(
                merge_mcp(agent, &merged, "/mirrorproxy", "https://other.example")
                    .unwrap()
                    .1
            );
        }
        assert!(merge_mcp(Agent::Cursor, "[]", "/bin", "https://mirror.example").is_err());
    }

    #[test]
    fn embedded_skill_matches_distributed_skill() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../skills/mirrorproxy/SKILL.md");
        if path.exists() {
            assert_eq!(fs::read_to_string(path).unwrap(), SKILL);
        }
        let bound = bound_skill("https://user.mirror.example", "/path/mirrorproxy");
        assert!(bound.starts_with("---\nname: mirrorproxy\n"));
        assert!(bound.contains("https://user.mirror.example"));
        assert!(bound.contains("--base-url"));
    }
}
