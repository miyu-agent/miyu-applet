## Miyu

开箱即用的 Miyu：装上就能用，不用开终端。装好以后从应用菜单打开「Miyu」，它用浏览器打开 Miyu 的网页界面，自己留在托盘里（macOS 上是菜单栏）：看 Miyu 在不在跑，停止、启动、重新启动，打开网页界面和设置，开机时启动，打开日志的文件夹；浏览器关了也弹「回答完了」「等你确认」这些系统通知，点了打开那个会话；「退出 Miyu」连核心一起关掉。

- 这个仓库是托盘程序 `miyu-applet`：没有自己的窗口，界面都在网页里。网页在 [miyu-agent/miyu-web](https://github.com/miyu-agent/miyu-web)，核心在 [miyu-agent/miyu](https://github.com/miyu-agent/miyu)；安装包把三样打在一起。
- 托盘用 [`tray-icon`](https://github.com/tauri-apps/tray-icon) 和 [`tao`](https://github.com/tauri-apps/tao)（Tauri 自己的托盘和事件循环），三个平台一份代码，不要网页视图。
- 支持 Linux、macOS（M 系列）、Windows。Linux 上要 GTK 3，托盘走 StatusNotifierItem（KDE、waybar 这类自带；GNOME 要装 AppIndicator 扩展才显示托盘）。
- 终端界面不在这里，另装 [miyu-agent/miyu-tui](https://github.com/miyu-agent/miyu-tui)。

怎么装（Linux 的 deb、rpm，macOS 的 dmg，Windows 的 msi，还有 AUR、Homebrew）见 [安装](https://github.com/miyu-agent/miyu/blob/main/docs/guide/install.md)；菜单、通知、开机时启动、局域网访问怎么用见 [托盘](https://github.com/miyu-agent/miyu/blob/main/docs/guide/heads/applet.md)。

改这个仓库的代码先读 [`docs/blueprint/app.md`](docs/blueprint/app.md)。

### 许可证

AGPL-3.0-or-later，见 [`LICENSE`](LICENSE)。菜单里的图标来自 [Lucide](https://lucide.dev)（ISC），见 [`icons/source/lucide/LICENSE`](icons/source/lucide/LICENSE)。

编进托盘的第三方包和资源，许可证原文都在 [`THIRD-PARTY-LICENSES.txt`](THIRD-PARTY-LICENSES.txt)（`cargo xtask notices` 生成，CI 的 `cargo xtask licenses` 查依赖的许可证能不能和 AGPL 合在一起发、这份是不是最新的）。安装包里托盘、核心、网页各带一份，在资源目录的 `licenses/` 下。
