## Miyu 托盘程序（miyu-applet）

Miyu 的托盘程序 `miyu-applet`（2026-11-03 项目主人定，图纸 `docs/blueprint/app.md`，原来的窗口版外壳 `desktop.md` 留作历史）：只有托盘图标，没有窗口；界面都在网页里，托盘调主程序 `miyu web` 用浏览器打开。核心在 `miyu-agent/miyu`，网页在 `miyu-agent/miyu-web`；安装包把核心、网页软件和托盘程序打在一起，用哪一版钉在仓库里（`pins.toml`）。仓库 2026-11-03 从 `miyu-desktop` 改名。

### 规矩

- 用核心只经门面 `miyu-client`（git 依赖，钉的提交和 `pins.toml` 的核心一样）：找数据根、连核心看状态、拉起核心。别的事调装在旁边的主程序（`miyu web`、`miyu core stop`）。不依赖网页的 crate。要它们多给什么，找核心、网页的会话加，不在托盘里另写一份。
- 协议、事件的形状以核心仓库的 `docs/blueprint/protocol.md` 为准；网页软件的命令以网页仓库的 `docs/blueprint/web-ui.md` 为准。
- 提交信息（2026-10-22 项目主人定：所有仓库一致，同核心 `AGENTS.md` 第 2 条）：标题写 `<类型>(<范围>): <这次改动的效果，说人话> (<编号>)`，类型是 `feat`、`fix`、`perf`、`refactor`、`docs`、`test`、`chore` 之一；编号是这条线的施工编号 `AP-<n>`（2026-11-03 以前是 `DT-<n>`），没有的不写括号；标题里不写只有看过施工单才懂的说法，也不写实现细节；正文三段：为什么、改了什么、怎么验；一件事一个提交；已经推了的不改。
- 一步一张施工单，放在 `docs/construction/`，格式照核心仓库 `docs/construction/README.md` 第四节；施工单、图纸和代码进同一个提交。
- 每个 `.rs` 文件不超过 500 行，测试也算；clippy `-D warnings`；不吞错误（不写 `let _ =`、`.ok()` 扔错误）。
- 给人看的字照三种语言（中文是正本），住在资源文件里，不写死在代码里；说法照通用软件的短句。
- 图纸在 `docs/blueprint/`；改了行为先改图纸。
- 加第三方依赖之前先看许可证：仓库是 AGPL-3.0-or-later，依赖要能和它合在一起发。
- 本机编译整条套整机编译锁 `flock ~/.cache/miyu-cargo.lock …`；跑测试、起程序不占锁（2026-10-22 项目主人定：锁只管编译）。
- 本机测：沙箱数据根（`MIYU_HOME` 指到单独的目录），不碰 `~/.miyu`，不重启别人在用的核心；key 和地址只放在命令里。
- 平台：Linux、macOS（M 系列）、Windows 都要编得过、测试过。和平台有关的关在边界上，代码问「这里能不能做」，不问「这是不是 macOS」。
- 用中文回复。每次回复末尾报总体进度和把握，附上依据。
