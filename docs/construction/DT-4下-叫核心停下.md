## DT-4 下：头能叫核心停下

状态：已完成，2026-10-30 合进核心的 main（`git log --grep DT-4下`）。形状核心的主会话 2026-10-29 定了。

### 目的

项目主人 2026-10-29 定：桌面端托盘右键「退出」连核心一起关，彻底退出。核心原来只在收到停的信号时停，头没有办法叫它停：Unix 上头不知道核心的进程号，Windows 上拉起的核心脱了控制台、发不了 Ctrl 事件。做完以后多一个方法 `core.stop`：出示本机令牌连上的头调了，核心照「收到停的信号」那一套停下。

### 蓝图改哪几节

- 核心仓库 `internal/blueprint/protocol.md` 方法表加 `core.stop`：没有参数，回应 `{}`；只给出示本机令牌连上的，闸照 `account.setup_code`：扩展、设备令牌、访问密码、交接票连上的回 `local_only`，远程的回 `local_machine_only`，一次性码连上的照旧先回 `setup_first`。
- 核心仓库 `internal/blueprint/core.md`「停下」第 10 条（新）、第 7 条、运行日志的表、「在哪」「守着它的」：停的原因多一种 `request`（头叫停的），和信号一样先停下会话（跑到一半的回合记成重启了）、请扩展退出、等记忆在后台的读写，运行日志 `stopped reason=request`。回应先写出去，端点再叫一声（`Activity::stop_requested`）。
- `docs/zh/reference/`、`docs/en/reference/` 的中英两份 `connection` 加 `core.stop` 的条目；`crates/miyu-endpoint/src/connection/rights.rs` 的权限表把它归到「只给人」；协议 1 的方法清单 `methods-v1.txt` 照 `cargo xtask methods` 重出。

### 不做什么

- 命令行入口 `miyu core stop` 由核心的 CL 线加（核心的主会话 2026-10-29 答应），这一步只做方法。
- 不停网页软件：核心停了，它照自己的规矩空闲 10 分钟退。
- 浏览器标签页还开着的，重连 `/ws` 时网页软件会把核心再拉起来：当成可以接受（核心的主会话 2026-10-29 同意），不另拦。
- 不管服务管理器：核心照服务跑的，正常退出（退出码 0）服务管理器不会重起它，下次登录照服务的规矩起。

### 验收

1. `crates/miyu-core/tests/serve.rs` 新的一条：有回合在跑时本机令牌的头调 `core.stop`，先收到 `{}`，核心停下、原因 `Stopped::Request`，跑到一半的回合记成重启了。
2. `crates/miyu-endpoint/tests/core_stop.rs`：本机令牌的回 `{}` 以后叫停到了；设备令牌连上的 `local_only`、远程的 `local_machine_only`，300 毫秒里没叫停，设备那条连接照旧能用。`local_machine.rs` 的本机才能用的名单、`connection/tests.rs` 的只给人的名单各加一行。
3. 上面三处新加的改代码之前都是红的；手写变异五个都被逮住：不叫停、谁都能叫停、远程不挡、原因写成信号、叫停不先停会话。
4. 本机门禁全过（文档、方法清单两项会拦漏登记的）；三台 CI 绿。

### 风险

- 回应还没写出去连接就断了：头分不清是停了还是出错。回应写出去以后再发停的信号，测试断言头先收到 `{}`。
- 叫停和空闲退出、信号同时到：照先到的那一个停，只停一次。
