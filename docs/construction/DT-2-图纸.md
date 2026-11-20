## DT-2：图纸，桌面端照薄封装写进三个仓库

状态：已完成（2026-10-30）。三个仓库的图纸都合进各自的 main（`git log --grep DT-2`）：这里的 `docs/blueprint/desktop.md`；核心的设计 01、12、21；网页的 `web/architecture.md`「宿主」、`web.md`、`web-ui.md`（网页的会话审过、三平台 CI 绿）。

### 目的

项目主人 2026-10-29 定桌面端做网页的薄封装（`README.md` 第一节），DT-1 在 Linux 上试过走得通。但核心的设计 01 还写着桌面端连核心、`head.kind` 写 `desktop`，网页的 `web/architecture.md`「宿主」还是 2026-09-30 那一版厚封装（`tauri.js`、`miyu://`、外壳经进程间通道连核心）。写外壳（DT-3）之前先把三个仓库的图纸对上，外壳照 `desktop.md` 写。

### 蓝图改哪几节

- 这里：新开 `docs/blueprint/desktop.md`：调主程序的两条命令、找主程序、开窗口、导航和下载、通知权限、关窗口、托盘、彻底退出、语言、装在哪、给人看的字、守着它的。
- 核心仓库：`internal/designs/01-架构.md` 第一节「头」那一条；`21-网页.md` 状态行、第一节最后两条、第九节加 X9 桌面端（已定、未选 A 厚封装、B Electron、C 关窗口就退）；`12-进程形态与分发.md` 单独分发的程序那一条、R2 的「桌面程序第一版不做」改成外壳 `miyu-desktop`。核心的主会话 2026-10-30 看过要点、同意，由这边在核心开 `step/DT-2-图纸` 改（施工图卡片批号 502）。
- 网页仓库：`docs/blueprint/web/architecture.md`「宿主」整节照薄封装改（`served.js` 一份，`tauri.js`、`desktop/`、`miyu://`、`frontendDist`、桥拆成库都不做）、「页面自己守的」第 2 到 7 条、「宿主怎么搬」第 3、4 条、「在哪」、多用户那张表；`web.md` 第 29、48、560 行；`web-ui.md` `--ticket` 那款补一句。

### 要拍板的

- macOS 的 `⌘Q` 当彻底退出（和托盘「退出」同一条，连核心停）：推荐这样，一个「退出」只有一个意思。
- 外壳自己的字照系统语言，不问核心的 `ui.language`：外壳不连核心，字就两三个词。

### 不做什么

- 不改协议：桌面窗口的 `head.kind` 照旧 `web`。
- 不写代码；外壳 DT-3 做。

### 验收

1. 三个仓库的图纸说的是同一件事：桌面窗口开 `miyu web --print --format json --ticket` 的网址、页面照 `served.js`、外壳不连核心。
2. 核心、网页的改动由那边的会话看过、合进各自的 main。

### 风险

- macOS 的 WKWebView 没有 `Notification`：图纸里先写「待定」（网页的会话 2026-10-30 定，macOS、Windows 都等 DT-3 实测了再写），可能要网页的 `served.js` 补一种办法。
