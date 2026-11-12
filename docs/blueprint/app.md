## Miyu 应用（miyu-applet）

状态：草案（施工 AP-1，2026-11-03；AP-2 托盘骨架、AP-3 托盘补全、AP-4 系统通知、AP-5 安装包收尾照实际做法改了几处，下面标了「AP-2」「AP-3」「AP-4」「AP-5」）。代替原来的桌面端外壳（`desktop.md`，Tauri 窗口，DT-1 到 DT-12）。

### 为什么换

2026-11-03 项目主人问「我们是否真的有必要做桌面版」，定：不做自己的窗口，照 oMLX 的样子做一个托盘程序加浏览器。

- 开箱即用照旧：装上 → 从应用菜单打开 → 自动用浏览器打开网页 → 开始用。和窗口版只差打开的是浏览器。
- 网页改了刷新就生效，不用重新出安装包；不再适配三种网页视图（WebKitGTK、WKWebView、WebView2），网页只照浏览器做。
- 局域网访问、im 这些要在后台一直跑的，本来就要一个常驻的图标表示「Miyu 在后台跑着」，托盘正好是它。
- 名字（同一天项目主人）：`-desktop` 是桌面版的意思，不合适，仓库、程序叫 `miyu-app`（2026-11-05 项目主人改名 `miyu-applet`：Linux 上托盘程序惯例叫 applet，如 nm-applet、blueman-applet；仓库 `miyu-agent/miyu-applet`，Windows AppID、macOS bundle id 是 `io.github.miyu-agent.miyu-applet`；还没发过正式版，不留清理旧名的代码）；给人看的地方（应用菜单、程序坞、任务栏、安装包、托盘提示）一律写「Miyu」，同 oMLX 只露一个名字。
- 谁想做自己的桌面版、手机 App，照网页和协议自己做，不归这里。

照 oMLX（README：原生菜单栏程序，只管起停、状态、崩溃了重启、更新、用量；界面都在服务给的网页里）：托盘不放界面，界面全在网页里。

### 它是什么

一个常驻的小程序 `miyu-applet`：只有托盘图标（macOS 上是菜单栏图标），没有窗口。它不连核心以外的东西，调装在旁边的主程序 `miyu`，另照网页的做法连核心，看状态、收提醒（第 4、5 条）。

1. **从应用菜单打开**：起 `miyu-applet`。已经有一个在跑的（单实例），交给它，它照第 2 条再开一次浏览器，自己退。
   - 单实例按数据根算（AP-2）：`MIYU_HOME` 指到别处的托盘各跑各的。托盘自己的目录（Windows `%LOCALAPPDATA%\miyu-applet`，macOS `~/Library/Application Support/miyu-applet`，别的 `$XDG_STATE_HOME/miyu-applet`，没设的 `~/.local/state/miyu-applet`；只给自己看）里一把系统的文件锁、一个「请打开」的文件：拿到锁的盯着那个文件，拿不到的往里写一行就退。不放进数据根：数据根第一次由核心建，没建之前往里写东西核心就认不出它了。不用本机套接字：Linux 的抽象套接字谁都连得上、谁都能抢先占名字。
   - 开机拉起来的（带 `--autostart`）碰到已经在跑的，什么都不做就退。macOS 上点程序坞、再开一次（系统不起第二个，发「重新打开」）同样打开网页界面。
2. **打开网页界面**：跑 `miyu web`（核心、网页服务没在跑的由它拉起；带交接票，经跳转页交给系统的浏览器，网页 `web-ui.md`「二、miyu web」）。起来那一下开一次；托盘菜单「打开网页界面」、点托盘图标（Windows、Linux 左键）再开。浏览器装成了应用（网页的应用清单，见「网页要配的」）的，系统照它开成独立窗口。
3. **点托盘图标**：Windows、Linux 左键直接「打开网页界面」，右键出菜单；macOS 照菜单栏的习惯，点一下就出菜单。
   - Linux 照 StatusNotifierItem 走（2026-11-04 项目主人定换，AP-3 做）：AP-2 用的 `tray-icon` 默认 AppIndicator 后端不给程序图标的点击，左键、右键都是出菜单；换成 `tray-icon` 0.26 的 `ksni` 后端，直接在会话的 D-Bus 上说 StatusNotifierItem：左键是托盘宿主调的 `Activate`，托盘照它打开网页界面；右键宿主照 dbusmenu 自己画菜单。不再要 libayatana-appindicator，多一套纯 Rust 的 zbus；GTK 照旧要（事件循环 `tao`、消息框 `rfd`）。没有 StatusNotifierItem 宿主的托盘（只认 XEmbed 的老托盘、没装 AppIndicator 扩展的 GNOME）显示不出图标，AppIndicator 原来在这种地方退回 GTK 的老图标，现在没有了。中键（宿主调 `SecondaryActivate`）不做事。
4. **托盘菜单**（照 oMLX 菜单栏的那一份：头一行状态，下面按用途分组，组之间一根分隔线；只放人会来点的东西，别的都在网页里。2026-11-03 项目主人看过可交互的样板定：https://claude.ai/artifact/WWVU3J6jpQeBryGB84ZJqH（样板页 2026-11-04 已删，菜单照下面写的））。从上往下：
   - **状态行**（灰字、前面一个小圆点，不能点）：
     - 「运行中」（绿点），局域网开着的写「运行中 · 局域网」；
     - 「正在启动…」「正在停止…」（黄点）、「已停止」（灰点）、「没有响应」（黄点，连得上、问不出话）；
     - 「出错了」（红点），下面一行小字写原因，照 `miyu` 的那一句，例如「这是旧版 Miyu 的数据根，新版不认」。
       - 原因照托盘的语言写（核心 LC-1，61f1f3ef2 起）：数据根认不出的三种带代码（`data_root_not_ours`、`data_root_old_layout`、`data_root_other_layout`，`PrepareError::code`、`StartError::Refused` 的 `code`），照核心资源目录里的说法 `core/start/<代码>` 写，`{path}` 填数据根；没代码的（读写出错、老核心）、资源目录里没有那一句的照核心的原话（`src/reason.rs`）。原来一律是原话，原话写死中文（AP-3 测出来的）。
     - 不写「Miyu：」，菜单本来就是 Miyu 的（同一天项目主人）。
     - 状态照托盘自己连核心的那条连接来（同第 5 条，连上就是运行中，断了就是已停止），不定时去问。
     - 「正在启动…」「正在停止…」是托盘自己在起停时写的，起停做完了再照连接写；拉起来成了、连接还没报的接着写「正在启动…」，不闪「已停止」（AP-2）。
     - 「没有响应」（AP-3）：托盘那条连接上问出去的（握手、订阅、读配置）过了 10 秒还没回就写它，接着等，回了照旧写「运行中」；连接断了照断了写。连着、托盘没在问什么的时候核心卡住了，托盘不知道：不为这个定时去问，下一次托盘问它（推来的变动引起的再读）时才看得出。
     - 「出错了」（AP-3）：断着的时候，最近一次出了错的写它，下面一行写原因：拉不起核心（「启动」「重新启动」：核心说了起不来的原因的照它的原话；数据根认不出、建不成的照 `miyu-client` 的那一句）；`miyu web` 没成（打开网页界面、设置、软件包的入口，照它标准错误的最后几行）；握手被核心拒了（本机令牌不对、协议版本谈不拢，照核心的原话）；起来时就找不到数据根、找不到主程序。连上了、再点「启动」时放下。连着的时候出的错不盖住「运行中」（照旧弹消息框）。
     - 圆点是现画的一张小图，放在图标那一列（AP-2）；「出错了」的原因同一项里另起一行（托盘菜单不给一项两种字号）。
   - **起停**，紧接在状态行下面（同一天项目主人：「停止」放状态行下面，「重新启动」单独一行放在它下面）：
     - 第一行「停止」（方块图标）：运行中、没有响应时；停着、出错了的时候这一行换成「启动」（播放图标）；正在启动、正在停止时灰着。
     - 第二行「重新启动」（转圈箭头图标）：一直在，运行中、没有响应、出错了时能点，已停止、正在启动、正在停止时灰着，不藏，免得菜单的行跟着状态跳。
     - 停止照原来外壳的做法 `miyu core stop`，网页服务、im 这些跟着停；托盘不退，状态行写「已停止」。重新启动是先停再拉起来。
       - 网页服务不是核心拉起来的，不跟着核心停：开着的页面一重连，它照 `connect_or_start` 一秒内又把核心拉起来（2026-11-03 在 Windows 虚拟机上测出来，核心的父进程是网页服务）。所以「停止」先 `miyu web --stop`（网页 W-1091，不连核心，等它放锁最多 10 秒），再 `miyu core stop`；停没停成都接着停核心，只记日志（旧网页不认这个参数、退出码 2 的，留给它空闲了退）。
       - 「重新启动」只停核心、不停网页服务：页面接着连新起来的核心，不用重开。停核心和再拉起来之间网页服务先把核心拉起来了的，托盘的「启动」照 `connect_or_start` 连上它就是。
     - 核心不是托盘叫停的、自己没了（崩了），状态行照实写，再弹一条系统通知「Miyu 意外停止了」，点了照「启动」做。托盘不自己悄悄拉起来：会话、网页、终端哪个头再连都会把核心拉起来（`connect_or_start`），用不着托盘另守着。AP-4 还没做这一条：托盘分不清核心是崩了，还是别的头（终端的 `miyu core stop`、网页）叫停的，照「断了」弹会弹错；要核心在停下前说一声为什么（或者给退出原因），再做。
     - 图标照网页用的 Lucide（`square`、`play`、`rotate-cw`、`power`），线条、单色，画在打勾那一列。
   - **打开**：
     - 「打开网页界面」（默认项，加粗；同第 2 条。同一天项目主人：不叫「打开 Miyu」）；托盘库（`muda`）不给菜单项加粗、不给设默认项，AP-2 只放在这一组第一行；
     - 「设置…」（浏览器打开网页的设置；macOS 带 ⌘,）：`miyu web --settings`（网页 W-1085）；旧网页不认这个参数、退出码 2（用法不对）的，记一行日志，退回 `miyu web`（AP-2）。
     - 没运行的时候点这两样，先把核心拉起来再开。
   - **软件包挂的入口**（同一天项目主人：别的程序要能往托盘菜单挂东西，例如 im 挂一个「打开通讯平台接入界面」）：
     - 装着、开着的包在清单里写了托盘入口的，一个包一行，名字照清单写（分语言），点了用浏览器打开那个包的后台页（`miyu pkg page <编号>`）；没运行的先拉起核心。
     - 打开走核心的 `miyu pkg page`（AP-18，发版前 check 第 14 条，核心 BP-2）：自己起服务的包（清单 `[page] server = true`）直接开它的服务，后台页在包目录里的开核心按需起的本机服务，都不用装网页；旧核心不认这个命令（退出码 2）的退回 `miyu web`。
     - 只能是「打开它的后台页」这一种，不挂任意命令：托盘不当启动器，也不开口子让包在这台电脑上跑别的东西。
     - 一个都没有的不出这一组。清单的写法要核心定（见「要核心给的」）。
     - 怎么跟（AP-3）：托盘那条连接握手以后订 `packages` 流，回应是整份软件包列表，之后推 `package.changed`（装、卸、开关、状态变了都推），掉了队（`resync`）的重新订；不定时去问，也不在菜单打开前去问。核心断着的时候留着最后知道的那几行（点了照样 `miyu pkg page`，它会拉起核心）。
     - 哪些算开着（AP-3）：带 `tray` 的，`enabled` 不是假的、不是卸掉了的出厂包，`status` 不是 `off`、`program_missing`、`depends_missing`；没有开关的（内置包）算开着。扩展崩了、在退避的（`stopped`、`starting`）照列：后台页由网页软件给，不靠扩展在跑。照编号排（同 `package.list`）。
     - 列表变了把整个菜单换一份（托盘库不给在中间插一组）。
   - **局域网**：
     - 「局域网访问」：打勾的开关，照配置 `web.listen`（`0.0.0.0` 是开）。打开前没设访问密码的，不开，改成用浏览器打开网页设置「通用」的「访问」一组，先设密码。
     - 开着时下面多一行「在手机上打开…」：用浏览器打开网页设置里显示局域网地址和二维码的那一组（网页 W-1086 做二维码），手机扫了进登录页、输访问密码。托盘不自己画窗口（2026-11-03 定，AP-2 改进图纸；这一组 AP-3 做）。
     - 实际做法（AP-3）：
       - 开没开照网页设置页的认法（网页 `account.js` 的 `loopbackOnly`）：回环（`127.*`、`::1`、`localhost`、空的）是关，别的（`0.0.0.0`、这台电脑的某一个地址）是开。打开写 `0.0.0.0`，关掉恢复默认（`unset`，出厂的 `127.0.0.1`），同网页设置页；网页软件自己跟着配置换监听（网页 `follow.rs`）。
       - 读：托盘那条连接订 `config` 流，连上时读一遍 `web.listen`（`config.get`），之后推来的 `config.changed` 里有它的、装上卸掉一个包的（这时核心不推 `config.changed`，网页软件的配置项跟着包有没有）再读一遍。核心不认这一项（网页软件没装）、还没连上过的不出这一组；断了留着最后知道的、灰着。
       - 写：点了临时连一次核心（不拉起），`config.set`；只在「运行中」能点（没在跑、没有响应、正在起停的灰着）。点了托盘库自己先翻了勾，托盘照知道的翻回来，等推来的变动再改。
       - 设没设访问密码：打开前现连现问，看握手回应的 `password_set`（核心参考 `connection.md`「hello」：头照它决定「允许局域网访问」开不开得了）；设过的才写，没设的、老核心不说的不写，改成 `miyu web --settings access`（网页 W-1085：开「通用」页、滚到「访问」那一组）。密码是在网页里设的，托盘收不到推送，所以不记着、每次点都问。
       - 「在手机上打开…」：`miyu web --settings access`（局域网地址、二维码在那一组）。
   - **这台电脑**：
     - 「开机时启动」：打勾的开关（Windows 照当前用户的 `Run` 注册表项，值名 `Miyu`；macOS 照登录项 `SMAppService`；Linux 照 `~/.config/autostart/miyu-applet.desktop`，照 `XDG_CONFIG_HOME`）。开机起来的带 `--autostart`，不开浏览器，只常驻。托盘是照 `MIYU_HOME` 指的数据根起来的（不在默认的 `~/.miyu`），打开开关时把它一起写进去，开机照同一个数据根起：三个平台的启动项都在程序后面写 `--home <数据根>`（同点了通知照网址拉起时那一个参数），Windows 的 `Run` 项也带得上（施工 AP-14、AP-17，2026-11-07 项目主人装来试用时，数据根在 `~/.cache/miyu-tui-home`，开机会连到旧版的 `~/.miyu`；AP-14 先用环境变量做了 Linux、macOS，AP-17 换成参数，三个平台一个写法）。托盘自己是照 `--home` 或者 `MIYU_HOME` 起来的才写，默认的不写。
       - macOS 实际用的是用户的 LaunchAgent（`~/Library/LaunchAgents/io.github.miyu-agent.miyu-applet.plist`，`RunAtLoad`，只在图形登录时起；AP-2）：`SMAppService` 要调 Objective-C 的接口（仓库 `forbid(unsafe_code)`），还要装成签过名的 `.app` 才认。系统设置的「登录项」里一样看得到、关得掉。
       - 打勾照现在真的开着没有：点了照反过来设，设完再读一遍；设不成的弹一句。Linux 上文件里写了 `Hidden=true`（桌面环境的设置里关掉的）算关着。
     - 「打开日志所在的文件夹」：核心、网页、托盘的日志都在那里（数据根的 `var/log/`），出了问题照它查。还没有的（核心没起过）弹一句「还没有日志」，不替核心建。
   - **关于、退出**：
     - 「关于 Miyu」：一个小窗口，写托盘、核心、网页三样的版本和钉的提交号，能复制。AP-2 用系统原生的消息框（`rfd`）：托盘是编的版本，核心照 `miyu --version`，网页照核心软件包列表里 `webui` 那一项的版本（2026-11-12 起包编号不带 `miyu-`，原来是 `miyu-webui`）（连着核心时记下的，就是核心照层挑好、真在用的那一份：出厂的、系统源加装的、装上的；2026-11-05 改，核心设计 60：分包以后核心的资源目录挪到 `/usr/lib/miyu/`，照资源目录读不到系统源装的网页），没连上过核心的退回资源目录里网页包清单的 `version`（打包时写进去的），提交号是编进程序的 `pins.toml`。Windows 的消息框按 Ctrl+C 整段复制；macOS、Linux 的消息框不一定选得中字，要真能复制得换成自己的小窗口或者交给网页。第一版不做自动更新（原来外壳第一节第 6 条）；以后做的话照 oMLX，有新版时这里多一行「安装更新…」。
     - 「退出 Miyu」（电源图标；macOS 带 ⌘Q）：先照「停止」把网页服务、核心这些停掉，托盘再退。没停成的照退，日志记一行。点了先拿掉托盘图标（停核心最多等一分钟）。
     - 「启动」（AP-2）：照头的办法拉起核心：建好数据根的骨架（`DataRoot::prepare`，第一次用时写标记），`connect_or_start` 拉起 `miyu core`，等它说好了就放掉那条连接；不开浏览器、不起网页服务（`miyu core` 就是核心进程本身，「由头拉起」）。没用 `miyu web --print`：它连网页服务一起起，网页包不在时还会报「未知命令」。
5. **系统通知**：浏览器关了也要收得到「回答完了」「等你回答」「等你确认」「定时任务跑完了」。托盘照终端的样子连核心（出示本机令牌，同网页服务读配置那样），订会话的状态和账号级的提醒（网页 `web.md`「系统通知」第 1、6 条同一套），只发「没有网页开着这个会话」（会话表那一项 `viewed` 不为真）的那些；标题、正文、`tag` 同网页第 2 条，图标用 Miyu 的 logo；点了用浏览器打开那个会话（`miyu web` 带上会话，见「要核心给的」）。实际做法（AP-4）：
   - 和网页的分工（核心 TR-1，网页 W-1088）：托盘那条连接握手以后调 `notices.claim`，认领「后台通知由托盘发」；认领着的时候网页只为它这个标签页正开着的会话弹，没人开着的会话、账号级的提醒都归托盘。认领跟着连接：断了就作废，重连时再认领一次（每连一次从头来）。核心不认 `notices.claim`（老核心）的托盘不发：这时网页照原来弹，托盘再弹就重了。
   - 订什么：认领成了再订 `sessions`（回应是整张会话表，之后推 `sessions.changed`，整项换）、`notices`；掉了队（`resync`）的重新订，会话表整份换上、掉队那一段不补。不定时去问。
   - 什么时候：会话表推来的一项和手上的比，`busy` 去掉了是「回答完了」，`waiting` 冒出来是在等人；订阅时、重连时换上的整份不算（不把旧的再报一遍）。会话表不分在问还是要确认：`waiting` 冒出来时读那个会话最新一轮的事件（`view.page {turns: 1}`），最后一个还没了结的 `question.asked` 是「等你回答」、`tool.approval_requested` 是「等你确认」（了结照网页 `asking` 包的认法），看不出的算「等你确认」。`viewed` 为真的、子会话、一次性的会话、整理记忆的会话不发。提醒：`schedule.ran` 照 `ok` 写「定时任务跑完了」「定时任务没跑完」（那个会话有网页开着的不发，由它弹「回答完了」）；`schedules.reset` 标题「Miyu」、正文同网页那一句，点了打开网页界面。
   - 写什么：同网页第 2、6 条：标题是会话的标题，没有的等核心起标题最多 5 秒，没等到照第一句（`preview`），定时任务的照那句话，再没有写「Miyu」；正文只写为什么，三种语言（`resources/text/` 的 `notify.*`，字同网页 `notify` 包）；`tag` 是 `<会话>:done`、`<会话>:ask`、`<会话>:approve`，定时任务跑完了同 `<会话>:done`，记录坏了是 `schedules.reset`。托盘发的都是没人开着的会话，不分出不出声，照系统默认。图标：Linux 用应用图标（写进托盘自己的目录给通知服务读），Windows、macOS 照应用身份用安装包的图标；不用会话人格的头像。
     - 照会话的人格来（AP-20，2026-11-10 项目主人：系统通知照会话的人格来；同网页 W-1143、终端、shellhook）：标题写这个会话人格的名字，不写会话标题；人格有头像的带头像，没头像的不带图标；无人格的会话、账号级的提醒（唤醒表坏了）标题写「Miyu」、不带图标，原来带的托盘 logo（`notification.png`）不再带。会话的标题挪进正文、接在为什么前面（「修 CI · 回答完了」），没标题的照第一句、定时任务那句话，再没有的只写为什么。人格照会话表那一项的 `persona`；报之前问 `persona.get`（名字，没名字的照编号；头像的版本），有头像的再读 `persona.avatar`（`{avatar, media_type, data}`，base64），同一个人格同一个版本记着不再读；问不到的（老核心、人格删了，`soft` 记一行）照无人格的写。头像写进托盘自己目录的 `avatars/<版本>.<扩展名>`（同样的不再写）：Linux 走 `image-path` 提示；macOS 走 notify-rust 的 `image_path`（`content_image`，横幅右边一张图，左边的应用图标换不了）；Windows 放在 toast 的 `appLogoOverride`（圆形裁剪，`file:///` 地址）。上面「标题是会话的标题……再没有写「Miyu」」「图标用 Miyu 的 logo」照这一条。
   - 点了：会话的 `miyu web --session <编号>`（网页 W-1085），账号级的 `miyu web`；旧网页不认参数（退出码 2）的退回 `miyu web`。通知上另有一个「打开」按钮，做的一样（macOS 要有按钮库才等人点）。
   - 用 `notify-rust` 4.18.2（钉死版本），三个平台做得到的不一样：
     - Linux：会话 D-Bus 的 `org.freedesktop.Notifications`（zbus，同 ksni 那一套），带 `desktop-entry: miyu-applet`；同一个 `tag` 的带上一条的编号发（`replaces_id`）换掉它；点了是通知服务发回的 `ActionInvoked`（默认动作 `default`）。没有通知服务的桌面不弹，只记日志。
     - Windows：WinRT 的 toast，照应用身份（AUMID）`io.github.miyu-agent.miyu-applet` 发，就是 MSI 开始菜单快捷方式上的 `System.AppUserModel.ID`（Tauri 打包工具 2.10.4 的模板自带，值是 `tauri.conf.json` 的 `identifier`）；系统照这个快捷方式写「Miyu」、画图标，没装（直接跑编出来的程序）的不一定弹得出来。toast 自己拼（`src/toast/winrt.rs`，`windows` 这个 crate 的 WinRT 接口，安全的 Rust），点了照网址拉起托盘（`src/protocol.rs`）。2026-11-04 Windows 虚拟机上测出：notify-rust 底下 tauri-winrt-notification 拼的 toast 只有「打开」按钮在横幅上时回得来，点正文、收进通知中心以后再点都回不来；改成在进程里接 `Activated` 也只管横幅，通知中心里点 Windows 只交给登记过的 COM 激活器（要 `unsafe`，不做）。现在：
       - 正文、「打开」按钮都是 `activationType="protocol"`，网址 `miyu-applet://open/?session=<编号>`（账号级的 `miyu-applet://open/`）。托盘每次起来在当前用户的注册表登记 `miyu-applet` 协议（`HKCU\Software\Classes\miyu-applet`，不要管理员），命令是 `"<exe>" --home "<数据根>" --open "%1"`：照网址拉起的程序不带托盘的环境变量（`MIYU_HOME` 另指的数据根会丢），数据根写进命令里，起来照它找、`miyu` 子进程都带上 `MIYU_HOME`。
       - 拉起来的那一个照单实例：有托盘在跑的交「打开那个会话」（`session <编号>`），自己退；没在跑的起来就打开那个会话。横幅上、通知中心里、托盘退了以后点都一样。
       - 同一个 `tag` 的放进同一组（`Tag` 截到 64 个字、`Group` 是 `miyu`），下一条换掉上一条。卸载不删协议那一项（同「开机时启动」的注册表项）。
     - macOS：NSUserNotificationCenter（系统已经标成过时，还能用；新的 UNUserNotificationCenter 要签过名的 `.app`），照 `Miyu.app` 的 bundle id（同上）发，没装成 `.app` 的照库的默认用访达的身份。点了要等：带「打开」按钮的才等人点，点了通知本身或者按钮都打开那个会话；横幅收起以后在通知中心里再点，回不回得来要手测。不换掉上一条。
6. **日志**：数据根 `var/log/` 下 `miyu-applet.log`，和核心、网页的放在一起，照原来外壳 DT-10 的轮转和限流。数据根的 `var/log/` 由核心建：起来时还没有的先写到标准错误上，连上核心以后再定下文件（AP-2）。

### 装在哪

照原来外壳 DT-6、DT-7、DT-9 的打包流水线（`pins.toml` 钉核心、网页，`packaging/stage.py` 搭内容），换掉的只有程序：

| 平台 | 安装包 | 应用菜单里那一项 |
|---|---|---|
| Windows | 64 位 MSI（中、英、日），装进 `Program Files\Miyu\`，带官方 PowerShell 7 的 zip 版（DT-9）、许可证（DT-7） | 开始菜单「Miyu」，指向 `miyu-applet.exe`（图形子系统，不闪命令行窗口） |
| macOS | dmg，`Miyu.app`，`LSUIElement`（不占程序坞，只在菜单栏） | 「应用程序」里的 Miyu |
| Linux | deb、rpm（AUR、Homebrew 的包由核心仓库的 `packaging/` 写，网页 `miyu-webui`、托盘 `miyu-applet` 各自成包，核心设计 60；2026-11-05 起托盘仓库不再带 PKGBUILD） | `miyu.desktop`（`Exec=miyu-applet`） |


上面这几种都是完整的：带核心和网页，装上就能用（核心设计 59），和系统源的 `miyu-core` 写冲突。另出只装托盘的包（施工 AP-11，2026-11-05 core1，给系统源用：AUR 的 `miyu-applet` 依赖 `miyu-core`、`miyu-webui`，Homebrew 的 cask 依赖 formula 的 `miyu-core`、`miyu-webui`；不从源码编），进同一个发版：

- Linux x86_64、aarch64：`miyu-applet-<版本>-linux-<架构>.tar.gz`（`packaging/solo.py`），一层 `miyu-applet-<版本>-linux-<架构>/` 照 `/usr` 摆：`bin/miyu-applet`、`share/applications/miyu-applet.desktop`、`share/icons/hicolor/{32x32,128x128,256x256}/apps/miyu-applet.png`、`share/licenses/miyu-applet/`（托盘自己的 `LICENSE`、第三方许可证清单）。
- macOS：`miyu-applet-<版本>-macos-aarch64.zip`，里面是只装托盘的 `Miyu.app`（`packaging/macos-solo.json`：不带核心的程序和资源，`LSUIElement` 照旧，`Resources/licenses/` 托盘自己的两份）。
- 托盘照 `PATH` 找系统装的 `miyu`；macOS 从访达、开机启动拉起的图形程序 `PATH` 只有 `/usr/bin:/bin:/usr/sbin:/sbin`，`PATH` 里也没有的先看家目录下安装脚本装的 `~/.local/lib/miyu/`、`~/.local/bin/`（AP-19，2026-11-10 项目主人：Mac 上照安装脚本装的，托盘找不到主程序；Linux 也看这两处），再看 `/opt/homebrew/bin`、`/usr/local/bin`（`src/program.rs`）。完整的包里核心就在托盘旁边，先找到它，两种包不打架。
- 发版附件另带 `SHA256SUMS`（全部附件）。和完整的包一样没签名。

安装包里不再有网页视图，Linux 上不再要 WebKitGTK。

实际做法（AP-5）：

- 许可证：托盘的第三方许可证清单 `THIRD-PARTY-LICENSES.txt` 在仓库根上（`cargo xtask notices` 生成，照网页仓库的做法；CI 的 `cargo xtask licenses` 查四个平台的依赖都能和 AGPL-3.0-or-later 合在一起发、清单是最新的）。安装包的资源目录下 `licenses/` 放三份：`miyu-applet/`、`miyu/`（核心钉的那一版的 `LICENSE`、`LICENSES/`、清单）、`miyu-web/`，加一份 `README.txt`；网页包的文件夹照网页 `pack-web.py` 的样子带它自己的 `LICENSE` 和清单。装好以后 Linux 在 `/usr/share/miyu/licenses/`，Windows 在 `resources\licenses\`，macOS 在 `Miyu.app/Contents/Resources/miyu/licenses/`。
- PowerShell 7（只 Windows）：官方 zip 版，版本和两个 SHA256 钉在 `pins.toml` 的 `[pwsh]`（7.6.9，7.6 是 LTS），打包时下载、核对，解进资源目录的 `pwsh\`，核心先找它（核心 SW-5）；MSI 装卸前停进程的名单有它。
- `miyu-applet.exe` 的图标和名字：`build.rs` 编给 Windows 时用 `embed-resource` 把 `icons/icon.ico` 和版本信息（文件说明、产品名称「Miyu」）编进程序，资源管理器、任务管理器里是 Miyu 的图标和名字。release 照旧 `windows_subsystem = "windows"`（`src/main.rs`）。
- macOS：`packaging/macos/Info.plist` 写 `LSUIElement = true`，经 `macos.json` 的 `infoPlist` 并进 `Miyu.app` 的 `Info.plist`；运行时另设 `Accessory`（AP-2）。
- deb、rpm：Tauri 的打包工具总加 WebKitGTK 的依赖，打完由 `packaging/drop-webkit.py` 去掉（只改依赖，文件不动）；GTK 3、bubblewrap 照旧要。
- 打包流水线拆开三个平台的包核对上面这些（`release.yml`「拆开核对」）。

版本号（施工 AP-12，2026-11-06 core1：项目主人定核心、终端、网页、托盘四件统一用「年.月.这个月第几版」，从 0 起、月不补零，预发布加 `-rc.N`，一起发版时同一个号，见核心设计 59「版本号」）：`Cargo.toml` 的 `version` 照这个写（例如 `2026.11.0`、`2026.11.1`、`2026.11.0-rc.1`），标签是 `v` 加上它。几种安装包认不了原样的，打包时由 `packaging/version.py` 换：

| 包 | 写成 | 为什么 |
|---|---|---|
| MSI | 年减 2000 . 月 . 第几版×100 加 rc 号（正式版加 99）：`2026.11.0-rc.1` → `26.11.1`，`2026.11.0` → `26.11.99`，`2026.11.1` → `26.11.199`；经 Tauri 的 `bundle.windows.wix.version` 单给 | MSI 的主版本、次版本最大 255、第三段最大 65535，预发布那段只能是纯数字（tauri-bundler 的 `validate_wix_version`），2026 过不去；MSI 升级只比前三段，这样 rc 升得到正式版、正式版升得到修复版。rc 号最多 98，一个月最多 654 版，年份到 2255 |
| deb | `-` 换成 `~`：`2026.11.0~rc.1` | deb 里 `-` 后面是打包修订号，`2026.11.0-rc.1` 会排在 `2026.11.0` 后面，从 rc 升不到正式版。Tauri 的配置只认 semver、写不了 `~`：打的时候给去掉预发布的版本，打完改 `control` 的 `Version`，文件名换回照 `Cargo.toml` 的版本 |
| rpm | 版本去掉预发布、发布号写 `0.rc.N`：`2026.11.0-0.rc.1`；正式版发布号照旧 `1` | Tauri 用的 rpm 库不收带 `-` 的版本，`~` 又写不进 Tauri 的配置；照 Fedora 原来的预发布写法，`0.rc.1` 排在 `1` 前面 |
| dmg、只装托盘的 tar.gz、zip | 原样 | 不走 App Store、不自动更新，排不排得对用不着（2026-11-06 core1：dmg 先不动） |

不是这种写法的版本都照原样（2026-11-07 定号 `2026.11.0` 以前是 `0.1.0`，AP-15）。AUR 的 `pkgver`（`2026.11.0rc1`）由核心仓库的 `packaging/` 写（核心 VR-1）。试打：手动触发「打包」时填「试打用的版本号」，只在那一次运行里改 `Cargo.toml`、`Cargo.lock`、`tauri.conf.json`，不提交。推标签（施工 AP-13，照核心 RL-2补2）：正式标签 `v<版本>` 要和 `Cargo.toml` 的一样；演练标签 `v<年.月.序号>-rc.N` 照它在 CI 机器上改版本（同试打），不往 main 提交 rc 号；`Cargo.toml` 已经是年.月.序号的，演练标签的前半段也要和它一样。发布一律是预发布、不设成最新的；核心的 `cargo xtask packaging --applet` 照这个发布的 `SHA256SUMS` 和 `miyu-applet-<版本>-linux-<架构>.tar.gz`、`-macos-aarch64.zip` 填 AUR、Homebrew 的托盘包（核心 RL-5补）。

### 网页要配的

- 应用清单（`manifest.webmanifest`、几种尺寸的图标）：Edge、Chrome 里能「安装为应用」，独立窗口、有自己的图标。AP-6 网页做了（网页蓝图 `web.md`「整页」的「装成应用」）：Chromium 实测本机地址（`127.0.0.1`、`localhost`）能装，不要 Service Worker；局域网的 `http://` 地址不是安全来源，装不了；装好的认来源连端口，网页软件换了端口要重装。托盘这边不用改：「打开 Miyu」照旧开浏览器，装过的人从开始菜单、任务栏点它自己的那个。
- 手机上看到的几处先修（网页 W-1082）。Safari 先搁置（同一天项目主人）。

### 要核心给的

core1 2026-11-03 答复：

- 连核心不拉起：已经有了，`miyu-client` 的 `Core::connect_running(root, kind)`（`miyu core stop` 用的那个），托盘只用它，不用会拉起核心的连法。终端、网页服务、`miyu web` 照旧会拉起核心：托盘停着的时候别的头一连，核心就又起来了，这是对的（人在用），托盘照第 4 条的状态行跟着写「运行中」。托盘不在跑的时候怎么知道核心又起来了：看核心的锁文件（`run/core.lock`，系统的文件变动通知，不定时去问），一变就再 `connect_running` 一次。
  - AP-2 实际做法：盯整个 `run/`（不递归），里面有新建、改了、删了的就再连一次。光盯 `run/core.lock` 不够：锁文件本来就有的，核心起来时只是打开、拿锁，不改它，macOS、Windows 上不报；核心起来一定会换 `run/token`、写 `run/socket`。只是打开、读的不算（inotify 会报，托盘自己连核心时读令牌也会报）。`run/` 没有的盯数据根，数据根也没有的盯它的上一级。
  - 连法同 `Core::connect_running`（`miyu-client` 的 `connect::connect`，加同一句 `hello`，`head.kind` 报 `miyu-applet`），只是读的那一半托盘自己拿着、读到头就是断了：`Core` 只能一问一答（`call` 30 秒没回就当没回应），等不了「断了」。网页软件跟着配置的那条连接（网页 `follow.rs`）也是这样写的。以后核心门面给一个「等它断」的口子，换成它。断了马上再连一次：核心崩了、被叫停的连不上，报「已停止」；只是这条连接断了的接着连上。
- 「谁来发后台通知」的登记（第 5 条）：核心 TR-1。连接说一声「我来发」，断开就作废；别的头不用轮询也知道这时有没有人在发。形状（2026-11-03 合进核心 main `d58f7b099`，托盘这一步起钉的就是它）：托盘连上核心以后调 `notices.claim`（参数空，回 `{}`），之后订会话表和 `notices` 流来发（AP-4 做了，见第 5 条）。
- `miyu web` 打开时直接到某个会话、设置页：核心不用动，`miyu web` 后面的参数原样转给网页包的 `open`，参数由网页定（网页 W-1085：`--session <编号>`、`--settings`）。
- 软件包清单里的托盘入口（第 4 条「软件包挂的入口」）：核心 TR-2。清单新加一张表，名字分语言，只能指向本包自己的后台页，指别处报错；`package.list` 照装着的带出来。写法（同上，核心 main `d58f7b099`）：清单写 `[tray] name = {zh, en}`；`package.list` 和 packages 流里，有后台页又写了 `[tray]` 的包多一格 `"tray": {"name": "<照连接的语言挑好的>"}`；托盘照 `enabled`、`status` 只列开着的，点了跑 `miyu pkg page <编号>`（AP-3 做了，AP-18 从 `miyu web --package` 换过来，见第 4 条）。

### 原来外壳的去留

- 留下：托盘、单实例、调主程序（`program.rs`）、叫核心停（`quit.rs`）、日志（`log.rs`）、给人看的字、图标、打包流水线、许可证门禁和清单、随包的 PowerShell 7。
- 去掉：窗口、网页视图、导航和下载交给谁（`navigate.rs`、`download.rs`）、网页视图的通知替身（`notify.rs`、`resources/notification.js`）、macOS 的应用菜单（`menu.rs`）、窗口大小。
- 分支 `step/DT-8-通知`（DT-7、DT-8、DT-9）不照原样合：许可证（DT-7）、PowerShell 7（DT-9）挪进 AP-5；DT-8 的通知是给网页视图的，不要了，系统通知改成第 5 条。
- 程序换框架：只要托盘，不要网页视图，Tauri 换成更小的 `tray-icon` + `tao`（Tauri 自己的托盘就是它们），三个平台一份代码。AP-2 做了：`tao` 0.37.2（关了默认功能）、`tray-icon` 0.26.1（Linux 用 AppIndicator 后端；AP-3 换成 `ksni` 后端，见第 3 条）、`rfd` 0.17.2（消息框，Linux 用 GTK 画）、`notify` 8.2.1、`png`、`open`、`tokio`，`winreg`（只 Windows）；核心照 git 依赖钉 `miyu-client`（核心 main `d58f7b099`，同 `pins.toml` 的核心）。打包还用 Tauri 的打包工具：先 `cargo build --release`，再 `cargo tauri bundle`（`cargo tauri build` 总带 `tauri/custom-protocol` 特性，没有 tauri 依赖编不过），`tauri.conf.json` 只当打包配置；换不换留给 AP-5。AP-5 定不换：除了 deb、rpm 总加 WebKitGTK 的依赖（打完改掉，见「装在哪」），别的（三种语言的 MSI 和开始菜单的应用身份、`.app`、dmg、deb/rpm 的桌面文件和图标）都合用。原来 `tauri-build` 把图标编进 exe，现在是自己的 `build.rs`。
