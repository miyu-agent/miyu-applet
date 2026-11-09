//! 菜单上的每一样实际做什么（图纸 `app.md` 第 2、4 条）。会等的（调 `miyu`、拉起核心、弹对话框）都在后台线程里做，做完经
//! [`Post`] 告诉界面；对话框也只在后台线程里弹（Linux 上 `rfd` 把它交给界面线程去画，界面线程自己弹会等死自己）。
//!
//! | 菜单 | 做什么 |
//! |---|---|
//! | 打开网页界面 | `miyu web` |
//! | 设置… | `miyu web --settings`；旧网页不认这个参数（退出码 2）的，退回 `miyu web` |
//! | 软件包挂的入口 | `miyu pkg page <编号>`（核心 BP-2：自己起服务的包开它的服务，别的开核心给的后台页）；退回同上 |
//! | 局域网访问 | 连核心改 `web.listen`（见 `lan`）；要打开可是没设访问密码的，改成 `miyu web --settings access` |
//! | 点了系统通知 | `miyu web --session <编号>`（网页 W-1085）；账号级的提醒 `miyu web`；退回同上 |
//! | 在手机上打开… | `miyu web --settings access`（网页设置「访问」那一组，局域网地址和二维码在那里，网页 W-1086）；退回同上 |
//! | 停止 | `miyu web --stop`（网页 W-1091），再 `miyu core stop` |
//! | 启动 | 照头的办法拉起核心：建好数据根的骨架，`connect_or_start` 拉起 `miyu core`（不开浏览器） |
//! | 重新启动 | 先停核心再起（网页软件留着，页面接着连新的） |
//! | 打开日志所在的文件夹 | 用系统的文件管理器打开数据根的 `var/log/` |
//! | 关于 Miyu | 系统的消息框，见 `about` |
//! | 退出 Miyu | 同「停止」，再退 |

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use miyu_client::places::DataRoot;

use crate::app::{Event, Post};
use crate::link::Waker;
use crate::program::{self, Ran, RunError};
use crate::status::Busy;
use crate::text::{Text, key};
use crate::{about, lan, log, quit};

/// 做事要用的几样：主程序、数据根、给人看的字、告诉界面的、叫连接再试的。
#[derive(Clone)]
pub struct Ctx {
    pub miyu: Option<PathBuf>,
    pub root: Option<DataRoot>,
    pub text: Arc<Text>,
    pub post: Post,
    pub waker: Option<Waker>,
}

impl Ctx {
    fn wake(&self) {
        if let Some(waker) = &self.waker {
            waker.wake();
        }
    }
}

/// 在后台线程里做 `work`。
fn background(name: &str, work: impl FnOnce() + Send + 'static) {
    if let Err(error) = std::thread::Builder::new().name(name.into()).spawn(work) {
        log::line(&format!("could not start {name}: {error}"));
    }
}

/// 用浏览器打开网页的哪一处。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Page {
    /// 首页（`miyu web`）。
    Home,
    /// 设置（`--settings`，网页 W-1085）。
    Settings,
    /// 设置「通用」页的「访问」那一组（`--settings access`，网页 W-1085）：访问密码、局域网地址和二维码。
    Access,
    /// 一个软件包的后台页（`--package <编号>`，网页 F-6 下）。
    Package(String),
    /// 一个会话（`--session <编号>`，网页 W-1085）：点了系统通知。
    Session(String),
}

impl Page {
    /// 跟在 `miyu` 后面的参数。
    pub fn args(&self) -> Vec<&str> {
        match self {
            Page::Home => program::WEB.to_vec(),
            Page::Settings => program::SETTINGS.to_vec(),
            Page::Access => vec!["web", "--settings", "access"],
            Page::Package(package) => vec!["pkg", "page", package],
            Page::Session(session) => vec!["web", "--session", session],
        }
    }
}

/// 用浏览器打开网页界面的 `page` 那一处。没在跑的由 `miyu web` 拉起核心、网页软件。
pub fn open_web(ctx: &Ctx, page: Page) {
    let ctx = ctx.clone();
    background("open-web", move || open_web_now(&ctx, &page));
}

/// 「局域网访问」点了，`on` 是现在开着没有：反过来设（见 `lan`）。要打开可是没设访问密码的，打开网页设置「访问」那一组先设。
/// 设成了的不用管菜单：核心推来配置的变动，托盘那条连接再读一遍、改打勾。
pub fn toggle_lan(ctx: &Ctx, on: bool) {
    let ctx = ctx.clone();
    background("lan", move || {
        let Some(root) = ctx.root.as_ref() else {
            tell(&ctx.text, key::LAN_FAILED, "");
            return;
        };
        match lan::set(root, !on) {
            Ok(lan::Outcome::Done) => log::line(&format!("LAN access set to {}", !on)),
            Ok(lan::Outcome::NeedsPassword) => {
                log::line("no access password yet, opening the access settings");
                open_web_now(&ctx, &Page::Access);
            }
            Err(failed) => {
                let why = match failed {
                    lan::Failed::Timeout => ctx.text.get(key::TIMEOUT).to_owned(),
                    lan::Failed::Said(why) => why,
                };
                log::line(&format!("LAN access not changed: {why}"));
                tell(&ctx.text, key::LAN_FAILED, &why);
            }
        }
    });
}

/// 同 [`open_web`]，在调的这个线程里做。
fn open_web_now(ctx: &Ctx, page: &Page) {
    let Some(miyu) = ctx.miyu.as_deref() else {
        tell(&ctx.text, key::NO_PROGRAM, "");
        return;
    };
    let mut got = run_logged(miyu, &page.args(), program::WEB_LIMIT);
    if *page != Page::Home && not_understood(&got) {
        log::line(&format!(
            "the web app does not know {page:?}, opening the web interface instead"
        ));
        got = run_logged(miyu, program::WEB, program::WEB_LIMIT);
    }
    if let Some(detail) = failure(&got, &ctx.text) {
        ctx.post.send(Event::Failed(detail.clone()));
        tell(&ctx.text, key::OPEN_FAILED, &detail);
    }
    // 没在跑的由 `miyu web` 拉起来了：叫连接马上再试，不光等文件变动。
    ctx.wake();
}

/// 「停止」。界面已经写上「正在停止…」。
pub fn stop(ctx: &Ctx) {
    let ctx = ctx.clone();
    background("stop", move || {
        let ok = stop_now(&ctx, true);
        ctx.post.send(Event::Done(Busy::Stopping, ok));
        ctx.wake();
    });
}

/// 「启动」。界面已经写上「正在启动…」。
pub fn start(ctx: &Ctx) {
    let ctx = ctx.clone();
    background("start", move || {
        let ok = start_now(&ctx);
        ctx.post.send(Event::Done(Busy::Starting, ok));
        ctx.wake();
    });
}

/// 「重新启动」：先停，停成了再起。界面已经写上「正在停止…」。
pub fn restart(ctx: &Ctx) {
    let ctx = ctx.clone();
    background("restart", move || {
        let stopped = stop_now(&ctx, false);
        if stopped {
            ctx.post.send(Event::Begin(Busy::Starting));
        }
        ctx.post.send(Event::Done(Busy::Stopping, stopped));
        if stopped {
            let ok = start_now(&ctx);
            ctx.post.send(Event::Done(Busy::Starting, ok));
        }
        ctx.wake();
    });
}

/// 「退出 Miyu」：叫网页软件、核心停，停没停成都退。
pub fn quit(ctx: &Ctx) {
    let ctx = ctx.clone();
    background("quit", move || {
        match ctx.miyu.as_deref() {
            Some(miyu) => {
                quit::stop_web(miyu);
                if let Err(why) = quit::stop_core(miyu, &ctx.text) {
                    log::line(&format!("quitting although the core did not stop: {why}"));
                }
            }
            None => log::line("main program not found, the core is left alone"),
        }
        ctx.post.send(Event::Exit);
    });
}

/// 「打开日志所在的文件夹」。
pub fn open_logs(ctx: &Ctx) {
    let Some(folder) = ctx.root.as_ref().map(DataRoot::logs) else {
        let text = Arc::clone(&ctx.text);
        background("tell", move || tell(&text, key::LOGS_FAILED, ""));
        return;
    };
    let text = Arc::clone(&ctx.text);
    background("open-logs", move || {
        if !folder.is_dir() {
            tell(&text, key::LOGS_FAILED, "");
            return;
        }
        if let Err(error) = open::that_detached(&folder) {
            log::line(&format!("opening {}: {error}", folder.display()));
            tell(&text, key::LOGS_FAILED, &error.to_string());
        }
    });
}

/// 「关于 Miyu」：`web` 是连着核心时记下的网页版本。
pub fn about(ctx: &Ctx, web: Option<String>) {
    let ctx = ctx.clone();
    background("about", move || {
        let message = about::gather(&ctx.text, ctx.miyu.as_deref(), web);
        let shown = rfd::MessageDialog::new()
            .set_title(ctx.text.get(key::ABOUT))
            .set_description(message)
            .set_level(rfd::MessageLevel::Info)
            .set_buttons(rfd::MessageButtons::Ok)
            .show();
        log::line(&format!("about closed: {shown:?}"));
    });
}

/// 「开机时启动」点了：照现在的反过来设，交回设完以后真的开着没有。设不成的弹一句。
pub fn toggle_autostart(ctx: &Ctx) -> bool {
    let want = !crate::autostart::enabled();
    if let Err(error) = crate::autostart::set(want) {
        log::line(&format!("autostart {want}: {error}"));
        let text = Arc::clone(&ctx.text);
        background("tell", move || {
            tell(&text, key::AUTOSTART_FAILED, &error.to_string());
        });
    }
    crate::autostart::enabled()
}

/// 叫核心停，`web` 的先叫网页软件停；没停成的弹一句。
fn stop_now(ctx: &Ctx, web: bool) -> bool {
    let Some(miyu) = ctx.miyu.as_deref() else {
        tell(&ctx.text, key::NO_PROGRAM, "");
        return false;
    };
    if web {
        quit::stop_web(miyu);
    }
    match quit::stop_core(miyu, &ctx.text) {
        Ok(()) => true,
        Err(why) => {
            tell(&ctx.text, key::STOP_FAILED, &why);
            false
        }
    }
}

fn start_now(ctx: &Ctx) -> bool {
    let (Some(miyu), Some(root)) = (ctx.miyu.as_deref(), ctx.root.as_ref()) else {
        tell(&ctx.text, key::NO_PROGRAM, "");
        return false;
    };
    let started = Instant::now();
    let got = start_core(miyu, root);
    log::line(&format!(
        "starting the core took {} ms: {got:?}",
        started.elapsed().as_millis()
    ));
    match got {
        Ok(()) => true,
        Err(why) => {
            ctx.post.send(Event::Failed(why.clone()));
            tell(&ctx.text, key::START_FAILED, &why);
            false
        }
    }
}

/// 照头的办法拉起核心（同终端、`miyu web`）：建好数据根的骨架（第一次用时写标记），连核心，没在跑就照 `miyu core` 拉起来，
/// 等它说好了。连上的那条马上放掉：状态照托盘自己那条连接来。核心说了起不来的原因的，交回它的原话（照人的语言，例如「这是旧版
/// Miyu 的数据根，新版不认」），状态行「出错了」下面照它写。
pub(crate) fn start_core(miyu: &Path, root: &DataRoot) -> Result<(), String> {
    let lang = crate::text::Lang::of_locale(&crate::text::system_locale());
    // 认不出的数据根带代码（核心 LC-1）：照托盘的语言写，别的照原话
    root.prepare()
        .map_err(|error| crate::reason::say(error.code(), &error.to_string(), root, lang))?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| error.to_string())?;
    let miyu = miyu.to_path_buf();
    runtime
        .block_on(miyu_client::connect::connect_or_start(root, move || {
            let mut command = program::command(&miyu);
            command.arg(program::CORE);
            command
        }))
        .map(drop)
        .map_err(|error| match error {
            miyu_client::connect::StartError::Refused { code, reason } => {
                crate::reason::say(code.as_deref(), &reason, root, lang)
            }
            other => other.to_string(),
        })
}

/// 调一次 `miyu`，记下耗时和退出码（不记输出：`miyu web` 印的网址里有票）。
fn run_logged(miyu: &Path, args: &[&str], limit: std::time::Duration) -> Result<Ran, RunError> {
    let started = Instant::now();
    let got = program::run(miyu, args, limit);
    let outcome = match &got {
        Ok(ran) => format!("exit {:?}", ran.code),
        Err(error) => format!("{error:?}"),
    };
    log::line(&format!(
        "miyu {} took {} ms: {outcome}",
        args.join(" "),
        started.elapsed().as_millis()
    ));
    got
}

/// 这一次是不是用法不对（网页包不认参数）。
fn not_understood(got: &Result<Ran, RunError>) -> bool {
    matches!(got, Ok(ran) if ran.code == Some(program::USAGE))
}

/// 没成的交给人看的一句：`miyu` 标准错误的最后几行，超时、起不来的照那一种说。成了的交 `None`。
fn failure(got: &Result<Ran, RunError>, text: &Text) -> Option<String> {
    match got {
        Ok(ran) if ran.code == Some(0) => None,
        Ok(ran) => Some(program::tail(&ran.stderr)),
        Err(RunError::Timeout) => Some(text.get(key::TIMEOUT).to_owned()),
        Err(RunError::Spawn(error) | RunError::Wait(error)) => Some(error.to_string()),
    }
}

/// 弹一个出错的消息框，等人点。头一句照 `lead` 这个键，下面是 `detail`（空的不写）。在后台线程里调。
fn tell(text: &Text, lead: &str, detail: &str) {
    let message = match detail.trim() {
        "" => text.get(lead).to_owned(),
        detail => format!("{}\n\n{detail}", text.get(lead)),
    };
    let shown = rfd::MessageDialog::new()
        .set_title("Miyu")
        .set_description(message)
        .set_level(rfd::MessageLevel::Error)
        .set_buttons(rfd::MessageButtons::Ok)
        .show();
    log::line(&format!("told {lead}: {shown:?}"));
}

#[cfg(test)]
mod tests;
