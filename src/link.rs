//! 托盘连核心看状态（图纸 `app.md` 第 4 条、「要核心给的」第一条）：连已经在跑的核心（不拉起），出示本机令牌握手，连着就是
//! 「运行中」，断了、连不上就是「已停止」；不定时去问。连不上的时候盯着数据根的 `run/`（系统的文件变动通知）：别人把核心拉起来了，
//! 核心会在那里换令牌、写套接字的位置，一变就再连一次。
//!
//! 连法同核心门面的 `Core::connect_running`（`miyu-client` 的 `connect::connect` 加同一句 `hello`），只是自己拿着这条连接（[`Rpc`]）：
//! `Core` 只能一问一答，等不了「断了」，也收不了推送。网页软件跟着配置的那条连接（网页 `follow.rs`）也是这样写的。
//!
//! 握手以后（施工 AP-3）在这条连接上订两条流，菜单照它们画，不定时去问；掉了队（`resync`）的重新订：
//! - `packages`：回应是整份软件包列表，之后推 `package.changed`，算出软件包挂的入口（[`Packages`]）；
//! - `config`：先读一遍 `web.listen`，之后推来的 `config.changed` 里有它的再读一遍（局域网开没开）；装上、卸掉一个包也再读一遍
//!   （核心这时不推 `config.changed`，网页软件的配置项跟着包有没有）。
//!
//! 系统通知（施工 AP-4，图纸第 5 条）：握手以后先 `notices.claim`（核心 TR-1），说「后台通知由托盘来发」，核心照这条连接记、
//! 断了就作废；断了重连的照旧再认领一次（每连一次从头来）。认领成了再订会话表和提醒（`notices`），见 [`notices`]；老核心不认的
//! 不发：这时网页还在照原来的弹，托盘再弹就重了。
//!
//! 「没有响应」：问出去的（握手、订阅、读配置）过了 [`rpc::SLOW`] 还没回报 [`Report::Slow`]，接着等，回了再报 [`Report::Up`]。连着、
//! 托盘不问什么的时候不知道核心卡没卡住：不为这个定时去问。「出错了」的一种：握手被拒（本机令牌不对、协议版本谈不拢），报
//! [`Report::Refused`] 带核心的原话。

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use miyu_client::places::DataRoot;
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde_json::{Value, json};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

use crate::about;
use crate::alerts::Note;
use crate::extras::{self, Entry, Packages};
use crate::log;
use crate::rpc::{self, Failure, Rpc};

/// 握手时报的头。
pub const KIND: &str = "miyu-applet";
/// 文件一变不马上连：核心起来那几步（换令牌、听套接字、写位置）一连串，等它们落定。
const SETTLE: Duration = Duration::from_millis(150);

/// 那条连接报来的。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Report {
    /// 连上了、握过手，答得上话。
    Up,
    /// 连着，问出去的迟迟不回（「没有响应」）。
    Slow,
    /// 断了或者连不上。
    Down,
    /// 连上了，握手被核心拒了：带核心的原话（照这台电脑的语言）。
    Refused(String),
    /// 软件包挂的入口（整份，照编号排）。
    Packages(Vec<Entry>),
    /// 局域网开没开；`None` 是核心不认 `web.listen`（网页软件没装）。
    Lan(Option<bool>),
    /// 核心用的网页包的版本（软件包列表里 `webui` 那一项；「关于」照它写）；`None` 是没装。
    Web(Option<String>),
    /// 弹一条系统通知。
    Note(Note),
}

/// 叫那条连接马上再试一次（托盘自己起停了核心以后）。
#[derive(Clone)]
pub struct Waker(UnboundedSender<()>);

impl Waker {
    pub fn wake(&self) {
        if self.0.send(()).is_err() {
            log::line("core link is gone, not woken");
        }
    }
}

/// 起一个线程连核心、跟着它：每连一次、每变一样报一次（不论和上次一不一样）。
pub fn spawn(root: DataRoot, report: impl Fn(Report) + Send + 'static) -> Waker {
    let (send, receive) = unbounded_channel();
    let waker = Waker(send.clone());
    let started = std::thread::Builder::new()
        .name("core-link".into())
        .spawn(move || follow(&root, send, receive, &report));
    if let Err(error) = started {
        log::line(&format!("could not start the core link: {error}"));
    }
    waker
}

/// 一次连接怎么完的。
enum Ended {
    /// 连上过，后来断了：马上再连一次。
    Closed(String),
    /// 连不上（核心没在跑）。
    Absent(String),
    /// 握手被拒。
    Refused(String),
}

fn follow(
    root: &DataRoot,
    send: UnboundedSender<()>,
    mut receive: UnboundedReceiver<()>,
    report: &dyn Fn(Report),
) {
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            log::line(&format!("no runtime for the core link: {error}"));
            return;
        }
    };
    let mut watch = match Watch::new(root, send) {
        Ok(watch) => Some(watch),
        Err(error) => {
            log::line(&format!("not watching {}: {error}", root.run().display()));
            None
        }
    };
    runtime.block_on(async {
        let mut was = None;
        loop {
            if let Some(watch) = watch.as_mut() {
                watch.rearm();
            }
            match session(root, report, &mut was).await {
                Ended::Closed(why) => {
                    log::line(&format!("core link closed: {why}"));
                    // 断了马上再连一次：核心崩了、被叫停的连不上，报「已停止」；只是这条连接断了的接着连上。
                    tokio::time::sleep(SETTLE).await;
                    drain(&mut receive);
                    continue;
                }
                Ended::Absent(why) => {
                    said(&mut was, false, &why);
                    report(Report::Down);
                }
                Ended::Refused(why) => {
                    said(&mut was, false, &format!("refused: {why}"));
                    report(Report::Refused(why));
                }
            }
            if receive.recv().await.is_none() {
                return;
            }
            tokio::time::sleep(SETTLE).await;
            drain(&mut receive);
        }
    });
}

/// 变了才记一行。
fn said(was: &mut Option<bool>, up: bool, why: &str) {
    if *was != Some(up) {
        log::line(&format!("core link up={up}: {why}"));
        *was = Some(up);
    }
}

fn drain(receive: &mut UnboundedReceiver<()>) {
    while receive.try_recv().is_ok() {}
}

/// 连一次：握手，订流，照推送跟着，直到断。
async fn session(root: &DataRoot, report: &dyn Fn(Report), was: &mut Option<bool>) -> Ended {
    let (connection, token) = match miyu_client::connect::connect(root).await {
        Ok(got) => got,
        Err(error) => return Ended::Absent(error.to_string()),
    };
    let (read, write) = tokio::io::split(connection);
    let mut rpc = Rpc::new(read, write);
    let mut slow = |late: bool| report(if late { Report::Slow } else { Report::Up });
    let hello = rpc::hello(&token, &crate::text::system_locale());
    match rpc.call("hello", hello, &mut slow).await {
        Ok(_) => {}
        Err(Failure::Refused { message, .. }) => return Ended::Refused(message),
        Err(Failure::Closed(why)) => return Ended::Closed(why),
    }
    said(was, true, "connected");
    report(Report::Up);
    let claimed = match soft(
        "notices.claim",
        rpc.call("notices.claim", json!({}), &mut slow).await,
    ) {
        Ok(claimed) => claimed.is_some(),
        Err(failure) => return Ended::Closed(failure.said().to_string()),
    };
    match streams(&mut rpc, report, &mut slow, claimed).await {
        Ok(never) => match never {},
        Err(failure) => Ended::Closed(failure.said().to_string()),
    }
}

/// 订 `packages`、`config` 两条流，照推送报软件包的入口、局域网开没开；认领了后台通知的（`claimed`）再订会话表和提醒，照它们报
/// 系统通知。只在断了时回来。
async fn streams<R: AsyncRead + Unpin, W: AsyncWrite + Unpin>(
    rpc: &mut Rpc<R, W>,
    report: &dyn Fn(Report),
    slow: &mut dyn FnMut(bool),
    claimed: bool,
) -> Result<std::convert::Infallible, Failure> {
    let mut packages = Packages::default();
    subscribe_packages(rpc, &mut packages, report, slow).await?;
    follow_config(rpc, report, slow).await?;
    let mut notices = if claimed {
        Some(notices::Follow::start(rpc, slow).await?)
    } else {
        log::line("background notices not claimed, the web pages keep showing them");
        None
    };
    loop {
        let push = next_push(rpc, &mut notices, report, slow).await?;
        if let Some(follow) = notices.as_mut() {
            follow.push(rpc, &push, report, slow).await?;
        }
        match push["method"].as_str().unwrap_or_default() {
            "package.changed" => {
                let installed_or_removed = packages.change(&push["params"]);
                report(Report::Packages(packages.entries()));
                report(Report::Web(packages.version(about::WEB_PACKAGE)));
                // 装卸包不推 `config.changed`：网页软件装上、卸掉了，`web.listen` 也跟着有没有。
                if installed_or_removed {
                    read_lan(rpc, report, slow).await?;
                }
            }
            "config.changed" if extras::touches_listen(&push["params"]["keys"]) => {
                read_lan(rpc, report, slow).await?;
            }
            "resync" => match push["params"]["stream"].as_str() {
                Some("packages") => subscribe_packages(rpc, &mut packages, report, slow).await?,
                Some("config") => follow_config(rpc, report, slow).await?,
                _ => {}
            },
            _ => {}
        }
    }
}

/// 等下一条推来的；有在等标题的通知的，到点了先把它们照第一句发掉（等的时候丢下读了一半的那一行不丢字，见 `rpc`）。
async fn next_push<R: AsyncRead + Unpin, W: AsyncWrite + Unpin>(
    rpc: &mut Rpc<R, W>,
    notices: &mut Option<notices::Follow>,
    report: &dyn Fn(Report),
    slow: &mut dyn FnMut(bool),
) -> Result<Value, Failure> {
    loop {
        let Some((follow, at)) = notices
            .as_mut()
            .and_then(|follow| follow.next_due().map(|at| (follow, at)))
        else {
            return rpc.push().await;
        };
        let wait = at.saturating_duration_since(Instant::now());
        match tokio::time::timeout(wait, rpc.push()).await {
            Ok(push) => return push,
            Err(_) => follow.due(rpc, report, slow).await?,
        }
    }
}

/// 拒了的不算断：记一行，交 `None`（老核心不认、网页软件没装），这条连接接着用。
fn soft(method: &str, got: Result<Value, Failure>) -> Result<Option<Value>, Failure> {
    match got {
        Ok(value) => Ok(Some(value)),
        Err(Failure::Refused { reason, message }) => {
            log::line(&format!("{method} refused: {reason} {message}"));
            Ok(None)
        }
        Err(closed) => Err(closed),
    }
}

async fn subscribe_packages<R: AsyncRead + Unpin, W: AsyncWrite + Unpin>(
    rpc: &mut Rpc<R, W>,
    packages: &mut Packages,
    report: &dyn Fn(Report),
    slow: &mut dyn FnMut(bool),
) -> Result<(), Failure> {
    let params = json!({"stream": "packages"});
    if let Some(reply) = soft(
        "subscribe packages",
        rpc.call("subscribe", params, slow).await,
    )? {
        packages.replace(&reply);
        report(Report::Packages(packages.entries()));
        report(Report::Web(packages.version(about::WEB_PACKAGE)));
    }
    Ok(())
}

async fn follow_config<R: AsyncRead + Unpin, W: AsyncWrite + Unpin>(
    rpc: &mut Rpc<R, W>,
    report: &dyn Fn(Report),
    slow: &mut dyn FnMut(bool),
) -> Result<(), Failure> {
    let params = json!({"stream": "config"});
    soft(
        "subscribe config",
        rpc.call("subscribe", params, slow).await,
    )?;
    read_lan(rpc, report, slow).await
}

async fn read_lan<R: AsyncRead + Unpin, W: AsyncWrite + Unpin>(
    rpc: &mut Rpc<R, W>,
    report: &dyn Fn(Report),
    slow: &mut dyn FnMut(bool),
) -> Result<(), Failure> {
    let params = json!({"keys": [extras::LISTEN]});
    let reply = soft("config.get", rpc.call("config.get", params, slow).await)?;
    report(Report::Lan(reply.as_ref().and_then(extras::lan_of)));
    Ok(())
}

/// 盯着 `run/`：没有的盯着数据根，数据根也没有的盯着它的上一级（第一次装上、核心还没起过）。每连一次之前重新看该盯哪里。
struct Watch {
    watcher: RecommendedWatcher,
    root: PathBuf,
    run: PathBuf,
    on: Option<PathBuf>,
}

impl Watch {
    fn new(root: &DataRoot, send: UnboundedSender<()>) -> notify::Result<Watch> {
        let inside = root.path().to_path_buf();
        let watcher =
            notify::recommended_watcher(move |event: notify::Result<notify::Event>| match event {
                Ok(event) if relevant(&event.kind, &event.paths, &inside) => {
                    if send.send(()).is_err() {
                        log::now_and_then(
                            "watch after link",
                            "core link is gone, change not passed on",
                        );
                    }
                }
                Ok(_) => {}
                Err(error) => log::now_and_then("watch error", &format!("watching run/: {error}")),
            })?;
        Ok(Watch {
            watcher,
            root: root.path().to_path_buf(),
            run: root.run(),
            on: None,
        })
    }

    fn rearm(&mut self) {
        let want = target(&self.run, &self.root, Path::is_dir);
        if want == self.on {
            return;
        }
        if let Some(old) = self.on.take()
            && let Err(error) = self.watcher.unwatch(&old)
        {
            log::line(&format!("unwatching {}: {error}", old.display()));
        }
        if let Some(want) = want {
            match self.watcher.watch(&want, RecursiveMode::NonRecursive) {
                Ok(()) => self.on = Some(want),
                Err(error) => log::line(&format!("watching {}: {error}", want.display())),
            }
        }
    }
}

/// 该盯哪个目录：`run`，没有的退到数据根 `root`，再没有的退到它的上一级；都没有的不盯。
fn target(run: &Path, root: &Path, is_dir: impl Fn(&Path) -> bool) -> Option<PathBuf> {
    [Some(run), Some(root), root.parent()]
        .into_iter()
        .flatten()
        .find(|dir| is_dir(dir))
        .map(Path::to_path_buf)
}

/// 这一下变动要不要再连：数据根里（含数据根本身）有东西新建、改了、删了。只是打开、读的不算（inotify 会报，托盘自己连核心
/// 时读令牌也会报，算了就停不下来）。
fn relevant(kind: &EventKind, paths: &[PathBuf], root: &Path) -> bool {
    matches!(
        kind,
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) | EventKind::Any
    ) && paths.iter().any(|path| path.starts_with(root))
}

mod notices;
#[cfg(test)]
mod tests;
