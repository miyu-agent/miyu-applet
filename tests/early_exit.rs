//! `miyu-applet --version`、`--help`（施工 AP-24：rc.5 装机时 `--version` 起了托盘，首次起来还会自己开浏览器）：印完就退，退出码 0，
//! 标准错误上什么都没有；不找数据根、不建托盘自己的目录（单实例的锁在里面）、不起界面。
//!
//! 环境整个清掉，只给一个空的沙箱：家目录、`XDG_STATE_HOME`、`XDG_RUNTIME_DIR`、`LOCALAPPDATA`、`MIYU_HOME` 都指进去，没有显示、
//! 没有会话总线。真起了托盘的话，沙箱里会多出托盘自己的目录（起事件循环之前就建），起不来界面也碰不到这台电脑的桌面。

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

/// 一个空的沙箱目录，用完删掉。
struct Sandbox(PathBuf);

impl Sandbox {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "miyu-applet-early-exit-{name}-{}",
            std::process::id()
        ));
        if dir.exists() {
            std::fs::remove_dir_all(&dir).expect("删得掉上次留下的沙箱");
        }
        std::fs::create_dir_all(&dir).expect("建得了沙箱");
        Self(dir)
    }

    fn entries(&self) -> Vec<String> {
        std::fs::read_dir(&self.0)
            .expect("读得了沙箱")
            .map(|entry| {
                entry
                    .expect("读得了沙箱里的一项")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect()
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        if let Err(error) = std::fs::remove_dir_all(&self.0) {
            eprintln!("could not remove {}: {error}", self.0.display());
        }
    }
}

/// 在沙箱里跑一次，10 秒没退的杀掉、算失败。
fn run(sandbox: &Path, locale: &str, args: &[&str]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_miyu-applet"));
    command
        .args(args)
        .env_clear()
        .env("HOME", sandbox)
        .env("XDG_STATE_HOME", sandbox.join("state"))
        .env("XDG_RUNTIME_DIR", sandbox.join("run"))
        .env("LOCALAPPDATA", sandbox.join("local"))
        .env("MIYU_HOME", sandbox.join("miyu"))
        .env("LC_ALL", locale)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // Windows 上没有它，系统的库加载不起来。
    if let Some(root) = std::env::var_os("SystemRoot") {
        command.env("SystemRoot", root);
    }
    let mut child = command.spawn().expect("起得来 miyu-applet");
    let start = Instant::now();
    while child.try_wait().expect("问得了退没退").is_none() {
        if start.elapsed() > Duration::from_secs(10) {
            child.kill().expect("停得了");
            child.wait().expect("等得到它退");
            panic!("{args:?} 过了 10 秒还没退");
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    child.wait_with_output().expect("拿得到输出")
}

/// 这个系统上托盘照不照 `LC_ALL` 挑语言（`sys-locale`：Unix 照环境变量，macOS、Windows 问系统）。
const LOCALE_FROM_ENV: bool = cfg!(all(unix, not(target_os = "macos")));

/// 三种帮助页的头一截。
const USAGES: [&str; 3] = [
    "Usage: miyu-applet",
    "用法：miyu-applet",
    "使い方：miyu-applet",
];

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[test]
fn the_version_is_printed_and_nothing_starts() {
    let want = format!("miyu-applet {}\n", env!("CARGO_PKG_VERSION"));
    for flag in ["--version", "-V"] {
        let sandbox = Sandbox::new("version");
        let out = run(&sandbox.0, "C", &[flag]);
        assert_eq!(out.status.code(), Some(0), "{flag}: {}", text(&out.stderr));
        assert_eq!(text(&out.stdout), want, "{flag}");
        assert_eq!(text(&out.stderr), "", "{flag}");
        assert!(
            sandbox.entries().is_empty(),
            "{flag}: {:?}",
            sandbox.entries()
        );
    }
}

#[test]
fn the_help_is_printed_in_the_language_of_the_machine_and_nothing_starts() {
    for (flag, locale, usage) in [
        ("--help", "en_US.UTF-8", "Usage: miyu-applet"),
        ("-h", "C", "Usage: miyu-applet"),
        ("--help", "zh_CN.UTF-8", "用法：miyu-applet"),
        ("--help", "ja_JP.UTF-8", "使い方：miyu-applet"),
    ] {
        let sandbox = Sandbox::new("help");
        let out = run(&sandbox.0, locale, &[flag]);
        assert_eq!(out.status.code(), Some(0), "{flag} {locale}");
        let page = text(&out.stdout);
        // 语言照环境变量挑的只有 Linux 这些；macOS、Windows 问系统，照 runner 的语言。
        let picked = match LOCALE_FROM_ENV {
            true => page.starts_with(usage),
            false => USAGES.iter().any(|usage| page.starts_with(usage)),
        };
        assert!(
            picked && page.contains("-V, --version"),
            "{flag} {locale}: {page}"
        );
        assert_eq!(text(&out.stderr), "", "{flag} {locale}");
        assert!(sandbox.entries().is_empty(), "{flag} {locale}");
    }
}

#[test]
fn the_flags_win_over_the_start_arguments() {
    let sandbox = Sandbox::new("mixed");
    let out = run(&sandbox.0, "C", &["--autostart", "--version", "--help"]);
    assert_eq!(out.status.code(), Some(0));
    let page = text(&out.stdout);
    assert!(USAGES.iter().any(|usage| page.starts_with(usage)), "{page}");
    assert!(sandbox.entries().is_empty());
}
