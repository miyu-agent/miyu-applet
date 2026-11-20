## DT-4 上：`.app` 里找得到资源目录

状态：已完成，2026-10-30 合进核心的 main（`git log --grep DT-4上`）。只有技术细节（核心的主会话 2026-10-29 同意照能力找）。

### 目的

macOS 的桌面端是一个 `.app`：程序在 `Miyu.app/Contents/MacOS/miyu`，资源只能放在 `Contents/Resources/` 下。核心原来只认程序旁边的 `resources/` 和上一级的 `share/miyu/`，在 `.app` 里起不来。做完以后多认一处：上一级下的 `Resources/miyu/`。

### 蓝图改哪几节

- 核心仓库 `internal/blueprint/store/resources.md`「怎么走」第 1 条第 2、3 款：三处照先后，`Resources/` 里还有图标这些别人的东西，所以认的是下面那一层 `miyu/`；照摆法认，不问是不是 macOS。装法的表加 Windows、macOS 的桌面端两行；「出错」`NotFound` 那句写三处；「守着它的」那一行跟着改。
- 核心仓库 `internal/designs/12-进程形态与分发.md` 第三节：装法的表加桌面端两行，「怎么找」加第三处。

### 不做什么

- Windows 的桌面端不用改：资源放在程序旁边的 `resources\`，照第一处就找得到。
- Linux 的 deb、rpm 照旧装进 `/usr/share/miyu/`（DT-6 用 Tauri 的 `files` 写）。
- 不动网页软件找自己包文件夹的办法（网页 W-975）。

### 验收

1. `cargo test -p miyu-store --lib resources::`：`.app` 那条认 `Contents/Resources/miyu/`，只有 `Contents/Resources/` 的不算；三处都有的照先后，删一处换下一处；都没有的报找过三处。改代码之前，新加的两条和改过的那条是红的。
2. 手写变异两个都被逮住：把第三处挪到第二处前面；只认 `Resources/` 不认下面那一层。
3. 本机门禁（`cargo xtask check`）全过；三台 CI 绿。

### 风险

- 别的装法的程序上一级恰好有一个 `Resources/miyu/`：排在两处原有的后面，原来找得到的照旧先找到，只多了一处兜底。
