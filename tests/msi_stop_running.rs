//! Windows 安装包装、卸、修之前停掉在跑的（`packaging/windows/miyu.wxs` 的 `MiyuStopRunning`，审计 #081）：照 Windows Installer 的
//! 样子把那一步的命令行拼出来（填上 `[INSTALLDIR]`、`[System64Folder]`，`Directory` 写了的在那个目录里起），真用系统的 PowerShell 跑一遍。
//! 装的目录名带单引号和方括号；托盘程序也要停；不在这个目录里的同名程序不碰。

#![cfg(windows)]

use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::{Duration, Instant};

const WXS: &str = include_str!("../packaging/windows/miyu.wxs");
const SYSTEM64: &str = r"C:\Windows\System32\";
/// 替身程序照它决定睡不睡（别的时候这个测试什么都不做）。
const SLEEPER: &str = "MIYU_TEST_SLEEPER";

/// 替身：被拷成 `miyu-applet.exe`、`miyu-web.exe` 起来，睡着等人停。
#[test]
#[ignore = "只给停进程的测试当替身"]
fn sleeper() {
    if std::env::var_os(SLEEPER).is_some() {
        std::thread::sleep(Duration::from_secs(120));
    }
}

/// `<名字 ...>` 这个元素（`Id` 是 `id` 的）里 `attr` 的值，XML 转义解开。
fn attribute(element: &str, id: &str, attr: &str) -> Option<String> {
    let open = format!("<{element} Id=\"{id}\"");
    let start = WXS.find(&open)?;
    let end = start + WXS[start..].find("/>")?;
    let tag = &WXS[start..end];
    let key = format!(" {attr}=\"");
    let from = tag.find(&key)? + key.len();
    let value = &tag[from..from + tag[from..].find('"')?];
    Some(
        value
            .replace("&quot;", "\"")
            .replace("&apos;", "'")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&amp;", "&"),
    )
}

/// 照 Windows Installer 的格式化填属性：目录带结尾的反斜杠。别的方括号、花括号它也会当属性、分组去填，这里不认。
fn format(text: &str, installdir: &Path) -> String {
    let rest = text
        .replace("[INSTALLDIR]", "")
        .replace("[System64Folder]", "");
    assert!(
        !rest.contains(['[', ']', '{', '}']),
        "Windows Installer 会把方括号、花括号当属性去填：{text}"
    );
    text.replace("[INSTALLDIR]", &format!("{}\\", installdir.display()))
        .replace("[System64Folder]", SYSTEM64)
}

/// 照那一步的写法起它：`Property` 写了的是那个属性里的程序、`ExeCommand` 当参数（类型 50）；`Directory` 写了的 `ExeCommand` 是整条
/// 命令行、在那个目录里起（类型 34）。
fn stop_running(installdir: &Path) -> Child {
    let command = format(
        &attribute("CustomAction", "MiyuStopRunning", "ExeCommand").expect("有 ExeCommand"),
        installdir,
    );
    if let Some(property) = attribute("CustomAction", "MiyuStopRunning", "Property") {
        let program = format(
            &attribute("SetProperty", &property, "Value").expect("属性有人设"),
            installdir,
        );
        return Command::new(program)
            .raw_arg(command)
            .current_dir(std::env::temp_dir())
            .spawn()
            .expect("起得来 PowerShell");
    }
    assert_eq!(
        attribute("CustomAction", "MiyuStopRunning", "Directory").as_deref(),
        Some("INSTALLDIR"),
        "在装的目录里起"
    );
    let rest = command.strip_prefix('"').expect("程序带引号");
    let (program, args) = rest.split_once('"').expect("引号成对");
    Command::new(program)
        .raw_arg(args.trim_start())
        .current_dir(installdir)
        .spawn()
        .expect("起得来 PowerShell")
}

/// 把这个测试程序拷成 `dir\name` 起来当替身。
fn fake(dir: &Path, name: &str) -> Child {
    std::fs::create_dir_all(dir).expect("建得了目录");
    let exe = dir.join(name);
    std::fs::copy(std::env::current_exe().expect("找得到自己"), &exe).expect("拷得了");
    Command::new(&exe)
        .args(["--ignored", "--exact", "sleeper"])
        .env(SLEEPER, "1")
        .spawn()
        .expect("起得来替身")
}

/// 等它在 `wait` 以内退出：退了交回 `true`。
fn exits_within(child: &mut Child, wait: Duration) -> bool {
    let start = Instant::now();
    while start.elapsed() < wait {
        if child.try_wait().expect("问得了").is_some() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    false
}

#[test]
fn the_tray_and_the_rest_in_the_install_dir_are_stopped() {
    let root: PathBuf = std::env::temp_dir().join(format!("miyu-msi-stop-{}", std::process::id()));
    let installdir = root.join("Bob's [1] Miyu");
    let mut tray = fake(&installdir, "miyu-applet.exe");
    let mut web = fake(&installdir, "miyu-web.exe");
    let mut elsewhere = fake(&root.join("other"), "miyu-applet.exe");
    // 替身都起来了再停（PowerShell 冷启动以前它们早就在了，这里只是保险）
    std::thread::sleep(Duration::from_secs(1));
    let mut stop = stop_running(&installdir);
    assert!(
        exits_within(&mut stop, Duration::from_secs(90)),
        "停进程的那一步跑得完"
    );
    let tray_stopped = exits_within(&mut tray, Duration::from_secs(10));
    let web_stopped = exits_within(&mut web, Duration::from_secs(10));
    let elsewhere_kept = elsewhere.try_wait().expect("问得了").is_none();
    for child in [&mut tray, &mut web, &mut elsewhere] {
        if child.try_wait().expect("问得了").is_none() {
            child.kill().expect("杀得掉");
            child.wait().expect("收得回");
        }
    }
    std::fs::remove_dir_all(&root).expect("删得掉");
    assert!(tray_stopped, "装的目录里的托盘程序停掉");
    assert!(web_stopped, "装的目录里的网页软件停掉");
    assert!(elsewhere_kept, "别的目录里的同名程序不碰");
}
