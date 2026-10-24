//! Exercise the shipped binary, MCP framing and real HTTP discovery together.
use std::{
    fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};

use serde_json::{json, Value};

fn catalog() -> Value {
    json!({
        "targets": [
            {"code": "pip", "name": "Python", "category": "lang", "aliases": ["pypi"], "capabilities": {"authentication": "operator_controlled"}},
            {"code": "bun", "category": "lang", "aliases": []},
            {"code": "custom-os", "category": "repo", "aliases": ["additional_os"]},
            {"code": "docker", "category": "repo", "aliases": []}
        ],
        "sources": [
            {"target_code": "pip", "provider_code": "mirrorproxy", "repo_url": "/pypi/simple/", "capability": "proxy"},
            {"target_code": "pip", "provider_code": "third-party", "repo_url": "https://mirror.example/simple/", "capability": "template"},
            {"target_code": "bun", "provider_code": "mirrorproxy", "repo_url": "/npm/", "capability": "proxy"},
            {"target_code": "custom-os", "provider_code": "mirrorproxy", "repo_url": "/os/custom-os/", "capability": "proxy"},
            {"target_code": "docker", "provider_code": "mirrorproxy", "repo_url": "/v2/", "capability": "proxy"}
        ],
        "templates": [{"target_code": "pip", "template": "advisory only"}],
        "container_registries": [{"code": "ghcr", "host": "ghcr.io"}]
    })
}

struct Server {
    base: String,
    requests: Arc<Mutex<Vec<String>>>,
    stopped: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
}

impl Server {
    fn new(mode: &'static str, catalog: Value) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let stopped = Arc::new(AtomicBool::new(false));
        let worker = {
            let requests = requests.clone();
            let stopped = stopped.clone();
            thread::spawn(move || {
                for stream in listener.incoming() {
                    if stopped.load(Ordering::SeqCst) {
                        break;
                    }
                    let mut stream = stream.unwrap();
                    stream
                        .set_read_timeout(Some(Duration::from_secs(5)))
                        .unwrap();
                    let mut request = Vec::new();
                    let mut byte = [0];
                    while request.len() < 32768 && !request.ends_with(b"\r\n\r\n") {
                        if stream.read(&mut byte).unwrap_or(0) == 0 {
                            break;
                        }
                        request.push(byte[0]);
                    }
                    let request = String::from_utf8(request).unwrap();
                    let path = request.split_whitespace().nth(1).unwrap_or("");
                    let body = match path {
                        "/api/public-config" => {
                            json!({"enabled_proxies": ["pypi", "os", "oci"], "user_access": {"enabled": true, "mode": "subdomain_required"}, "quota": {"enabled": true, "monthly_gb": 10}})
                        }
                        "/api/sources" => catalog.clone(),
                        _ => panic!("unexpected network target: {path}"),
                    };
                    requests.lock().unwrap().push(request);
                    let response = match mode {
                        "redirect" => "HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:1/secret\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".into(),
                        "html" => "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: 6\r\nConnection: close\r\n\r\nsecret".into(),
                        "invalid" => "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 6\r\nConnection: close\r\n\r\nsecret".into(),
                        "large" => {
                            let body = "x".repeat(4 * 1024 * 1024 + 1);
                            format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len())
                        }
                        _ => {
                            let body = body.to_string();
                            format!("HTTP/1.1 200 OK\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len())
                        }
                    };
                    let _ = stream.write_all(response.as_bytes());
                }
            })
        };
        Self {
            base,
            requests,
            stopped,
            worker: Some(worker),
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(self.base.trim_start_matches("http://"));
        self.worker.take().unwrap().join().unwrap();
    }
}

fn command() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_mirrorproxy"));
    // Discovery must work directly even in an agent with inherited proxy vars.
    for key in [
        "HTTP_PROXY",
        "HTTPS_PROXY",
        "ALL_PROXY",
        "http_proxy",
        "https_proxy",
        "all_proxy",
    ] {
        command.env(key, "http://127.0.0.1:1");
    }
    command.env("NO_PROXY", "").env("no_proxy", "");
    command
}

fn initialize() -> Value {
    json!({"jsonrpc": "2.0", "id": "init", "method": "initialize", "params": {
        "protocolVersion": "2025-11-25", "capabilities": {}, "clientInfo": {"name": "test", "version": "1"}
    }})
}

fn rpc(method: &str, params: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": 2, "method": method, "params": params})
}

fn mcp(base: &str, messages: &[Value]) -> Vec<Value> {
    let mut child = command()
        .args(["mcp", "--base-url", base])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    for message in messages {
        writeln!(stdin, "{message}").unwrap();
    }
    drop(stdin);
    let result = child.wait_with_output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(result.stderr.is_empty());
    String::from_utf8(result.stdout)
        .unwrap()
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect()
}

fn tool(base: &str, name: &str, arguments: Value) -> Value {
    mcp(
        base,
        &[
            initialize(),
            json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
            rpc("tools/call", json!({"name": name, "arguments": arguments})),
        ],
    )
    .pop()
    .unwrap()
}

#[test]
fn stdio_lifecycle_discovery_and_notifications() {
    // Initialization/listing do not contact the instance.
    let replies = mcp(
        "http://127.0.0.1:1",
        &[
            rpc("tools/list", json!({})),
            initialize(),
            rpc("tools/list", json!({})),
            json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
            json!({"jsonrpc": "2.0", "method": "notifications/cancelled", "params": {"requestId": 99}}),
            rpc("ping", json!({})),
            rpc("tools/list", json!({})),
            rpc("unknown", json!({})),
            initialize(),
        ],
    );
    assert_eq!(replies.len(), 7);
    assert_eq!(replies[0]["error"]["code"], -32600);
    assert_eq!(replies[1]["id"], "init");
    assert_eq!(replies[1]["result"]["protocolVersion"], "2025-11-25");
    assert_eq!(replies[2]["error"]["code"], -32600);
    assert_eq!(replies[3]["result"], json!({}));
    assert_eq!(replies[4]["result"]["tools"].as_array().unwrap().len(), 5);
    assert!(replies[4]["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .all(|t| t["annotations"]["readOnlyHint"] == (t["name"] != "download_file")));
    assert_eq!(replies[5]["error"]["code"], -32601);
    assert_eq!(replies[6]["error"]["code"], -32600);
}

#[test]
fn live_catalog_aliases_disabled_adapters_and_custom_os() {
    let server = Server::new("normal", catalog());
    let response = tool(&server.base, "get_source", json!({"target": "PyPI"}));
    assert_eq!(response["result"]["isError"], false);
    let data: Value =
        serde_json::from_str(response["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(data["target"]["code"], "pip");
    assert_eq!(data["sources"][0]["enabled"], true);
    assert_eq!(
        data["sources"][0]["url"],
        format!("{}/pypi/simple/", server.base)
    );
    assert_eq!(data["sources"][1]["enabled"], Value::Null);
    assert_eq!(data["templates"][0]["template"], "advisory only");
    let output = command()
        .args([
            "agent",
            "--base-url",
            &server.base,
            "sources",
            "--enabled-only",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let data: Value = serde_json::from_slice(&output.stdout).unwrap();
    let codes: Vec<_> = data["targets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["code"].as_str().unwrap())
        .collect();
    assert!(codes.contains(&"pip") && codes.contains(&"custom-os") && codes.contains(&"docker"));
    assert!(!codes.contains(&"bun"));
    let requests = server.requests.lock().unwrap();
    assert_eq!(requests.len(), 4);
    for request in requests.iter() {
        let request = request.to_ascii_lowercase();
        assert!(request.starts_with("get /api/"));
        assert!(!request.contains("authorization:") && !request.contains("cookie:"));
    }
}

#[test]
fn public_policy_does_not_claim_personal_usage_or_general_proxy() {
    let server = Server::new("normal", catalog());
    let output = command()
        .args(["agent", "--base-url", &server.base, "instance"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let data: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(data["arbitrary_url_proxy"], false);
    assert_eq!(data["per_user_usage_available"], false);
    assert_eq!(data["user_access"]["mode"], "subdomain_required");
    assert_eq!(data["quota_policy"]["monthly_gb"], 10);
    assert_eq!(server.requests.lock().unwrap().len(), 1);
}

#[test]
fn invalid_tool_arguments_never_make_network_requests() {
    for (name, args) in [
        ("fetch_url", json!({"url": "http://127.0.0.1/secret"})),
        (
            "get_source",
            json!({"target": "pip", "base_url": "http://127.0.0.1"}),
        ),
        ("get_source", json!({"target": ""})),
        ("get_source", json!({})),
        ("list_sources", json!({"category": "invalid"})),
        ("list_sources", json!({"enabled_only": "true"})),
        ("inspect_instance", json!({"token": "secret"})),
    ] {
        let response = tool("http://127.0.0.1:1", name, args);
        assert_eq!(response["error"]["code"], -32602, "{response}");
    }
}

#[test]
fn redirects_non_json_invalid_json_and_oversized_responses_are_errors() {
    for mode in ["redirect", "html", "invalid", "large"] {
        let server = Server::new(mode, catalog());
        let response = tool(&server.base, "inspect_instance", json!({}));
        assert_eq!(response["result"]["isError"], true, "{mode}: {response}");
        assert!(!response.to_string().contains("secret"));
        assert_eq!(server.requests.lock().unwrap().len(), 1);
    }
}

#[test]
fn rejects_catalog_paths_that_change_the_origin() {
    let mut data = catalog();
    data["sources"][0]["repo_url"] = json!("//127.0.0.1/secret");
    let server = Server::new("normal", data);
    assert_eq!(
        tool(&server.base, "get_source", json!({"target": "pip"}))["result"]["isError"],
        true
    );
    assert_eq!(server.requests.lock().unwrap().len(), 2);
}

#[test]
fn rejects_credential_urls_and_repository_paths_before_connecting() {
    for base in [
        "https://user:secret@example.com",
        "file:///etc/passwd",
        "https://example.com/path",
        "https://example.com?token=secret",
        "https://example.com/#x",
    ] {
        let output = command()
            .args(["mcp", "--base-url", base])
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(!String::from_utf8_lossy(&output.stderr).contains("secret"));
    }
}

#[test]
fn protocol_negotiation_and_malformed_envelopes() {
    for version in ["2024-11-05", "2025-03-26", "2025-06-18", "2099-01-01"] {
        let mut init = initialize();
        init["params"]["protocolVersion"] = json!(version);
        let replies = mcp("http://127.0.0.1:1", &[init]);
        assert_eq!(
            replies[0]["result"]["protocolVersion"],
            if version == "2025-06-18" {
                version
            } else {
                "2025-11-25"
            }
        );
    }
    let replies = mcp(
        "http://127.0.0.1:1",
        &[
            json!([]),
            json!({"jsonrpc": "1.0", "id": 1, "method": "ping"}),
            json!({"jsonrpc": "2.0", "id": true, "method": "ping"}),
            rpc("initialize", json!({})),
        ],
    );
    assert_eq!(replies[0]["error"]["code"], -32600);
    assert_eq!(replies[1]["error"]["code"], -32600);
    assert_eq!(replies[2]["error"]["code"], -32600);
    assert_eq!(replies[3]["error"]["code"], -32602);
}

#[test]
fn malformed_json_recovers_and_oversized_frames_stop_the_process() {
    let mut child = command()
        .args(["mcp", "--base-url", "http://127.0.0.1:1"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    stdin.write_all(b"not json\n\xff\n").unwrap();
    writeln!(stdin, "{}", rpc("ping", json!({}))).unwrap();
    drop(stdin);
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let replies: Vec<Value> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect();
    assert_eq!(replies.len(), 3);
    assert_eq!(replies[0]["error"]["code"], -32700);
    assert_eq!(replies[1]["error"]["code"], -32700);
    assert_eq!(replies[2]["result"], json!({}));

    let mut child = command()
        .args(["mcp", "--base-url", "http://127.0.0.1:1"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let _ = stdin.write_all(&vec![b'x'; 1024 * 1024 + 1]);
    drop(stdin);
    let output = child.wait_with_output().unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("exceeds 1 MiB"));
}

struct SetupWorkspace(std::path::PathBuf);

impl SetupWorkspace {
    fn new() -> Self {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "mirrorproxy-agent-setup-{}-{stamp}",
            std::process::id()
        ));
        fs::create_dir_all(root.join("home")).unwrap();
        fs::create_dir_all(root.join("project")).unwrap();
        Self(root.canonicalize().unwrap())
    }
    fn cli(&self) -> Command {
        let mut cmd = command();
        cmd.current_dir(self.0.join("project"))
            .env("HOME", self.0.join("home"))
            .env("USERPROFILE", self.0.join("home"))
            .env_remove("CODEX_HOME");
        cmd
    }
}

impl Drop for SetupWorkspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn setup_binds_all_agents_merges_existing_configuration_and_rolls_back_exactly() {
    for scope in ["global", "project"] {
        let workspace = SetupWorkspace::new();
        let root = workspace
            .0
            .join(if scope == "global" { "home" } else { "project" });
        fs::create_dir_all(root.join(".codex")).unwrap();
        let codex_original =
            "# retain this comment\nmodel = 'example'\n[mcp_servers.other]\ncommand = 'existing'\n";
        fs::write(root.join(".codex/config.toml"), codex_original).unwrap();
        let claude_path = root.join(if scope == "global" {
            ".claude.json"
        } else {
            ".mcp.json"
        });
        let claude_original = r#"{"theme":"dark","mcpServers":{"other":{"command":"existing"}}}"#;
        fs::write(&claude_path, claude_original).unwrap();
        let server = Server::new("normal", catalog());
        let output = workspace
            .cli()
            .args([
                "setup",
                "--base-url",
                &server.base,
                "--scope",
                scope,
                "--agent",
                "codex,claude-code,cursor",
            ])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert!(stdout.contains("MCP handshake / instance catalog: OK"));
        let codex = fs::read_to_string(root.join(".codex/config.toml")).unwrap();
        assert!(codex.starts_with(codex_original));
        assert!(codex.contains(&server.base));
        for directory in [".agents", ".claude", ".cursor"] {
            let skill =
                fs::read_to_string(root.join(directory).join("skills/mirrorproxy/SKILL.md"))
                    .unwrap();
            assert!(skill.contains(&server.base) && skill.contains("--base-url"));
        }
        let claude: Value =
            serde_json::from_str(&fs::read_to_string(&claude_path).unwrap()).unwrap();
        assert_eq!(claude["theme"], "dark");
        assert_eq!(claude["mcpServers"]["other"]["command"], "existing");
        assert_eq!(claude["mcpServers"]["mirrorproxy"]["type"], "stdio");
        let cursor: Value =
            serde_json::from_str(&fs::read_to_string(root.join(".cursor/mcp.json")).unwrap())
                .unwrap();
        assert!(std::path::Path::new(
            cursor["mcpServers"]["mirrorproxy"]["command"]
                .as_str()
                .unwrap()
        )
        .is_absolute());
        assert_eq!(cursor["mcpServers"]["mirrorproxy"]["args"][2], server.base);
        let records = workspace.0.join("home/.mirrorproxy/agent-setups");
        let id = fs::read_dir(&records)
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .file_name()
            .to_string_lossy()
            .to_string();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(records.join(&id).join("manifest.json"))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
        }
        let output = workspace
            .cli()
            .args(["setup", "--scope", scope, "--rollback", &id])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            fs::read_to_string(root.join(".codex/config.toml")).unwrap(),
            codex_original
        );
        assert_eq!(fs::read_to_string(claude_path).unwrap(), claude_original);
        assert!(!root.join(".cursor/mcp.json").exists());
        assert!(!root.join(".agents/skills/mirrorproxy/SKILL.md").exists());
    }
}

#[test]
fn setup_validates_live_payload_before_writing_and_dry_run_is_offline() {
    let workspace = SetupWorkspace::new();
    let output = workspace
        .cli()
        .args([
            "setup",
            "--base-url",
            "http://127.0.0.1:1",
            "--scope",
            "global",
            "--agent",
            "codex",
            "--dry-run",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(!workspace.0.join("home/.codex").exists());
    let server = Server::new("html", catalog());
    let output = workspace
        .cli()
        .args([
            "setup",
            "--base-url",
            &server.base,
            "--scope",
            "global",
            "--agent",
            "codex",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(!workspace.0.join("home/.codex").exists());
    assert!(!workspace.0.join("home/.mirrorproxy").exists());
}

#[test]
fn setup_refuses_changed_files_on_rollback_and_rejects_traversal_ids() {
    let workspace = SetupWorkspace::new();
    let server = Server::new("normal", catalog());
    let output = workspace
        .cli()
        .args([
            "setup",
            "--base-url",
            &server.base,
            "--scope",
            "project",
            "--agent",
            "cursor",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let root = workspace.0.join("project");
    let id = fs::read_dir(workspace.0.join("home/.mirrorproxy/agent-setups"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .file_name()
        .to_string_lossy()
        .to_string();
    fs::write(root.join(".cursor/mcp.json"), "{\"user_changed\":true}").unwrap();
    let output = workspace
        .cli()
        .args(["setup", "--scope", "project", "--rollback", &id])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(root.join(".cursor/skills/mirrorproxy/SKILL.md").exists());
    assert_eq!(
        fs::read_to_string(root.join(".cursor/mcp.json")).unwrap(),
        "{\"user_changed\":true}"
    );
    let output = workspace
        .cli()
        .args(["setup", "--scope", "project", "--rollback", "../escape"])
        .output()
        .unwrap();
    assert!(!output.status.success());
}

#[cfg(unix)]
#[test]
fn setup_refuses_symlinked_agent_directories_before_any_writes() {
    use std::os::unix::fs::symlink;
    let workspace = SetupWorkspace::new();
    let root = workspace.0.join("project");
    symlink(workspace.0.join("home"), root.join(".cursor")).unwrap();
    let output = workspace
        .cli()
        .args([
            "setup",
            "--base-url",
            "http://127.0.0.1:1",
            "--scope",
            "project",
            "--agent",
            "codex,cursor",
            "--dry-run",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(!root.join(".agents").exists());
    assert!(!workspace.0.join("home/mcp.json").exists());
}

#[cfg(target_os = "linux")]
#[test]
fn setup_reads_a_real_terminal_when_standard_input_is_a_pipe_and_defaults_to_global() {
    let workspace = SetupWorkspace::new();
    let server = Server::new("normal", catalog());
    let quoted = |value: &str| format!("'{}'", value.replace('\'', "'\"'\"'"));
    let shell = format!(
        "printf unused | {} setup --base-url {}",
        quoted(env!("CARGO_BIN_EXE_mirrorproxy")),
        quoted(&server.base)
    );
    let mut child = Command::new("timeout")
        .args(["30", "script", "-q", "-e", "-c", &shell, "/dev/null"])
        .current_dir(workspace.0.join("project"))
        .env("HOME", workspace.0.join("home"))
        .env("USERPROFILE", workspace.0.join("home"))
        .env_remove("CODEX_HOME")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(b"\n1\n").unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("strongly recommended") && stdout.contains("Choose [1]"));
    assert!(workspace
        .0
        .join("home/.agents/skills/mirrorproxy/SKILL.md")
        .exists());
    assert!(!workspace.0.join("project/.agents").exists());
}

#[cfg(unix)]
#[test]
fn bootstrap_installs_verified_user_binary_and_passes_the_bound_instance_to_setup() {
    use std::os::unix::fs::PermissionsExt;
    let workspace = SetupWorkspace::new();
    let tools = workspace.0.join("tools");
    let payload = workspace.0.join("payload");
    fs::create_dir_all(&tools).unwrap();
    fs::create_dir_all(&payload).unwrap();
    let binary = payload.join("mirrorproxy");
    fs::write(
        &binary,
        "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$SETUP_CAPTURE\"\n",
    )
    .unwrap();
    fs::set_permissions(&binary, fs::Permissions::from_mode(0o755)).unwrap();
    let archive = workspace.0.join("client.tar.gz");
    assert!(Command::new("tar")
        .arg("-czf")
        .arg(&archive)
        .arg("-C")
        .arg(&payload)
        .arg("mirrorproxy")
        .status()
        .unwrap()
        .success());
    let hash_output = Command::new("sh").arg("-c").arg("if command -v sha256sum >/dev/null 2>&1; then sha256sum \"$1\"; else shasum -a 256 \"$1\"; fi").arg("checksum").arg(&archive).output().unwrap();
    assert!(hash_output.status.success());
    let checksum = workspace.0.join("client.sha256");
    fs::write(&checksum, hash_output.stdout).unwrap();
    let curl = tools.join("curl");
    fs::write(&curl, "#!/bin/sh\nwhile [ \"$#\" -gt 0 ]; do\n case \"$1\" in\n -o) destination=$2; shift 2;;\n http*) url=$1; shift;;\n *) shift;;\n esac\ndone\ncase \"$url\" in\n *.sha256) cp \"$SETUP_CHECKSUM\" \"$destination\";;\n *) cp \"$SETUP_ARCHIVE\" \"$destination\";;\nesac\n").unwrap();
    fs::set_permissions(&curl, fs::Permissions::from_mode(0o755)).unwrap();
    let capture = workspace.0.join("setup-args");
    let installer =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/install.sh");
    let output = Command::new("sh")
        .arg(installer)
        .args(["--setup", "--base-url", "https://personal.mirror.example"])
        .env("HOME", workspace.0.join("home"))
        .env_remove("MIRRORPROXY_INSTALL_DIR")
        .env_remove("MIRRORPROXY_DOWNLOAD_MIRROR")
        .env(
            "PATH",
            format!("{}:{}", tools.display(), std::env::var("PATH").unwrap()),
        )
        .env("SETUP_ARCHIVE", &archive)
        .env("SETUP_CHECKSUM", checksum)
        .env("SETUP_CAPTURE", &capture)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(workspace.0.join("home/.local/bin/mirrorproxy").exists());
    assert_eq!(
        fs::read_to_string(capture).unwrap(),
        "setup\n--base-url\nhttps://personal.mirror.example\n"
    );
}
