<p align="center">
  <a href="README.md">English</a> | 简体中文
</p>

# Miyu 系统托盘（miyu-applet）

> <sub>本文档由 Anthropic 的 Claude Fable 5.2 生成。</sub>

Miyu 系统托盘让 Miyu 开箱即用：安装以后就能使用，不需要打开终端。安装完成后，从应用菜单打开 `Miyu`，它会用浏览器打开 Miyu 的网页界面，自己留在系统托盘里（在 macOS 上是菜单栏）。在系统托盘里，你可以查看 Miyu 是否正在运行，停止、启动、重新启动 Miyu，打开网页界面和设置，设置开机时启动，以及打开日志所在的文件夹。即使浏览器已经关闭，它也会弹出 `回答完了`、`等你确认` 这类系统通知，点击通知会打开对应的会话；点击 `退出 Miyu` 会连同核心一起关闭。

- 这个仓库是系统托盘程序 `miyu-applet`：它没有自己的窗口，界面都在网页界面里。网页界面在 [miyu-agent/miyu-web](https://github.com/miyu-agent/miyu-web)，核心在 [miyu-agent/miyu](https://github.com/miyu-agent/miyu)；安装包把这三个部分打包在一起。
- 系统托盘使用 [`tray-icon`](https://github.com/tauri-apps/tray-icon) 和 [`tao`](https://github.com/tauri-apps/tao)（Tauri 自己的托盘和事件循环），三个平台使用同一份代码，不需要网页视图。
- 支持 Linux、macOS（M 系列芯片）、Windows。在 Linux 上需要 GTK 3，托盘使用 StatusNotifierItem 协议（KDE、waybar 这类桌面环境和状态栏自带支持；GNOME 需要安装 AppIndicator 扩展才能显示托盘）。
- 终端界面不在这个仓库里，需要另外安装 [miyu-agent/miyu-tui](https://github.com/miyu-agent/miyu-tui)。

安装方法（Linux 的 deb、rpm，macOS 的 dmg，Windows 的 msi，还有 AUR、Homebrew）见 [安装](https://github.com/miyu-agent/miyu/blob/main/docs/zh/guide/install.md)；菜单、通知、开机时启动、局域网访问的用法见 [系统托盘](https://github.com/miyu-agent/miyu/blob/main/docs/zh/guide/heads/applet.md)。

修改这个仓库的代码之前，先阅读 [`docs/blueprint/app.md`](docs/blueprint/app.md)。

## 许可证

这个程序的许可证是 AGPL-3.0-or-later，详细的见 [`LICENSE`](LICENSE)。菜单里的图标来自 [Lucide](https://lucide.dev)（ISC 许可证），见 [`icons/source/lucide/LICENSE`](icons/source/lucide/LICENSE)。

编译进系统托盘程序的第三方软件包和资源，许可证原文都在 [`THIRD-PARTY-LICENSES.txt`](THIRD-PARTY-LICENSES.txt) 里。这份文件由 `cargo xtask notices` 生成；CI 会运行 `cargo xtask licenses`，检查依赖的许可证能否和 AGPL 合在一起发布，以及这份文件是否是最新的。安装包里的系统托盘、核心、网页界面各带一份许可证原文，放在资源目录的 `licenses/` 下。
