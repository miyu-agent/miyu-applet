//! 托盘自己给人看的字（图纸 `app.md` 第 4、5 条）：托盘菜单、对话框、系统通知。照系统的语言挑一份，中文、日文、英文，别的用
//! 英文；不问核心的 `ui.language`（托盘起来时核心不一定在跑）。三份住在 `resources/text/`，编进程序。

use std::collections::BTreeMap;

/// 代码里用的键。
pub mod key {
    pub const STATUS_RUNNING: &str = "status.running";
    pub const STATUS_RUNNING_LAN: &str = "status.running_lan";
    pub const STATUS_STARTING: &str = "status.starting";
    pub const STATUS_STOPPING: &str = "status.stopping";
    pub const STATUS_STOPPED: &str = "status.stopped";
    pub const STATUS_UNRESPONSIVE: &str = "status.unresponsive";
    pub const STATUS_FAILED: &str = "status.failed";
    pub const STOP: &str = "menu.stop";
    pub const START: &str = "menu.start";
    pub const RESTART: &str = "menu.restart";
    pub const OPEN: &str = "menu.open";
    pub const SETTINGS: &str = "menu.settings";
    pub const LAN: &str = "menu.lan";
    pub const PHONE: &str = "menu.phone";
    pub const AUTOSTART: &str = "menu.autostart";
    pub const LOGS: &str = "menu.logs";
    pub const ABOUT: &str = "menu.about";
    pub const QUIT: &str = "menu.quit";
    pub const NO_PROGRAM: &str = "error.no_program";
    pub const OPEN_FAILED: &str = "error.open";
    pub const START_FAILED: &str = "error.start";
    pub const STOP_FAILED: &str = "error.stop";
    pub const AUTOSTART_FAILED: &str = "error.autostart";
    pub const LAN_FAILED: &str = "error.lan";
    pub const LOGS_FAILED: &str = "error.logs";
    pub const TIMEOUT: &str = "error.timeout";
    pub const ABOUT_APP: &str = "about.app";
    pub const ABOUT_CORE: &str = "about.core";
    pub const ABOUT_WEB: &str = "about.web";
    pub const ABOUT_UNKNOWN: &str = "about.unknown";
    pub const NOTIFY_DONE: &str = "notify.done";
    pub const NOTIFY_ASK: &str = "notify.ask";
    pub const NOTIFY_APPROVE: &str = "notify.approve";
    pub const NOTIFY_SCHEDULE_RAN: &str = "notify.schedule_ran";
    pub const NOTIFY_SCHEDULE_FAILED: &str = "notify.schedule_failed";
    pub const NOTIFY_SCHEDULES_RESET: &str = "notify.schedules_reset";
    pub const NOTIFY_OPEN: &str = "notify.open";
}

/// 上面每一个键，测试查三份里都有。
#[cfg(test)]
const KEYS: [&str; 37] = [
    key::STATUS_RUNNING,
    key::STATUS_RUNNING_LAN,
    key::STATUS_STARTING,
    key::STATUS_STOPPING,
    key::STATUS_STOPPED,
    key::STATUS_UNRESPONSIVE,
    key::STATUS_FAILED,
    key::STOP,
    key::START,
    key::RESTART,
    key::OPEN,
    key::SETTINGS,
    key::LAN,
    key::PHONE,
    key::AUTOSTART,
    key::LOGS,
    key::ABOUT,
    key::QUIT,
    key::NO_PROGRAM,
    key::OPEN_FAILED,
    key::START_FAILED,
    key::STOP_FAILED,
    key::AUTOSTART_FAILED,
    key::LAN_FAILED,
    key::LOGS_FAILED,
    key::TIMEOUT,
    key::ABOUT_APP,
    key::ABOUT_CORE,
    key::ABOUT_WEB,
    key::ABOUT_UNKNOWN,
    key::NOTIFY_DONE,
    key::NOTIFY_ASK,
    key::NOTIFY_APPROVE,
    key::NOTIFY_SCHEDULE_RAN,
    key::NOTIFY_SCHEDULE_FAILED,
    key::NOTIFY_SCHEDULES_RESET,
    key::NOTIFY_OPEN,
];

/// 一种语言。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Zh,
    En,
    Ja,
}

impl Lang {
    /// 照系统的语言标签挑：`zh*` 中文，`ja*` 日文，别的英文。
    pub fn of_locale(locale: &str) -> Self {
        let lower = locale.to_ascii_lowercase();
        if lower.starts_with("zh") {
            Self::Zh
        } else if lower.starts_with("ja") {
            Self::Ja
        } else {
            Self::En
        }
    }

    /// 核心资源目录里说法的语言名（`human/<语言>.json`）。
    pub fn code(self) -> &'static str {
        match self {
            Self::Zh => "zh",
            Self::En => "en",
            Self::Ja => "ja",
        }
    }

    fn source(self) -> &'static str {
        match self {
            Self::Zh => include_str!("../resources/text/zh.toml"),
            Self::En => include_str!("../resources/text/en.toml"),
            Self::Ja => include_str!("../resources/text/ja.toml"),
        }
    }
}

/// 这台电脑的语言（`zh-CN` 这种写法）：托盘菜单的字照它挑，连核心握手的 `locale` 也交它，软件包托盘入口的名字照连接的语言给，两边一样
/// （2026-11-04 Windows 虚拟机上测出：握手原来交 `miyu_client::open::locale()`，那个只读 `LC_ALL`、`LANG` 这些环境变量，Windows 上没有，
/// 核心照英文给了「Open tray test」，菜单别的字是中文）。系统给不出的照环境变量，都没有的是空的。
pub fn system_locale() -> String {
    sys_locale::get_locale()
        .filter(|locale| !locale.is_empty())
        .unwrap_or_else(miyu_client::open::locale)
}

/// 一份字：键到这种语言的一句。
#[derive(Debug)]
pub struct Text {
    lines: BTreeMap<String, String>,
}

impl Text {
    /// 照这个进程的系统语言挑一份。
    pub fn system() -> Self {
        Self::of(Lang::of_locale(&system_locale()))
    }

    /// 这种语言的那一份。读不出来的（不会：测试查过三份）记一行日志，每个键照键名给。
    pub fn of(lang: Lang) -> Self {
        match toml::from_str(lang.source()) {
            Ok(lines) => Self { lines },
            Err(error) => {
                crate::log::line(&format!("text for {lang:?} is broken: {error}"));
                Self {
                    lines: BTreeMap::new(),
                }
            }
        }
    }

    /// 这个键的一句；没有的照键名给。
    pub fn get<'a>(&'a self, key: &'a str) -> &'a str {
        self.lines.get(key).map_or(key, String::as_str)
    }
}

#[cfg(test)]
mod tests;
