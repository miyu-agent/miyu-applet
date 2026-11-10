//! 调装在旁边的主程序 `miyu`（图纸 `app.md`「它是什么」）：打开网页界面、设置，叫核心停，问版本。托盘自己只连核心看状态，
//! 别的事都交给它。

use std::ffi::OsString;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

/// 用浏览器打开网页界面（`miyu web`：核心、网页服务没在跑的由它拉起，带交接票交给系统的浏览器）。
pub const WEB: &[&str] = &["web"];
/// 用浏览器打开网页的设置（网页 W-1085 加的参数；旧网页不认，见 [`USAGE`]）。
pub const SETTINGS: &[&str] = &["web", "--settings"];
/// 叫核心停（核心 CL-4）：软件包跟着停。
pub const STOP: &[&str] = &["core", "stop"];
/// 叫网页软件停（网页 W-1091）：不连核心；旧网页不认，见 [`USAGE`]。
pub const STOP_WEB: &[&str] = &["web", "--stop"];
/// 核心进程本身：头照 `miyu core` 把它拉起来（核心的 `connect_or_start`）。
pub const CORE: &str = "core";
/// 问主程序的版本。
pub const VERSION: &[&str] = &["--version"];
/// 打开网页最多等多久：第一次要先拉起核心和网页软件（核心等它起来最多 60 秒）。
pub const WEB_LIMIT: Duration = Duration::from_secs(90);
/// 叫核心停最多等多久：`miyu core stop` 自己最多等 60 秒。
pub const STOP_LIMIT: Duration = Duration::from_secs(70);
/// 叫网页软件停最多等多久：`miyu web --stop` 自己等它放锁最多 10 秒。
pub const STOP_WEB_LIMIT: Duration = Duration::from_secs(20);
/// 问版本最多等多久。
pub const VERSION_LIMIT: Duration = Duration::from_secs(10);
/// 用法不对的退出码：网页包的 `open` 不认的参数照它退（网页 `main.rs` 的 `USAGE`），`miyu web` 原样转回来。
pub const USAGE: i32 = 2;
/// 出错时带回标准错误的最后几行。
const TAIL_LINES: usize = 6;

/// 找主程序：`MIYU_PROGRAM` 写了的用它；再看外壳自己所在的目录；再照 `PATH` 找，只认里面的绝对目录（第五批审计 D5-4：
/// 空项、`.`、相对目录照外壳的当前目录解析，图形程序的当前目录说不准）。`exists` 说那个文件在不在。
pub fn locate(
    program: Option<OsString>,
    own_dir: Option<&Path>,
    path: Option<OsString>,
    exists: impl Fn(&Path) -> bool,
) -> Option<PathBuf> {
    if let Some(program) = program.filter(|p| !p.is_empty()) {
        return Some(PathBuf::from(program));
    }
    let name = format!("miyu{}", std::env::consts::EXE_SUFFIX);
    let beside = own_dir.map(|dir| dir.join(&name));
    let on_path = path
        .into_iter()
        .flat_map(|p| std::env::split_paths(&p).collect::<Vec<_>>())
        .filter(|dir| dir.is_absolute());
    beside
        .into_iter()
        .chain(on_path.map(|dir| dir.join(&name)))
        .find(|p| exists(p))
}

/// `PATH` 里也没有时再看的几个目录：macOS 从访达、开机启动拉起的图形程序，`PATH` 只有 `/usr/bin:/bin:/usr/sbin:/sbin`。先看家目录下
/// 安装脚本装的地方（核心 `packaging/install.sh`：程序在 `~/.local/lib/miyu/miyu`，`~/.local/bin/miyu` 链过去；AP-19，2026-11-10 项目主人：
/// Mac 上照安装脚本装的，托盘找不到主程序），再看 Homebrew 装的 `miyu`（只装托盘的 `.app` 配 Homebrew 的 miyu-core，施工 AP-11）的前缀。
/// 完整的安装包里核心就在托盘旁边，轮不到这里。Linux 从桌面、开机启动拉起的 `PATH` 也不一定带 `~/.local/bin`，家目录这两处照样看。
fn fallback_dirs(home: Option<&Path>) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    if cfg!(windows) {
        return dirs;
    }
    if let Some(home) = home.filter(|home| home.is_absolute()) {
        dirs.push(home.join(".local/lib/miyu"));
        dirs.push(home.join(".local/bin"));
    }
    if cfg!(target_os = "macos") {
        dirs.extend(["/opt/homebrew/bin", "/usr/local/bin"].map(PathBuf::from));
    }
    dirs
}

/// 在 `dirs` 里照先后找主程序。
pub fn in_dirs(dirs: &[PathBuf], exists: impl Fn(&Path) -> bool) -> Option<PathBuf> {
    let name = format!("miyu{}", std::env::consts::EXE_SUFFIX);
    dirs.iter().map(|dir| dir.join(&name)).find(|p| exists(p))
}

/// 照这个进程的环境找主程序：[`locate`]，都没有的再看 [`fallback_dirs`]（家目录照 `HOME`）。
pub fn locate_here() -> Option<PathBuf> {
    let own = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf));
    let home = std::env::var_os("HOME").map(PathBuf::from);
    locate(
        std::env::var_os("MIYU_PROGRAM"),
        own.as_deref(),
        std::env::var_os("PATH"),
        Path::is_file,
    )
    .or_else(|| in_dirs(&fallback_dirs(home.as_deref()), Path::is_file))
}

/// 调了一次，跑完了。
#[derive(Debug)]
pub struct Ran {
    /// 退出码；被信号杀掉的没有。
    pub code: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

/// 调不成、没跑完。
#[derive(Debug)]
pub enum RunError {
    /// 起不来（找不到、没有权限……）。
    Spawn(std::io::Error),
    /// 等它退出时出错。
    Wait(std::io::Error),
    /// 过了时限还没退，杀掉了。
    Timeout,
}

/// 托盘用的数据根（起来时定下，[`pin_home`]）：拉起的 `miyu` 都照它，带上 `MIYU_HOME`。点了通知照 `miyu-applet:` 网址拉起的托盘，
/// 数据根是登记时写进去的（`protocol`），环境变量里没有，子进程不带上就会去默认的那一个。
static HOME: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

/// 定下托盘用的数据根（只定一次）。
pub fn pin_home(root: &Path) {
    if HOME.set(root.to_path_buf()).is_err() {
        // 定过了：照第一次的。
    }
}

/// 调 `program` 的命令：带上托盘的数据根（[`pin_home`]）。
pub fn command(program: &Path) -> Command {
    let mut command = Command::new(program);
    if let Some(home) = HOME.get() {
        command.env("MIYU_HOME", home);
    }
    command
}

/// 调 `program args`，最多等 `limit`；标准输入是空的，标准输出、标准错误收下来。Windows 上不开控制台窗口。
///
/// # Errors
/// 起不来、等不了、超时。
pub fn run(program: &Path, args: &[&str], limit: Duration) -> Result<Ran, RunError> {
    let mut command = command(program);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    no_console(&mut command);
    let mut child = command.spawn().map_err(RunError::Spawn)?;
    let stdout = collect(child.stdout.take());
    let stderr = collect(child.stderr.take());
    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if started.elapsed() < limit => thread::sleep(Duration::from_millis(20)),
            Ok(None) => {
                if let Err(error) = child.kill().and_then(|()| child.wait().map(drop)) {
                    crate::log::line(&format!("could not kill {}: {error}", program.display()));
                }
                return Err(RunError::Timeout);
            }
            Err(error) => return Err(RunError::Wait(error)),
        }
    };
    Ok(Ran {
        code: status.code(),
        stdout: gather(stdout),
        stderr: gather(stderr),
    })
}

/// 在另一个线程里读一个管道，读到一块交一块。不等读到头：Windows 上它拉起的程序（核心、网页软件）会继承这条管道一直拿着，
/// 读到头要等它们退出（施工 DT-6 在 Windows 上碰到：第一次开窗口时拿不到网址）。
fn collect(pipe: Option<impl Read + Send + 'static>) -> mpsc::Receiver<Vec<u8>> {
    let (send, receive) = mpsc::channel();
    if let Some(mut pipe) = pipe {
        thread::spawn(move || {
            let mut chunk = [0u8; 8192];
            loop {
                match pipe.read(&mut chunk) {
                    Ok(0) => return,
                    Ok(n) => {
                        if send.send(chunk[..n].to_vec()).is_err() {
                            return;
                        }
                    }
                    Err(error) => {
                        crate::log::line(&format!("reading a pipe: {error}"));
                        return;
                    }
                }
            }
        });
    }
    receive
}

/// 程序退出以后收下已经读到的：管道读到头了的立刻交，没到头的（还有别的程序拿着）再有 200 毫秒没新东西就交。
fn gather(receive: mpsc::Receiver<Vec<u8>>) -> Vec<u8> {
    let mut bytes = Vec::new();
    while let Ok(chunk) = receive.recv_timeout(QUIET) {
        bytes.extend_from_slice(&chunk);
    }
    bytes
}

/// 程序退出以后，管道多久没新东西就当读完了。
const QUIET: Duration = Duration::from_millis(200);

#[cfg(windows)]
fn no_console(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    /// `CREATE_NO_WINDOW`：图形程序调命令行程序时不闪一个黑窗口。
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn no_console(_: &mut Command) {}

/// 标准错误的最后几行（`miyu` 照人的语言说的那几句），去掉空行。
pub fn tail(stderr: &[u8]) -> String {
    let text = String::from_utf8_lossy(stderr);
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    lines[lines.len().saturating_sub(TAIL_LINES)..].join("\n")
}

#[cfg(test)]
mod tests;
