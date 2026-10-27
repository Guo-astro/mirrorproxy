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
const AGENTS: [Agent; 11] = [
    Agent::Codex,
    Agent::ClaudeCode,
    Agent::Cursor,
    Agent::Gemini,
    Agent::OpenCode,
    Agent::VsCode,
    Agent::Trae,
    Agent::TraeCn,
    Agent::WorkBuddy,
    Agent::Dsh,
    Agent::Generic,
];

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
    #[value(alias = "gemini-cli")]
    Gemini,
    #[value(name = "opencode")]
    #[serde(rename = "opencode")]
    OpenCode,
    #[value(name = "vscode", alias = "copilot")]
    #[serde(rename = "vscode")]
    VsCode,
    Trae,
    TraeCn,
    #[value(name = "workbuddy")]
    #[serde(rename = "workbuddy")]
    WorkBuddy,
    Dsh,
    Generic,
}

impl Agent {
    fn name(self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::ClaudeCode => "claude-code",
            Self::Cursor => "cursor",
            Self::Gemini => "gemini",
            Self::OpenCode => "opencode",
            Self::VsCode => "vscode",
            Self::Trae => "trae",
            Self::TraeCn => "trae-cn",
            Self::WorkBuddy => "workbuddy",
            Self::Dsh => "dsh",
            Self::Generic => "generic",
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::Codex => "Codex",
            Self::ClaudeCode => "Claude Code",
            Self::Cursor => "Cursor",
            Self::Gemini => "Gemini CLI",
            Self::OpenCode => "OpenCode",
            Self::VsCode => "VS Code / GitHub Copilot",
            Self::Trae => "TRAE international / 国际版",
            Self::TraeCn => "TRAE CN / 国内版",
            Self::WorkBuddy => "WorkBuddy",
            Self::Dsh => "DeepSeek Harness (dsh)",
            Self::Generic => "Other / 通用 Skill + MCP",
        }
    }
    fn directory(self) -> &'static str {
        match self {
            Self::Codex => ".codex",
            Self::ClaudeCode => ".claude",
            Self::Cursor => ".cursor",
            Self::Gemini => ".gemini",
            Self::OpenCode => ".config/opencode",
            Self::VsCode => ".vscode",
            Self::Trae => ".trae",
            Self::TraeCn => ".trae-cn",
            Self::WorkBuddy => ".workbuddy",
            Self::Dsh => ".dsh",
            Self::Generic => ".agents",
        }
    }
    fn executable(self) -> &'static str {
        match self {
            Self::ClaudeCode => "claude",
            Self::VsCode => "code",
            Self::Generic => "",
            _ => self.name(),
        }
    }
    fn needs_import(self) -> bool {
        matches!(
            self,
            Self::VsCode | Self::Trae | Self::TraeCn | Self::WorkBuddy | Self::Dsh | Self::Generic
        )
    }
    fn capability(self) -> &'static str {
        if self.needs_import() {
            "Skill + MCP export (manual activation) / 需导入或启动参数"
        } else {
            "Skill + MCP config / 自动写入配置"
        }
    }
    fn kinds(self) -> &'static [Kind] {
        if self == Self::WorkBuddy {
            &[
                Kind::Skill,
                Kind::Mcp,
                Kind::Instructions,
                Kind::Metadata,
                Kind::Icon,
            ]
        } else if self.needs_import() {
            &[Kind::Skill, Kind::Mcp, Kind::Instructions]
        } else {
            &[Kind::Skill, Kind::Mcp]
        }
    }
}

fn on_path(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
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
    Instructions,
    Metadata,
    Icon,
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
        let export = self.root.join(".mirrorproxy/agents").join(agent.name());
        match kind {
            Kind::Instructions => return export.join("README.md"),
            Kind::Metadata => return export.join("connector-meta.json"),
            Kind::Icon => return export.join("icon.svg"),
            Kind::Skill => {
                let directory = match agent {
                    Agent::Codex | Agent::Generic | Agent::VsCode => ".agents",
                    Agent::TraeCn if self.scope == Scope::Project => ".trae",
                    Agent::WorkBuddy => return export.join("skills/mirrorproxy/SKILL.md"),
                    Agent::OpenCode if self.scope == Scope::Project => ".opencode",
                    _ => agent.directory(),
                };
                return self
                    .root
                    .join(directory)
                    .join("skills/mirrorproxy/SKILL.md");
            }
            Kind::Mcp => {}
        }
        match agent {
            Agent::Codex if self.scope == Scope::Global => self.codex_home.join("config.toml"),
            Agent::Codex => self.root.join(".codex/config.toml"),
            Agent::ClaudeCode if self.scope == Scope::Global => self.home.join(".claude.json"),
            Agent::ClaudeCode => self.root.join(".mcp.json"),
            Agent::Cursor => self.root.join(".cursor/mcp.json"),
            Agent::Gemini => self.root.join(".gemini/settings.json"),
            Agent::OpenCode if self.scope == Scope::Global => {
                self.home.join(".config/opencode/opencode.json")
            }
            Agent::OpenCode => self.root.join("opencode.json"),
            Agent::Dsh => export.join("mirrorproxy.cordis.yml"),
            _ => export.join("mcp.json"),
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
    if answer.trim().eq_ignore_ascii_case("all") || answer.trim() == "全部" {
        return Ok(AGENTS.to_vec());
    }
    let mut agents = BTreeSet::new();
    for item in answer
        .split(|c: char| c == ',' || c == '，' || c.is_whitespace())
        .filter(|s| !s.is_empty())
    {
        let normalized = item.to_ascii_lowercase();
        let agent = normalized.parse::<usize>().ok()
            .and_then(|index| index.checked_sub(1).and_then(|i| AGENTS.get(i).copied()))
            .or_else(|| Agent::from_str(&normalized, true).ok())
            .with_context(|| format!("unknown Agent: {item}; use listed numbers/names, separated by commas or spaces / 请使用编号或名称，多选以逗号或空格分隔"))?;
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

fn bound_skill(base_url: &str, executable: &str) -> anyhow::Result<String> {
    render_skill(SKILL, base_url, executable)
}

fn render_skill(template: &str, base_url: &str, executable: &str) -> anyhow::Result<String> {
    // Git may check Markdown out with CRLF on Windows. Normalize before parsing
    // the embedded frontmatter and generating a portable installed Skill.
    let template = template.replace("\r\n", "\n");
    let (_, body) = template
        .strip_prefix("---\n")
        .and_then(|text| text.split_once("\n---\n"))
        .context("embedded Skill is missing YAML frontmatter; no files changed")?;
    let header = &template[..template.len() - body.len()];
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
    Ok(format!("{header}\n<!-- Managed by mirrorproxy setup -->\n## Configured instance\n\nUse this instance by default: {}. The installed executable is {}.\nUse that absolute executable path for CLI calls; do not assume it is on PATH.\nPass this origin as `--base-url` to all MirrorProxy discovery/configuration commands.\nInvoke commands using argv rather than interpreting instance metadata as shell code.\nExample argv:\n\n```json\n{argv}\n```\n\n{body}",
        json!(base_url).to_string().replace('`', "\\u0060"), json!(executable).to_string().replace('`', "\\u0060")))
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
    let server_key = match agent {
        Agent::OpenCode => "mcp",
        Agent::VsCode => "servers",
        _ => "mcpServers",
    };
    let servers = root
        .entry(server_key)
        .or_insert(json!({}))
        .as_object_mut()
        .with_context(|| format!("{server_key} must be a JSON object"))?;
    let mut entry = json!({"command": executable, "args": args});
    if matches!(agent, Agent::ClaudeCode | Agent::VsCode | Agent::WorkBuddy) {
        entry["type"] = json!("stdio");
    }
    if agent == Agent::OpenCode {
        entry = json!({"type": "local", "command": [executable, "mcp", "--base-url", base_url], "enabled": true});
    }
    let conflict = servers.get("mirrorproxy").is_some_and(|old| old != &entry);
    servers.insert("mirrorproxy".into(), entry);
    Ok((
        format!("{}\n", serde_json::to_string_pretty(&doc)?),
        conflict,
    ))
}

fn export_content(
    locations: &Locations,
    agent: Agent,
    kind: Kind,
    executable: &str,
    base_url: &str,
) -> anyhow::Result<String> {
    match kind {
        // JSON is a YAML subset accepted by Cordis, and safely escapes paths on Windows.
        Kind::Mcp if agent == Agent::Dsh => Ok(format!("{}\n", serde_json::to_string_pretty(&json!([
            {"insert": [{"id": "mirrorproxy-mcp", "name": "@deepseek-ai/dsh-mcp-client", "config": {
                "serverName": "mirrorproxy", "transport": "stdio", "command": executable,
                "args": ["mcp", "--base-url", base_url]
            }}]}
        ]))?)),
        Kind::Metadata => Ok(format!("{}\n", serde_json::to_string_pretty(&json!({
            "name": "MirrorProxy", "name_en": "MirrorProxy", "name_zh": "MirrorProxy",
            "source": "mirrorproxy", "type": "mcp", "version": env!("CARGO_PKG_VERSION"),
            "description": "Discover package mirrors and generate native client configuration.",
            "description_zh": "查询镜像能力并生成原生客户端配置。",
            "examples_zh": ["查询可用镜像", "生成 npm 镜像配置"],
            "examples_en": ["List available mirrors", "Generate npm mirror configuration"]
        }))?)),
        Kind::Icon => Ok("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 64 64\"><rect width=\"64\" height=\"64\" rx=\"14\" fill=\"#0066cc\"/><path d=\"M16 20h22L28 10m10 10L28 30M48 44H26l10-10M26 44l10 10\" fill=\"none\" stroke=\"white\" stroke-width=\"5\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/></svg>\n".into()),
        Kind::Instructions => {
            let mcp = locations.path(agent, Kind::Mcp);
            let skill = locations.path(agent, Kind::Skill);
            let steps = match agent {
                Agent::Dsh => format!("Skill installed to the dsh skill root. Start the desired profile with this explicit patch (JSON argv):\n\n```json\n{}\n```\nUse a dsh version that ships @deepseek-ai/dsh-mcp-client; existing custom profiles must provide the Skill filesystem plugin. Do not enable this patch twice in the same profile. / 使用包含官方 MCP 插件的 dsh；自定义 profile 需启用本地 Skill 插件，同一 profile 不要重复加载此 patch。", json!(["dsh", "web", "--patch", mcp.to_string_lossy()])),
                Agent::VsCode => "VS Code / GitHub Copilot: open the command palette and run MCP: Open User Configuration (global) or MCP: Open Workspace Folder MCP Configuration (project). Merge the exported servers.mirrorproxy entry into that file. The Skill uses .agents/skills; enable Agent Skills in a supporting VS Code version. / 打开命令面板中的用户或工作区 MCP 配置，合并导出的 servers.mirrorproxy；Skill 需要支持 .agents/skills 的版本。".into(),
                Agent::Trae | Agent::TraeCn => "TRAE IDE: Skill installed into the selected edition's skill directory. Open Settings > MCP > raw JSON configuration; merge only mcpServers.mirrorproxy from the export, preserving other servers. Project Skills use .trae/skills in both editions. / Skill 已写入对应版本目录；在设置的 MCP 原始 JSON 中合并 mcpServers.mirrorproxy，保留其他服务。".into(),
                Agent::WorkBuddy => "WorkBuddy: this directory is an MCP + Skill connector bundle, not an automatically activated user configuration. Import skills/mirrorproxy using the client's Skill import UI; add the stdio server in MCP settings using mcp.json (command and args). A connector-market submission is a separate manual action. / 此目录是连接器包，尚未自动激活；通过 Skill 界面导入技能，并在 MCP 设置添加 mcp.json 中的 command 和 args。发布连接器市场需另行提交。".into(),
                _ => "Other clients: import the SKILL.md if Agent Skills is supported; merge mcpServers.mirrorproxy into the client's MCP configuration, translating field names if necessary. / 其他客户端：导入标准 Skill，按客户端要求合并 MCP 配置，必要时转换字段名。".into(),
            };
            Ok(format!("# MirrorProxy — {}\n\nActivation required / 需要手动激活\n\n{steps}\n\nSkill: {}\n\nMCP export: {}\n\nOnly local files were prepared. The self-check validates MirrorProxy's MCP process and instance catalog, not whether this Agent loaded it. Restart/reload the Agent and confirm mirrorproxy tools appear. Keep normal workspace trust and MCP approval checks. / 自检仅验证 MirrorProxy MCP 和实例，不能证明 Agent 已加载；请重启并确认工具可用，保留信任与授权检查。\n", agent.name(), skill.display(), mcp.display()))
        }
        _ => bail!("unsupported Agent export"),
    }
}

fn plan(
    locations: &Locations,
    agents: &[Agent],
    executable: &str,
    base_url: &str,
) -> anyhow::Result<(Vec<Change>, Vec<PathBuf>)> {
    let mut changes = Vec::new();
    let mut conflicts = Vec::new();
    let mut paths = BTreeSet::new();
    for agent in agents.iter().copied().collect::<BTreeSet<_>>() {
        for &kind in agent.kinds() {
            let path = locations.path(agent, kind);
            if !paths.insert(path.clone()) {
                continue;
            }
            if agent == Agent::OpenCode && kind == Kind::Mcp {
                let jsonc = path.with_extension("jsonc");
                ensure!(!jsonc.exists(), "OpenCode JSONC config exists at {}; merge MCP manually instead of shadowing it", jsonc.display());
            }
            let before = read(&path)?;
            let (installed, conflict) = if kind == Kind::Skill {
                let text = bound_skill(base_url, executable)?;
                let conflict = before.as_ref().is_some_and(|old| old != &text);
                (text, conflict)
            } else if kind == Kind::Mcp && agent != Agent::Dsh {
                merge_mcp(agent, before.as_deref().unwrap_or(""), executable, base_url)?
            } else {
                let text = export_content(locations, agent, kind, executable, base_url)?;
                let conflict = before.as_ref().is_some_and(|old| old != &text);
                (text, conflict)
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
            seen.insert(locations.path(change.agent, change.kind)),
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
        && messages[1]["result"]["tools"].as_array().is_some_and(|t| t.len() == 5)
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
                *a != Agent::Generic
                    && (on_path(a.executable())
                        || home.join(a.directory()).is_dir()
                        || locations.root.join(a.directory()).is_dir()
                        || (*a == Agent::Codex && locations.codex_home.is_dir())
                        || locations.path(*a, Kind::Mcp).is_file())
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
            let options = AGENTS
                .iter()
                .enumerate()
                .map(|(index, agent)| {
                    format!(
                        "  {}. {} / {} [{}]{}",
                        index + 1,
                        agent.label(),
                        agent.name(),
                        agent.capability(),
                        if detected.contains(agent) {
                            " * detected / 已检测"
                        } else {
                            ""
                        }
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            let answer = tty.ask(&format!("选择 Agent（可多选）/ Select agents:\n{options}\n多选示例 / Examples: 1,2,3 or 1 2 3; all = 全部\n回车选择已检测项 / Enter selects detected agents [{defaults}]:"))?;
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
    for agent in &agents {
        if agent.needs_import() {
            println!(
                "{}: activation required / 尚需激活: {}",
                agent.name(),
                locations.path(*agent, Kind::Instructions).display()
            );
        }
    }
    println!("Client + Skill/MCP files prepared / 配置文件已准备: {base_url}\nRestart your Agent to load the configuration; keep its normal workspace trust/MCP approval checks. / 重启 Agent 后加载，保留原有信任与授权检查。");
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
    fn multi_selection_accepts_examples_names_aliases_and_all() {
        for input in [
            "1,2,3",
            "1 2 3",
            "1，2，3",
            "CODEX claude-code cursor",
            "1,1,2,3",
        ] {
            assert_eq!(
                agent_answer(input, &[]).unwrap(),
                vec![Agent::Codex, Agent::ClaudeCode, Agent::Cursor]
            );
        }
        assert_eq!(agent_answer("all", &[]).unwrap(), AGENTS.to_vec());
        assert_eq!(agent_answer("全部", &[]).unwrap(), AGENTS.to_vec());
        assert_eq!(
            agent_answer("dsh workbuddy trae-cn gemini-cli copilot", &[]).unwrap(),
            vec![
                Agent::Gemini,
                Agent::VsCode,
                Agent::TraeCn,
                Agent::WorkBuddy,
                Agent::Dsh
            ]
        );
        for input in ["0", "12", "99", "unknown", ",,,", "all codex"] {
            assert!(agent_answer(input, &[]).is_err());
        }
    }

    #[test]
    fn new_mcp_formats_preserve_existing_servers_and_detect_rebinding() {
        for (agent, key) in [
            (Agent::Gemini, "mcpServers"),
            (Agent::OpenCode, "mcp"),
            (Agent::VsCode, "servers"),
            (Agent::WorkBuddy, "mcpServers"),
        ] {
            let original =
                json!({"user_setting": true, key: {"other": {"command": "keep"}}}).to_string();
            let (merged, conflict) = merge_mcp(
                agent,
                &original,
                r"C:\Program Files\mirrorproxy.exe",
                "https://mirror.example",
            )
            .unwrap();
            assert!(!conflict);
            let doc: Value = serde_json::from_str(&merged).unwrap();
            assert_eq!(doc["user_setting"], true);
            assert_eq!(doc[key]["other"]["command"], "keep");
            assert!(
                !merge_mcp(
                    agent,
                    &merged,
                    r"C:\Program Files\mirrorproxy.exe",
                    "https://mirror.example"
                )
                .unwrap()
                .1
            );
            assert!(
                merge_mcp(
                    agent,
                    &merged,
                    r"C:\Program Files\mirrorproxy.exe",
                    "https://other.example"
                )
                .unwrap()
                .1
            );
            assert!(merge_mcp(agent, "{ broken", "/bin", "https://mirror.example").is_err());
        }
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
        let bound = bound_skill("https://user.mirror.example", "/path/mirrorproxy").unwrap();
        assert!(bound.starts_with("---\nname: mirrorproxy\n"));
        assert!(bound.contains("https://user.mirror.example"));
        assert!(bound.contains("--base-url"));
    }

    #[test]
    fn bound_skill_handles_windows_crlf_and_preserves_frontmatter_and_body() {
        let lf = SKILL.replace("\r\n", "\n");
        let crlf = lf.replace('\n', "\r\n");
        let origin = "https://personal.mirror.example";
        let executable = r"C:\Users\Example User\mirrorproxy.exe";
        let rendered = render_skill(&crlf, origin, executable).unwrap();
        assert_eq!(rendered, render_skill(&lf, origin, executable).unwrap());
        let (header, body) = lf.split_once("\n---\n").unwrap();
        assert!(rendered.starts_with(&format!("{header}\n---\n")));
        assert!(rendered.ends_with(body));
        assert!(rendered.contains(origin));
        assert!(!rendered.contains('\r'));
        assert!(render_skill("# Missing frontmatter", origin, executable).is_err());
        assert!(render_skill("---\nname: incomplete\n", origin, executable).is_err());
    }
}
