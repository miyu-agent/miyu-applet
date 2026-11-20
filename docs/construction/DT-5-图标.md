## DT-5：图标

状态：已完成（2026-10-30）。Linux 上托盘看过；macOS、Windows 的图标等 DT-6 出安装包看。

### 目的

DT-3 用原图直接缩的占位图标：原图是不透明的位图，托盘里是一块白方块，小尺寸糊成一团。这一步从项目主人画的黑白剪影（`icons/source/logo.png`，1254px）出应用图标、托盘图标，用一个脚本出，换图只要重跑。

### 蓝图改哪几节

- `docs/blueprint/desktop.md`「在哪」的 `icons/` 一行、「怎么走」第 11 条托盘图标。

### 要拍板的

- 小图标原来打算「只留头和大蝴蝶结」（裁一块）：试了 16 到 48px，裁出来的反而认不出（只剩几道黑白斜条），整张圆形徽章倒还认得。改成：32px 以下用先糊再描的简化版（细发丝没了，圆、脸、蝴蝶结的大形状留着），48px 以上用原样描的。项目主人要自己另画小图标的，换 `icons/source/` 里的稿子重跑。
- 托盘：Linux、Windows 用带浅色底板的小图标（任务栏深浅都有，单色的黑在深色任务栏上看不见）；macOS 用模板图（黑色加透明，系统照菜单栏着色）。

### 不做什么

- macOS 的 `.icns`：Tauri 打包时从 png 出（DT-6）。
- 深色模式另一套、动起来的托盘图标。

### 验收

1. `icons/source/build.sh` 从 `logo.png` 出全部图标：`miyu.svg`、`miyu-simple.svg` 两份矢量稿，`icon.png`（512）、`32x32.png`、`128x128.png`、`256x256.png`、`icon.ico`（16、24、32、48、64、256）、`tray.png`、`tray-template.png`，都是 RGBA（Tauri 只认这个）。
2. 一张对照图看过每个尺寸，深、浅任务栏上的托盘图标都看得见。
3. 本机 Linux 托盘上看到新图标；三平台 CI 编译过（Windows 把 `icon.ico` 编进程序）。

### 风险

- 底板的浅色在浅色的程序坞、开始菜单里不显眼：真机上看了再调 `build.sh` 里的 `plate`。

### 结果

- `icons/source/build.sh` 出了全部图标，都是 RGBA；对照图看过 512、128、48、32、24、16 和深浅任务栏上的托盘。
- 本机 Linux：托盘图标的文件（`$XDG_RUNTIME_DIR/tray-icon/` 下 tray-icon 写的那张）和 `icons/tray.png` 逐像素一样；clippy（Linux、Windows 目标）过。
- 顺带复测了网页的端口回退（W-987，网页 e072dc8d）：空数据根、不写 `web.port`，这台机器 8300、8301、8302 都被占着，外壳 259 毫秒拿到 `http://127.0.0.1:8303/` 的网址，页面到欢迎屏；托盘「退出」核心照旧停（`stopped reason=request`）。DT-3「碰到的」第一条网页那边解决了。
