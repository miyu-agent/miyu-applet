//! 点通知用的网址 `miyu-applet://open/?session=<编号>`（AP-4 补，2026-11-04 Windows 虚拟机上测出来的）：Windows 上没打包的程序，通知收进
//! 通知中心以后再点，系统不交回 `Activated`（要注册 COM 激活器，仓库不许 `unsafe`）。改成照网址激活：toast 的正文、「打开」按钮都写这个
//! 网址，系统照当前用户注册表里登记的 `miyu-applet` 拉起 `miyu-applet.exe --open <网址>`；已经有托盘在跑的照单实例交给它（`instance::ask`
//! 带上会话），没在跑的这一个起来就打开那个会话。横幅上点、通知中心里点、托盘退了以后点都一样。
//!
//! 登记写在 `HKCU\Software\Classes\miyu-applet`（不要管理员），每次起来照自己现在的位置写一遍；卸载时不删（同「开机时启动」的那一项）。

/// 网址的协议名。
pub const SCHEME: &str = "miyu-applet";

/// 命令行上带网址的参数。
pub const ARG: &str = "--open";

/// 命令行上带数据根的参数：登记时写进去的托盘的数据根。照网址拉起的程序环境变量照系统的（`MIYU_HOME` 这类不一定在），照它找同一个
/// 数据根、交给同一个托盘（2026-11-04 虚拟机上测出：数据根照 `MIYU_HOME` 另指的，点了通知拉起来的去了默认的那一个）。
pub const HOME_ARG: &str = "--home";

/// 点了这条通知要去的网址：有会话的 `miyu-applet://open/?session=<编号>`，账号级的提醒 `miyu-applet://open/`。
pub fn link(session: Option<&str>) -> String {
    match session {
        Some(id) => format!("{SCHEME}://open/?session={id}"),
        None => format!("{SCHEME}://open/"),
    }
}

/// 读一个网址：是这里的、写法对的交回要打开的会话（`None` 是打开网页界面）；别的（不是这个协议、会话编号写法不对）是 `None`。
pub fn parse(url: &str) -> Option<Option<String>> {
    let rest = url.strip_prefix(SCHEME)?.strip_prefix("://open")?;
    let rest = rest.strip_prefix('/').unwrap_or(rest);
    if rest.is_empty() {
        return Some(None);
    }
    let id = rest.strip_prefix("?session=")?;
    session_id(id).then(|| Some(id.to_owned()))
}

/// 照命令行找 `--open <网址>`：有、读得懂的交回要打开的。
pub fn from_args(args: impl IntoIterator<Item = String>) -> Option<Option<String>> {
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        if arg == ARG {
            return args.next().as_deref().and_then(parse);
        }
    }
    None
}

/// 照命令行找 `--home <数据根>`。
pub fn home_from_args(args: impl IntoIterator<Item = String>) -> Option<std::path::PathBuf> {
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        if arg == HOME_ARG {
            return args
                .next()
                .filter(|home| !home.is_empty())
                .map(std::path::PathBuf::from);
        }
    }
    None
}

/// 会话编号的写法（核心的 UUID：十六进制和 `-`，8-4-4-4-12）。别的不认：网址是外面交进来的。
fn session_id(text: &str) -> bool {
    let parts: Vec<&str> = text.split('-').collect();
    parts.iter().map(|part| part.len()).eq([8, 4, 4, 4, 12])
        && parts
            .iter()
            .all(|part| part.bytes().all(|b| b.is_ascii_hexdigit()))
}

/// 在当前用户的注册表里登记 `miyu-applet` 协议，指到 `exe`、带上数据根 `home`（[`open_command`]）。
///
/// # Errors
///
/// 写不进注册表。
#[cfg(windows)]
pub fn register(exe: &std::path::Path, home: &std::path::Path) -> std::io::Result<()> {
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;

    let classes = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey_with_flags(r"Software\Classes", winreg::enums::KEY_WRITE)?;
    let (key, _) = classes.create_subkey(SCHEME)?;
    key.set_value("", &"URL:Miyu")?;
    key.set_value("URL Protocol", &"")?;
    let (command, _) = key.create_subkey(r"shell\open\command")?;
    command.set_value("", &open_command(exe, home))
}

/// 登记的那一行命令：`"<exe>" --home "<数据根>" --open "%1"`，数据根照 [`quoted`] 加引号。
#[cfg_attr(not(windows), allow(dead_code, reason = "只有 Windows 用"))]
pub fn open_command(exe: &std::path::Path, home: &std::path::Path) -> String {
    format!(
        "\"{}\" {HOME_ARG} {} {ARG} \"%1\"",
        exe.display(),
        quoted(&home.to_string_lossy())
    )
}

/// Windows 命令行上的一个参数，照系统拆参数的规矩（`CommandLineToArgvW`）加引号：紧挨着引号的反斜杠要成双，不然 `D:\miyu\` 收尾的
/// `\"` 被读成字面的引号，后面的参数都并进它里面（审计 #099）。程序那一格不照这个拆，不用它。
#[cfg_attr(not(windows), allow(dead_code, reason = "只有 Windows 用"))]
pub fn quoted(arg: &str) -> String {
    let mut out = String::with_capacity(arg.len() + 2);
    out.push('"');
    let mut slashes = 0;
    for c in arg.chars() {
        match c {
            '\\' => slashes += 1,
            '"' => {
                out.extend(std::iter::repeat_n('\\', slashes * 2 + 1));
                out.push('"');
                slashes = 0;
            }
            other => {
                out.extend(std::iter::repeat_n('\\', slashes));
                out.push(other);
                slashes = 0;
            }
        }
    }
    out.extend(std::iter::repeat_n('\\', slashes * 2));
    out.push('"');
    out
}

#[cfg(test)]
mod tests;
