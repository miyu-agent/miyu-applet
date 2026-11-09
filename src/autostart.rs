//! 「开机时启动」（图纸 `app.md` 第 4 条「这台电脑」）：登录时把托盘带一个参数拉起来，只常驻、不开浏览器。
//!
//! - Windows：当前用户的 `Run` 注册表项，值名 `Miyu`。
//! - macOS：用户的 LaunchAgent（`~/Library/LaunchAgents/io.github.miyu-agent.miyu-applet.plist`）。图纸写的是登录项 `SMAppService`：
//!   它要调 Objective-C 的接口（仓库 `forbid(unsafe_code)`），还要装成签过名的 `.app` 才认，第一版先用 LaunchAgent，系统设置的
//!   「登录项」里一样看得到、关得掉（「允许在后台」那一栏）。
//! - Linux：`~/.config/autostart/miyu-applet.desktop`（XDG 自动启动）。
//!
//! 写什么、写在哪是纯的函数，单测钉住；真去读写的那几行按平台分开。
//!
//! 托盘是照明说的数据根起来的（命令行的 `--home <数据根>`，或者 `MIYU_HOME`；施工 AP-14，2026-11-07：装来试用时数据根不在默认的
//! `~/.miyu`），开机也照它起：三个平台的启动项都在程序后面写 `--home <数据根>`（施工 AP-17，同点了通知照网址拉起时那一个参数，
//! `protocol`）。原来 Linux 写 `env MIYU_HOME=…`、macOS 写 `EnvironmentVariables`，Windows 的 `Run` 项带不了环境变量只能照默认；
//! 写参数三个平台一个样子，Windows 也带得上，不用套一层 `cmd /c set` 闪控制台。没明说的照默认，什么都不加。

use crate::protocol::HOME_ARG;
use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};

/// 开机拉起来时带的参数：起来不开浏览器。
pub const ARG: &str = "--autostart";
/// Windows `Run` 项里的值名、macOS LaunchAgent 的标签（同 `tauri.conf.json` 的 identifier）。
#[cfg_attr(not(windows), allow(dead_code, reason = "只有 Windows 用"))]
const NAME: &str = "Miyu";
#[cfg_attr(not(target_os = "macos"), allow(dead_code, reason = "只有 macOS 用"))]
const LABEL: &str = "io.github.miyu-agent.miyu-applet";

/// 开着没有。
pub fn enabled() -> bool {
    sys::enabled()
}

/// 打开、关上。关上时本来就没有的不算错。
///
/// # Errors
/// 找不到自己在哪、家目录在哪，写不了、删不了。
pub fn set(on: bool) -> io::Result<()> {
    if on {
        sys::enable(&std::env::current_exe()?, data_root().as_deref())
    } else {
        sys::disable()
    }
}

/// 托盘照哪个数据根起的：命令行的 `--home`（照网址、开机拉起来的），没有的照 `MIYU_HOME`；是绝对路径的才算，都没有的交 `None`
/// （照默认）。
fn data_root() -> Option<PathBuf> {
    crate::protocol::home_from_args(std::env::args().skip(1))
        .or_else(|| std::env::var_os("MIYU_HOME").map(PathBuf::from))
        .filter(|root| root.is_absolute())
}

/// Linux 上那个文件：`$XDG_CONFIG_HOME/autostart/miyu-applet.desktop`，没设（或者不是绝对路径）的照 `~/.config`。
#[cfg_attr(target_os = "macos", allow(dead_code, reason = "macOS、Windows 不用"))]
#[cfg_attr(windows, allow(dead_code, reason = "macOS、Windows 不用"))]
pub fn desktop_file(config_home: Option<OsString>, home: Option<PathBuf>) -> Option<PathBuf> {
    let config = config_home
        .map(PathBuf::from)
        .filter(|dir| dir.is_absolute())
        .or_else(|| home.filter(|h| h.is_absolute()).map(|h| h.join(".config")))?;
    Some(config.join("autostart").join("miyu-applet.desktop"))
}

/// Linux 的自动启动文件（XDG Desktop Entry）。`root` 是托盘起来时明说的数据根，有的写成 `--home <数据根>`。
#[cfg_attr(target_os = "macos", allow(dead_code, reason = "macOS、Windows 不用"))]
#[cfg_attr(windows, allow(dead_code, reason = "macOS、Windows 不用"))]
pub fn desktop_entry(exe: &Path, root: Option<&Path>) -> String {
    let home = root.map_or_else(String::new, |root| {
        format!(" {HOME_ARG} {}", desktop_exec(&root.to_string_lossy()))
    });
    format!(
        "[Desktop Entry]\nType=Application\nName=Miyu\nExec={}{home} {ARG}\nIcon=miyu-applet\nTerminal=false\nNoDisplay=true\nX-GNOME-Autostart-enabled=true\n",
        desktop_exec(&exe.to_string_lossy())
    )
}

/// 照 Desktop Entry 规范写 `Exec` 里的一个参数：有保留字符的用双引号包起来，里面的 `"`、`` ` ``、`$`、`\` 加反斜杠；再照
/// 字符串值的规矩把反斜杠写成两个；`%` 写成 `%%`。
fn desktop_exec(arg: &str) -> String {
    const RESERVED: &[char] = &[
        ' ', '\t', '\n', '"', '\'', '\\', '>', '<', '~', '|', '&', ';', '$', '*', '?', '#', '(',
        ')', '`',
    ];
    let quoted = if arg.contains(RESERVED) {
        let mut out = String::from("\"");
        for c in arg.chars() {
            if matches!(c, '"' | '`' | '$' | '\\') {
                out.push('\\');
            }
            out.push(c);
        }
        out.push('"');
        out
    } else {
        arg.to_string()
    };
    quoted.replace('\\', "\\\\").replace('%', "%%")
}

/// macOS 上那个文件。
#[cfg_attr(not(target_os = "macos"), allow(dead_code, reason = "只有 macOS 用"))]
pub fn launch_agent_file(home: Option<PathBuf>) -> Option<PathBuf> {
    let home = home.filter(|h| h.is_absolute())?;
    Some(
        home.join("Library")
            .join("LaunchAgents")
            .join(format!("{LABEL}.plist")),
    )
}

/// macOS 的 LaunchAgent：登录到图形界面时拉起一次。`root` 是托盘起来时明说的数据根，有的在参数里写 `--home <数据根>`。
#[cfg_attr(not(target_os = "macos"), allow(dead_code, reason = "只有 macOS 用"))]
pub fn launch_agent(exe: &Path, root: Option<&Path>) -> String {
    let home = root.map_or_else(String::new, |root| {
        format!(
            "    <string>{HOME_ARG}</string>\n    <string>{}</string>\n",
            xml_escape(&root.to_string_lossy())
        )
    });
    format!(
        concat!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n",
            "<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n",
            "<plist version=\"1.0\">\n<dict>\n",
            "  <key>Label</key>\n  <string>{label}</string>\n",
            "  <key>ProgramArguments</key>\n  <array>\n    <string>{exe}</string>\n{home}    <string>{arg}</string>\n  </array>\n",
            "  <key>RunAtLoad</key>\n  <true/>\n",
            "  <key>LimitLoadToSessionType</key>\n  <string>Aqua</string>\n",
            "  <key>ProcessType</key>\n  <string>Interactive</string>\n",
            "</dict>\n</plist>\n"
        ),
        label = LABEL,
        exe = xml_escape(&exe.to_string_lossy()),
        arg = ARG,
        home = home,
    )
}

fn xml_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Windows `Run` 项里的那一行：程序加引号（`Program Files` 有空格），有明说的数据根的带 `--home "<数据根>"`（同网址协议登记的写法，
/// `protocol::register`），再带参数。
#[cfg_attr(not(windows), allow(dead_code, reason = "只有 Windows 用"))]
pub fn run_value(exe: &Path, root: Option<&Path>) -> String {
    let home = root.map_or_else(String::new, |root| {
        format!(" {HOME_ARG} \"{}\"", root.display())
    });
    format!("\"{}\"{home} {ARG}", exe.display())
}

/// 删一个文件，本来就没有的不算错。
#[cfg_attr(windows, allow(dead_code, reason = "Windows 不用"))]
fn remove(path: &Path) -> io::Result<()> {
    match std::fs::remove_file(path) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

/// 写一个文件，连同没有的上级目录。
#[cfg_attr(windows, allow(dead_code, reason = "Windows 不用"))]
fn write(path: &Path, text: &str) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, text)
}

#[cfg_attr(windows, allow(dead_code, reason = "Windows 不用"))]
fn no_home() -> io::Error {
    io::Error::new(io::ErrorKind::NotFound, "no home folder")
}

#[cfg(windows)]
mod sys {
    use super::{NAME, run_value};
    use std::io;
    use std::path::Path;
    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_SET_VALUE};

    const RUN: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

    pub fn enabled() -> bool {
        RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey_with_flags(RUN, KEY_READ)
            .and_then(|key| key.get_value::<String, _>(NAME))
            .is_ok()
    }

    pub fn enable(exe: &Path, root: Option<&Path>) -> io::Result<()> {
        let (key, _) = RegKey::predef(HKEY_CURRENT_USER).create_subkey(RUN)?;
        key.set_value(NAME, &run_value(exe, root))
    }

    pub fn disable() -> io::Result<()> {
        let key = RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(RUN, KEY_SET_VALUE)?;
        match key.delete_value(NAME) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            other => other,
        }
    }
}

#[cfg(target_os = "macos")]
mod sys {
    use super::{launch_agent, launch_agent_file, no_home, remove, write};
    use std::io;
    use std::path::{Path, PathBuf};

    fn file() -> io::Result<PathBuf> {
        launch_agent_file(std::env::home_dir()).ok_or_else(no_home)
    }

    pub fn enabled() -> bool {
        file().is_ok_and(|file| file.is_file())
    }

    pub fn enable(exe: &Path, root: Option<&Path>) -> io::Result<()> {
        write(&file()?, &launch_agent(exe, root))
    }

    pub fn disable() -> io::Result<()> {
        remove(&file()?)
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
mod sys {
    use super::{desktop_entry, desktop_file, no_home, remove, write};
    use std::io;
    use std::path::{Path, PathBuf};

    fn file() -> io::Result<PathBuf> {
        desktop_file(std::env::var_os("XDG_CONFIG_HOME"), std::env::home_dir()).ok_or_else(no_home)
    }

    /// 文件在、没写 `Hidden=true`（桌面环境的设置里关掉的会这样写）。
    pub fn enabled() -> bool {
        file()
            .and_then(std::fs::read_to_string)
            .is_ok_and(|text| !text.lines().any(|line| line.trim() == "Hidden=true"))
    }

    pub fn enable(exe: &Path, root: Option<&Path>) -> io::Result<()> {
        write(&file()?, &desktop_entry(exe, root))
    }

    pub fn disable() -> io::Result<()> {
        remove(&file()?)
    }
}

#[cfg(test)]
mod tests;
