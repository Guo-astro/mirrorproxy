# 公网自定义下载与真实 Agent 容器验收

日期：2026-10-28。目标实例：`https://sina.dev`。用户明确授权使用其下载 token 验证；本文不记录该凭据。

## 隔离边界

所有真实 Agent 的安装、用户配置、Skill、MCP 批准、桌面应用和登录状态检查都在临时 Docker 容器中进行。主容器只挂载一个只读文件：本地构建的 `mirrorproxy` 二进制；未挂载主机 HOME、任何 Agent 配置、工作区源码、SSH 密钥或模型账号凭据。Linux 桌面程序通过容器内 Xvfb 启动。下载凭据通过 stdin 设置，测试后注销，容器凭据目录为空。没有创建或撤销生产 token、修改公网配置、发布或部署。

## 真实下载结果

### 小文件：CLI 与 MCP 均通过

文件：`idna-3.10-py3-none-any.whl`，官方元数据来自 [PyPI idna 3.10 JSON API](https://pypi.org/pypi/idna/3.10/json)。文件 URL 指向官方 `files.pythonhosted.org`。

- 大小：70,442 字节。
- PyPI 官方 SHA-256：`946d195a0d259cbba61165e88e65941f16e9b36ea6ddb97f00452bae8b1287d3`。
- 本地修复版 CLI：保存凭据 → 公网 HEAD 预检查 → 公网下载 → 摘要核对，全部通过。
- 官网安装器现发 CLI：在 `/opt/public-mirrorproxy` 独立安装，安装器校验 Release 摘要；管道写入 token 后实际下载，大小和 SHA-256 同样通过。
- 实际 Codex 注册的 MCP 命令：从 `codex mcp get mirrorproxy --json` 读取实际 command/args，启动该 MCP 进程，完成 initialize / tools/list / check_download / download_file。下载文件的大小与 SHA-256 同样匹配。此项是配置对应 MCP 进程的真实协议验收，不冒充模型自主调用。

### 较大文件：公网自定义下载通过

安装真实 Cursor Agent 时，原始直连下载很慢；经同一授权身份的 `sina.dev` 自定义下载通道取得官方 `downloads.cursor.com` 分发的 Linux x64 归档。

- 官方归档版本路径：`2026.10.22-27dcf5f/linux/x64/agent-cli-package.tar.gz`。
- 大小：184,859,176 字节。
- 客户端计算 SHA-256：`85296944e4600a83c4b764f578d1502bf9c416db200d33a593e8f3ecabfdc294`。
- 归档成功解包，真实 Cursor Agent 启动并连接 MCP。
- 该 SHA-256 是实际下载得到的摘要，不冒充厂商独立发布的签名或摘要认证。

## 用户截图中的“输入后没反应”

用实际官网下载客户端与本地修复版分别连接 Docker PTY，以合成 token 测试输入，不调用生产下载接口：

| 客户端 | 粘贴提示 | 粘贴后 Enter | 后续结果 |
| --- | --- | --- | --- |
| 公网现发版本 | 无 | 继续等待 EOF | Ctrl-D 后成功保存 |
| 本地修复版 | `Paste download token and press Enter:` | 立即完成 | 成功保存 |

原因是旧版通过 `read_to_string` 读取 stdin 到 EOF。修复版在真正终端中读取一行，在管道中保留 EOF 行为。完整管道命令兼容旧客户端，无需等待升级。实际客户端还验证了凭据文件为 0600，以及注销后文件删除。

## 真正安装的 Agent 与实际结果

版本是本次实际安装运行的版本，不表示长期最新版本。全局与项目两种 MirrorProxy setup 均绑定 `https://sina.dev`；本次同时安装全部适配器和两个作用域，因此部分客户端报告同名 Skill 覆盖，实际选中的 Skill 内容相同且实例一致。

| 程序 | 实际版本 | 实际验证 | 未完成边界 |
| --- | --- | --- | --- |
| Codex CLI | 0.165.0 | 原生 `mcp list/get`；实际 app-server `skills/list` 识别 MirrorProxy；`mcpServerStatus/list` 返回 5 个工具 | 容器未登录；未执行付费模型自主下载任务 |
| Claude Code | 2.1.312 | 原生 `mcp list` 显示 Connected；真实启动日志加载用户/项目 Skill 目录，MCP 初始化成功 | `claude auth status` 为未登录，模型请求被认证限制终止 |
| Cursor Agent | 2026.10.22-27dcf5f | 真正安装官方 Linux 归档；通过原生 `mcp enable mirrorproxy` 正常批准；`mcp list` ready，`list-tools` 返回全部 5 工具 | 未登录，不能声称模型读取 Skill 并自主使用工具 |
| Gemini CLI | 0.66.0 | 原生 `skills list` 发现 MirrorProxy；在正常交互信任本次测试目录后，`mcp list` Connected | 未配置 Gemini 模型账号/API 凭据 |
| OpenCode | 1.19.4 | 原生 `debug skill` 返回 MirrorProxy 内容；`mcp list` connected | 尝试官方免登录 `opencode/beam-free` 模型，HTTP 403 RegionError：当前国家/地区不可用；未更换网络或身份绕过 |
| VS Code + Copilot | VS Code 1.143.1，内置 Copilot Chat 0.71.0 | 官方 deb 真正安装；通过 `code --add-mcp` 导入；Xvfb 中实际启动扩展宿主，通过正常 UI 信任测试目录；实际 `vscode.lm.tools` 出现 5 个 `mcp_mirrorproxy_*` 工具 | Copilot 账号未登录；工具加载成功不等于模型执行成功 |
| GitHub Copilot CLI（额外验证） | 1.0.105 | 原生 `skill list` 识别 MirrorProxy；通过 `copilot mcp add` 导入，`mcp list/get` 显示 Enabled | CLI 的原生导入是附加步骤，不冒充 VS Code 向导自动配置；未进行模型任务 |
| DeepSeek Harness | 0.2.1-rc.2 | 实际 `dsh web --patch ... --dump-config` 解析成功；Web profile 真正启动，包含 Skill 文件系统插件；进程树确认 DSH 启动了 MirrorProxy MCP 子进程 | 未配置模型凭据，未声称模型调用通过 |
| TRAE 国际版 | 官方包 2.3.91093，底层应用 1.107.1 / b014a319… | 官方 Linux deb 真正安装，CLI 版本命令与 Xvfb 中应用进程、扩展/主进程日志产生 | 未登录；未取得 Skill/MCP 工具已在 Agent 中激活的运行时证据，不能记为全通过 |
| TRAE CN | 官方包 2.3.97430，底层应用 1.107.1 / e96bc25b… | 官方中国站 Linux deb 真正安装，CLI 与 Xvfb 中应用启动 | 同样未登录，Skill/MCP 激活未完成 |

`generic` 是通用 Skill/MCP 导出格式，本身没有可安装的 Agent 程序。WorkBuddy 官方 Linux 页面显示“请到系统应用商店下载”，未提供本环境可用的独立 Linux 包；本次没有将导出 connector 文件当成实际客户端安装。

这份表格严格区分安装、配置识别、MCP 连通、模型自主执行四个阶段。没有模型凭据以及厂商地区限制的项目，仍是未完成的模型端到端验收，不能以配置文件存在或程序 `--version` 成功代替。

## 表格的纵向滚动修复验证

用户明确反馈的是纵向滚动条。本地新版本对两张表格及其所有后代节点执行了真实 Chromium 计算样式检查，使用 20 条 token / 20 条会话，并覆盖 1440 / 768 / 375 / 320px：

- 两张表格的 `overflow-y` 为 `visible`，没有固定高度或最大高度。
- 后代节点中，没有同时满足 `overflow-y: auto/scroll` 且 `scrollHeight > clientHeight` 的节点。
- 20 条记录时表格自然高于 900px；末行按钮通过整页滚动可见。
- 页面无横向溢出。手机改为带字段名称的记录行，不用内部滚动条隐藏内容。
- 新增浏览器回归 `lets token and session records grow with the page without nested vertical scrollbars` 通过；最终完整浏览器回归为 60 项通过。

这证明本地修复后的表现；本轮尚未重新部署到公网上。

## 验证命令与来源

- `cargo fmt --all -- --check`、全工作区 Clippy、Rust 测试（444 通过、1 忽略）已通过。
- 前端测试 62 项、生产构建、完整 Chromium 60 项均通过。
- 新版静态 CLI 的实际管道认证、PTY 输入、0600 写入与注销通过。
- `git diff --check` 通过。

安装来源：[Codex 官方安装](https://learn.chatgpt.com/docs/codex/cli)、[Claude Code 官方安装](https://code.claude.com/docs/en/setup)、[Gemini CLI 官方安装](https://geminicli.com/docs/get-started/installation/)、[OpenCode 官方安装](https://opencode.ai/docs/)、[Cursor 官方安装](https://cursor.com/docs/cli/overview)、[VS Code Linux 安装](https://code.visualstudio.com/docs/setup/linux)、[DeepSeek Harness 官方项目](https://github.com/deepseek-ai/deepseek-harness)、[TRAE 国际版下载](https://www.trae.ai/download)、[TRAE CN 下载](https://www.trae.cn/download)、[WorkBuddy 官网](https://www.workbuddy.cn/)。
