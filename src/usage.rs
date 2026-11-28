//! 命令行上问一句就退的两样（施工 AP-24，图纸 `app.md` 第 1 条）：`-V`、`--version` 印一行 `miyu-applet <版本>`，`-h`、`--help`
//! 印帮助页，都印在标准输出上、退出码 0。`main` 一上来就照它办，在找数据根、单实例、建事件循环和托盘、连核心、打开浏览器之前：
//! rc.5 装机时跑 `miyu-applet --version` 起了托盘，首次起来还会自己开浏览器。
//!
//! 版本照核心 `miyu --version` 的写法（程序名加版本，同 clap）；帮助页照核心、网页的帮助页（`用法：` 一行、说明、`选项：`），三种语言
//! 住在 `resources/help/`，照这台电脑的语言挑（同托盘菜单）。别的参数照旧不管：`--autostart`、`--open`、`--home` 是启动项、
//! 通知的网址给的，不进帮助（同网页不写 `serve`、`forget`）；不认识的照旧当没写，起托盘。

use crate::protocol;
use crate::text::Lang;

/// 只问一句就退的是哪一样。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Early {
    /// `-h`、`--help`：印帮助页。
    Help,
    /// `-V`、`--version`：印版本。
    Version,
}

/// 照参数（不带程序名）看是不是只问一句：哪里有 `-h`、`--help` 都是帮助（两样都写了的也是帮助，同网页），不然有 `-V`、`--version`
/// 的是版本。`--open`、`--home` 后面那一个是它们的值，不算。
pub fn early(args: impl IntoIterator<Item = String>) -> Option<Early> {
    let mut found = None;
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => return Some(Early::Help),
            "-V" | "--version" => found = Some(Early::Version),
            protocol::ARG | protocol::HOME_ARG => {
                args.next();
            }
            _ => {}
        }
    }
    found
}

/// `--version` 印的那一行（不带换行）。
pub fn version_line() -> String {
    format!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
}

/// 这种语言的帮助页，以一个换行结尾。
pub fn help(lang: Lang) -> &'static str {
    match lang {
        Lang::Zh => include_str!("../resources/help/zh.txt"),
        Lang::En => include_str!("../resources/help/en.txt"),
        Lang::Ja => include_str!("../resources/help/ja.txt"),
    }
}

/// 印出来：帮助页照这台电脑的语言（同托盘菜单，`text::system_locale`）。
pub fn answer(early: Early) {
    match early {
        Early::Help => print!("{}", help(Lang::of_locale(&crate::text::system_locale()))),
        Early::Version => println!("{}", version_line()),
    }
}

#[cfg(test)]
mod tests;
