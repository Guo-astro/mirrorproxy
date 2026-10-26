# Agent 接入：Skill 与 MCP

MirrorProxy 提供可移植的 Skill，以及集成在 `mirrorproxy` 客户端中的原生
MCP stdio 服务，用于发现现有实例的源目录、已启用适配器、公开访问限制与配额策略。
也可通过独立授权的受控下载通道获取公网文件；不提供无限制正向代理，也不直接修改客户端配置。

## 安装

### 推荐：从 WebUI 一次完成配置

在公开首页找到 **一键配置 AI Agent**，复制 Linux/macOS 或 Windows PowerShell
命令。安装器会下载并校验原生客户端，再运行
`mirrorproxy setup --base-url <当前实例>`。已登录且分配了个人代理子域名时，
命令使用个人地址。此流程需要已发布包含 `setup` 命令的稳定客户端；不会让旧版
稳定 Release 自动获得尚未发布的源码功能。

首页从同一服务端构建提供 `/install.sh` 和 `/install.ps1`，不再依赖可能过期的
`main` 分支脚本。下载后先校验摘要，再检查客户端是否支持 `setup`，通过后才替换
已有客户端。旧版缺少此能力时会直接拒绝，已有二进制保持不变。发布新版稳定客户端
仍是单独的发版步骤。

Windows 的一行命令将下载文本作为脚本块运行，并显式传入参数；不运行
`Set-ExecutionPolicy`，不修改机器或用户执行策略。企业应用控制和组策略限制仍然
生效。

本地向导首先询问安装范围：

1. **全局（当前用户，强烈推荐，默认选项）**：所有项目均可使用。
2. **仅当前项目**：Skill 和 MCP 配置放在选定项目中。

随后可多选 Codex、Claude Code、Cursor，检测到 PATH 中的程序或已有初始化目录的 Agent 会作为
默认选择。无交互终端时必须显式指定 `--agent` 与 `--scope`，不会因为无法询问
就静默全局安装。Unix 向导从 `/dev/tty` 读取输入，不受安装脚本管道占用 stdin
影响；Windows 从当前控制台读取。

已有客户端时可直接运行：

```sh
mirrorproxy setup --base-url https://mirror.example.com
# 无人值守配置，写入前仍会验证当前实例：
mirrorproxy setup --base-url https://mirror.example.com --agent codex,cursor --scope global
# 只预览路径，不联网、不写入：
mirrorproxy setup --base-url https://mirror.example.com --agent codex --scope project --dry-run
```

Unix 完整安装默认把客户端放到 `~/.local/bin`，可通过原有参数覆盖；仅安装 CLI
时保留原来的默认目录。Windows 沿用现有用户目录及 PATH 处理。MCP 配置和绑定后的
Skill 使用客户端绝对路径，因此 Agent 不依赖 shell 重启或 PATH 更新。向导不会
安装 Agent 应用本身。

| Agent | 全局 Skill | 全局 MCP | 项目 MCP |
| --- | --- | --- | --- |
| Codex | `~/.agents/skills/mirrorproxy/SKILL.md` | `$CODEX_HOME/config.toml`，默认 `~/.codex/config.toml` | `.codex/config.toml` |
| Claude Code | `~/.claude/skills/mirrorproxy/SKILL.md` | `~/.claude.json` | `.mcp.json` |
| Cursor | `~/.cursor/skills/mirrorproxy/SKILL.md` | `~/.cursor/mcp.json` | `.cursor/mcp.json` |

项目 Skill 使用项目下对应目录，Codex 使用 `.agents/skills`。项目模式下可用
`--project-dir` 指定项目。参见官方 [Codex MCP](https://developers.openai.com/codex/mcp/)、
[Codex Skills](https://developers.openai.com/codex/skills/)、
[Claude MCP](https://code.claude.com/docs/en/mcp)、
[Claude Skills](https://code.claude.com/docs/en/skills)、
[Cursor MCP](https://prod.cursor.com/help/customization/mcp) 和
[Cursor Skills](https://cursor.com/docs/skills) 文档。

向导保留其他 MCP 服务及设置，也保留 Codex TOML 注释。已有 MirrorProxy 配置
发生冲突时会询问，非交互替换必须显式指定 `--replace-existing`。拒绝符号链接
路径和无效配置文件，不在 Skill 或 MCP 配置中嵌入管理员或会话 Token。
Agent 原有信任和工具授权策略保持有效，配置完成后需重启 Agent。

写入前，客户端实际验证 MCP 握手、工具列表和实例源目录响应；这验证的是发现能力，
不代表文件下载健康或当前用户配额可用。改动会备份到
`~/.mirrorproxy/agent-setups/<id>/manifest.json`，项目模式也不把可能包含已有凭据的
备份放入项目。Unix 备份文件权限为 `0600`；Windows 文件继承用户目录的访问控制。
写入失败时自动恢复仍可识别的文件，需要恢复时会打印明确的恢复 ID。
配置解析错误不会打印可能包含凭据的源文件行。

使用向导打印的 ID 回滚：

```sh
mirrorproxy setup --scope global --rollback <id>
# 在原项目执行，或增加 --project-dir：
mirrorproxy setup --scope project --rollback <id>
```

回滚精确恢复原文件，删除本次新建的文件；如果文件在安装后被修改，会拒绝覆盖。
受保护的备份清单会保留，客户端二进制也会保留。“全局”指当前用户，不是整台机器
所有用户。本地 Agent 配置不等于云端 Agent 安装。

### 从源码构建客户端

使用包含 `mirrorproxy mcp --help` 和 `mirrorproxy agent --help` 的客户端构建。
尚未发布的源码版本可以这样编译：

```sh
cargo build --locked --release -p mirrorproxy-cli
./target/release/mirrorproxy mcp --help
```

原生二进制运行时无需 Python 或 Node。正常的客户端 Release 压缩包与原生安装包
分发同一个二进制；旧版本若没有这些命令，需要等包含此功能的版本发布后升级。
服务端无需升级或迁移数据库，也没有新增接口：发现功能复用
`/api/public-config` 和 `/api/sources`。

### 一条命令安装 Skill

`skills/mirrorproxy/SKILL.md` 发布到仓库默认分支后，在使用 Agent 的项目中执行：

```sh
npx skills add inbjo/MirrorProxy --skill mirrorproxy
```

安装器会让你选择目标 Agent，默认安装到当前项目；加 `-g` 可安装到用户全局目录。
安装器目前需要 Node.js 22.20+，原生 MirrorProxy 二进制运行时无需 Node。
详见[安装器官方说明](https://github.com/vercel-labs/skills)。
尚未发布的本地版本，可在仓库根目录执行 `npx skills add . --skill mirrorproxy`。
公网安装命令无法安装尚未推送的本地改动。

手动安装时，把整个 `skills/mirrorproxy` 目录复制到 Agent 配置的 Skill 目录，
保留 `mirrorproxy/SKILL.md` 结构，并把新版 `mirrorproxy` 放到 Agent 的 PATH 中。
安装 Skill 只安装使用说明，不会安装原生客户端、配置 MCP 连接或选择实例地址。
未连接 MCP 时，Skill 可以直接调用下文的 JSON 查询命令；访问 MirrorProxy 网站
不会自动加载 Skill。

### Skill 目录与市场

- **skills.sh**：建议优先接入。其文档说明通过 `skills` CLI 的安装遥测发现和排行，
  Skill 本身无需另发 npm 包；被收录也不代表获得平台官方背书。
  详见 [skills.sh FAQ](https://skills.sh/docs/faq)。
- **ClawHub**：另一种发布渠道，需要发布者账号，上传版本会经过校验和自动安全检查。
  发布的 Skill 适用其 MIT-0 条款；上传仓库当前采用 MIT 许可的 Skill 前应先确认
  授权安排。详见[发布说明](https://github.com/openclaw/clawhub/blob/main/docs/publishing.md)
  和 [CLI 授权条款](https://github.com/openclaw/clawhub/blob/main/docs/cli.md)。

通用市场安装方式不绑定实例，也不注册 MCP；需要完整体验时使用 WebUI 的配置向导。
仓库提供的是可安装的 Skill 源码，不代表 MirrorProxy 已在上述平台被收录、审核或
发布。市场发布和授权决定，与本地安装验证分别处理。

## 连接 MCP

在支持 MCP 的应用中配置 stdio 服务。常见的 JSON 配置形状为：

```json
{
  "mcpServers": {
    "mirrorproxy": {
      "command": "mirrorproxy",
      "args": ["mcp", "--base-url", "https://mirror.example.com"]
    }
  }
}
```

如果应用不继承 PATH，请使用二进制绝对路径。配置文件位置和外层字段以具体应用
要求为准。实例地址在启动时固定，工具无法选择其他实例或直接连接任意上游。受控下载通过绑定的实例执行服务端策略。
这是本地 stdio 进程，不是托管的 `/mcp` HTTP 接口。它使用逐行 JSON-RPC，支持
初始化、ping、工具发现与调用。诊断写入 stderr，stdout 仅输出协议消息。
支持的协议版本为 `2025-06-18` 和 `2025-11-25`，不声明支持需要 JSON-RPC 批处理的旧协议。

| 工具 | 参数 | 返回内容 |
| --- | --- | --- |
| `inspect_instance` | 无 | 已启用适配器、公开访问模式与配额策略 |
| `list_sources` | 可选 `category`：`lang`、`os`、`repo`；`enabled_only`：布尔值 | 实时源元数据及映射，包括自定义操作系统仓库 |
| `get_source` | `target`：源代号或别名 | 对应源、地址、限制及供参考的配置模板 |
| `check_download` | `url`：公网 HTTPS 地址 | 认证后的 HEAD/策略检查，不预留配额 |
| `download_file` | `url`、`output`，可选 `max_bytes` | 新建本地文件、字节数及 SHA-256，需本地下载 token |

每条映射的 `enabled` 值：MirrorProxy 代理路由为 `true` 或 `false`；第三方镜像及
仅指导用途的映射为 `null`。启用状态不是健康检查，也不证明当前用户获得了访问
授权。公开配额策略不包含个人用量或剩余额度。模板是数据，MCP 不执行其中的命令。
实例返回的文本应当作为不可信数据处理。

## 不使用 MCP 的查询方式

```sh
mirrorproxy agent --base-url https://mirror.example.com instance
mirrorproxy agent --base-url https://mirror.example.com sources --enabled-only
mirrorproxy agent --base-url https://mirror.example.com sources --category lang
mirrorproxy agent --base-url https://mirror.example.com source pip
```

实例地址必须为 HTTP(S) origin，不带凭据、查询参数、片段或仓库路径。除本地开发外
建议使用 HTTPS。查询会验证 TLS、拒绝重定向、忽略环境代理设置，不发送用户或
管理员凭据，仅访问固定公开 API 路径。允许使用本地地址，便于自托管。
网络请求超时为十秒，响应上限为 4 MiB，MCP 输入帧上限为 1 MiB。
发现调用不会下载文件或跟随目录中的地址。

使用实例的规范地址发现能力；实际包流量若要求用户子域名，应保留用户分配到的
地址。公开能力查询不会绕过用户访问规则，也不会授予代理权限。

## 配置、验证与回滚

可以对 Agent 说：“使用我的 MirrorProxy 实例查看可用 Python 源，并预览 pip
改源。”Skill 先查询实时能力，再使用现有改源 CLI：

```sh
mirrorproxy sources get pip --base-url https://mirror.example.com
mirrorproxy sources set pip --base-url https://mirror.example.com --dry-run
# 仅在本次任务已授权改源时应用：
mirrorproxy sources set pip --base-url https://mirror.example.com
mirrorproxy sources reset pip --dry-run
mirrorproxy sources reset pip
```

set/reset 应使用相同作用域。系统仓库还可能需要指定发行版，或使用
`--scope system --replace-existing`；详见[客户端](Client-zh-CN)与
[系统源](System-Sources-zh-CN)。保留备份与非托管文件保护，使用原生客户端操作
以及官方摘要或签名验证，不能仅凭源目录或 HTTP 200 断言代理可用。

Docker `registry-mirrors` 只适用于 Docker Hub，其他 OCI 仓库必须显式改写镜像
主机名。Composer 支持不代表支持 PHP 源码压缩包。Playwright 浏览器压缩包需要
匹配的下载适配器；Chromium 操作系统依赖可以按情况使用已有系统源。
通用 `/proxy?url=...` 下载接口尚未实现。独立的认证下载通道见[自定义下载](Custom-Downloads-zh-CN)。

移除接入时，删除 MCP 配置与已安装的 Skill 目录即可，也可使用
`npx skills remove mirrorproxy` 卸载（全局安装时加 `-g`）。能力查询本身不修改客户端
文件；如果另外应用过改源，请使用 CLI 现有回滚机制恢复。

## 受控下载

[下载授权、CLI 凭据、MCP 工具及域名统计](Custom-Downloads-zh-CN)。
