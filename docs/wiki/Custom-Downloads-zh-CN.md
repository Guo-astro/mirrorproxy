# 受控自定义下载

此通道用于补充现有镜像源，下载 PHP 等源码归档或浏览器文件。默认关闭，不提供
HTTP 正向代理、CONNECT 隧道、上传或私有制品访问。

## 开启与授权

超级管理员在 `/admin` 的「自定义下载」页面开启通道，向注册用户或已有分组授权。
注册本身没有权限。用户直接拒绝优先于组授权；否则用户直接允许或任一组允许即可。
授权列表显示用户的姓名、账号和邮箱，或分组名称，ID 仅作辅助标识。
「撤销权限」删除对应的直接授权规则；用户之后仍可能通过分组获得权限，
没有任何分组授权时默认禁止下载。「撤销禁止」删除直接禁止规则，可能恢复分组授予的权限。
分组不授权只表示该组不授予权限，不覆盖用户的单独允许或其他组的允许；
此类条目使用「移除规则」按钮。
个人中心的有效 token 以表格显示名称、用途、到期时间与撤销操作。
「仅用于自定义下载」对应技术作用域 `download:fetch`，不代表登录或管理权限。
组成员仍在「用户与分组」中管理。

用户在 `/account` 创建具名、有期限的下载专用 token。作用域仅为 `download:fetch`，
仅创建时以明文显示，输入框旁可直接复制；刷新或离开页面后无法再次查看，
服务端只保存哈希。若创建成功但列表刷新失败，请先保存已显示的 token；
「重试」只刷新列表，不会再次创建。每人最多 20 个有效 token，有效期 1–365 天。
下载 token 不能登录或管理实例；浏览器会话和管理员 token 也不能用于下载认证。
传输中会重新检查账号状态、权限和 token 有效性。撤销 token 会终止该用户的活跃下载；
更改权限会取消活跃下载，使策略及时生效。

## CLI 与 MCP

安装包含 `download` 命令的客户端。创建 token 后，个人中心提供 Linux/macOS 和
Windows PowerShell 两种完整配置命令。复制后在本机终端执行即可，无需再次输入：

```sh
printf '%s\n' '你的下载token' | mirrorproxy download --base-url 'https://mirror.example.org' auth --token-stdin
```

```powershell
'你的下载token' | mirrorproxy download --base-url 'https://mirror.example.org' auth --token-stdin
```

看到 `Download credential saved for this instance.` 即保存完成。命令包含 token，
只在本机使用，勿分享；终端可能保存命令历史。新客户端也支持单独执行
`mirrorproxy download --base-url https://mirror.example.org auth --token-stdin`，
按提示粘贴 token 并回车。旧客户端会等待 EOF：Unix 为 Ctrl-D，Windows 为 Ctrl-Z
后回车；上面的管道命令兼容旧客户端，无需 EOF 操作。不要把 token 发送到聊天、
Skill、项目文件或共享 MCP 配置。

凭据按规范化后的精确实例地址保存在 `~/.mirrorproxy/credentials`。
Unix 权限为 0600，Windows 继承用户目录 ACL，请勿使用共享用户目录。
受控本地进程可以通过 `MIRRORPROXY_DOWNLOAD_TOKEN` 环境变量覆盖凭据。
实例认证必须使用 HTTPS，仅回环开发地址例外。客户端拒绝实例地址的重定向。

```sh
mirrorproxy download --base-url https://mirror.example.org check https://example.org/source.tar.xz
mirrorproxy download --base-url https://mirror.example.org fetch https://example.org/source.tar.xz --output source.tar.xz
mirrorproxy download --base-url https://mirror.example.org auth --logout
```

拒绝已有输出和符号链接；先写私有临时文件，失败删除，成功通过硬链接发布完整文件，
不会覆盖目标。目标文件系统需支持硬链接。返回路径、大小和 SHA-256，仍需与可信
上游摘要或签名比对。本地默认上限 2 GiB，`--max-bytes` 可调整至最多 64 GiB，
同时受服务端上限约束。

现有 MCP 连接直接增加 `check_download` 和 `download_file`，无需安装第二个服务。
工具读取同一份本地凭据，不接收或返回 token。`inspect_instance` 的
`controlled_download` 仅表示公开策略，不代表用户权限或剩余额度。
预检查通过服务端发起 HEAD，不预留配额，也不保证稍后下载成功。
`download_file` 创建本地文件并消耗配额，工具标记为写入、非幂等。
文件正文不进入模型上下文。已有 Skill 可重新运行 setup 更新；重启 Agent 刷新工具列表。

## HTTP 接口与安全边界

- `GET /api/download?url=<编码后的 HTTPS URL>`：流式附件下载。
- `HEAD /api/download?url=...`：仅元数据。
- `GET /api/download/check?url=...`：认证后的策略与 HEAD 检查，返回 JSON。
- 均要求 `Authorization: Bearer <下载 token>`，不支持 URL token。
- 只允许端口 443 和公网 DNS 域名，拒绝 IP 字面量、URL 凭据、片段及私网或特殊 IPv4/IPv6 地址。
- DNS 全部结果必须合规，实际连接绑定已检查的 IP，保留 TLS 主机名验证。
  每次跳转重新校验 URL、DNS 和域名封禁，最多五次；禁止 HTTP 降级和其他协议。
- 不使用环境或镜像上游代理，不继承不安全 TLS、上游凭据、Cookie 或客户端请求头。
- HTTP 接口支持单段 `bytes=start-end` 或 `bytes=start-` Range，取决于上游支持情况；
  拒绝后缀和多段 Range。CLI 仅执行完整下载。
- 返回强制附件、octet-stream 和 `no-store`，移除上游 Cookie、HTML 类型和认证挑战。
- 上游错误经过脱敏，统计与应用请求日志不保存完整 URL、路径或查询参数。
  反向代理也应避免记录可能包含签名的下载查询参数。

## 限额、统计与封禁

配置见 `config.example.toml` 的 `[download]`：默认每文件 10 GiB、每用户每天
20 GiB（UTC）、实例并发 16、用户并发 5、每用户每分钟 300 次、总时限 30 分钟、
实例共享出口速率 100 MiB/s。默认仍关闭，已保存的策略保持不变。配置沿用现有运行配置持久化机制，保存会取消当前下载。

下载同时受已有实例、用户及计费组月配额约束。每个数据块入队前在同一事务中检查
并扣减额度。月计费沿用双向计费设置，每日额度按输出字节计一次。
客户端断开时，已入队的少量缓冲数据可能已计费。传输中错误会中断响应，CLI 删除
临时文件；响应头发出后无法改变 HTTP 状态码。

管理员按最终精确主机名、用户和日期查看请求、错误、拒绝、输出流量及回源正文流量，
明细关联原始入口域名。本版不使用共享缓存。DNS/响应头失败不产生正文流量；
中断前已经读取的回源正文仍记录。统计保留 90 天，不保存 URL 路径或查询参数。
域名和用户榜各最多 200 项，明细最多 2000 行；总计和每日汇总覆盖全部匹配记录。
支持按精确入口/最终域名、用户及 1–90 天筛选。

封禁支持「仅此域名」或「包含子域名」，按完整域名标签匹配。入口及所有跳转都检查，
封禁同时取消关联下载。操作记录管理员、时间和原因。活跃列表支持终止单个下载。
封禁共享 CDN 可能影响多个项目，应选择合适的最小域名范围。

## 升级与回退

数据库 v5 仅增加表，不改变已有用户、会话或镜像路由，不自动授权任何用户。
升级前备份 SQLite。回退功能时先关闭下载，按需撤销 token，并通过 `auth --logout`
删除本地凭据。现有镜像客户端及能力发现工具继续可用。源码开发不代表已发布或部署。
