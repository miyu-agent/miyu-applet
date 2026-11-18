//! 单实例（图纸 `app.md` 第 1 条）：一个数据根只跑一个托盘。第二个起来的把「打开网页界面」交给已有的那一个，自己退；点通知拉起来的
//! （`miyu-applet --open miyu-applet://open/?session=<编号>`，[`crate::protocol`]）交「打开那个会话」。
//!
//! 托盘自己的目录（只给自己看，见 [`crate::folder`]）里一把锁、一个「请打开」的文件：拿到锁的是第一个，它盯着那个文件；拿不到的
//! 往文件里写一行就退。锁是系统的文件锁，进程没了系统就放，崩了不留死锁。不用本机套接字：Linux 的抽象套接字谁都连得上、谁都能抢先
//! 占名字，放在只给自己看的目录里的文件没有这个问题，盯文件的也是看核心 `run/` 的同一个库。

use std::fs::{File, OpenOptions, TryLockError};
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use crate::{folder, log};

/// 认数据根的那一串：数据根路径的 FNV-1a 哈希，十六进制。同一个程序算出来的一样，换了数据根（`MIYU_HOME`）的托盘各跑各的。
pub fn key(root: &Path) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in root.as_os_str().as_encoded_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{hash:016x}")
}

/// 抢单实例的结果。
pub enum Claim {
    /// 是第一个：拿着这把锁直到退出。
    First(#[allow(dead_code, reason = "只是拿着：丢了锁就放了")] File),
    /// 已经有一个在跑。
    Second,
}

/// 抢锁：`dir` 是托盘自己的目录，建不出的照常建。
///
/// # Errors
/// 建不了目录、打不开锁文件、拿锁出错。
pub fn claim(dir: &Path, key: &str) -> io::Result<Claim> {
    folder::private(dir)?;
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(dir.join(format!("{key}.lock")))?;
    match file.try_lock() {
        Ok(()) => Ok(Claim::First(file)),
        Err(TryLockError::WouldBlock) => Ok(Claim::Second),
        Err(TryLockError::Error(error)) => Err(error),
    }
}

/// 「请打开」的那个文件。
fn request(dir: &Path, key: &str) -> PathBuf {
    dir.join(format!("{key}.open"))
}

/// 第二个起来的：叫第一个打开网页界面，`session` 有的打开那个会话。每次写的不一样（带时间），文件本来就有的也照样算改了。
/// 先写旁边的临时文件（名字带自己的进程号，几个同时起来的不撞）再改名盖过去：盯着的读不到写了一半的（审计 #098）。
///
/// # Errors
/// 写不进去。
pub fn ask(dir: &Path, key: &str, session: Option<&str>) -> io::Result<()> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let line = match session {
        Some(id) => format!("session {id} {}\n", now.as_nanos()),
        None => format!("open {}\n", now.as_nanos()),
    };
    let temporary = dir.join(format!("{key}.open.{}.tmp", std::process::id()));
    std::fs::write(&temporary, line)?;
    std::fs::rename(&temporary, request(dir, key)).inspect_err(|_| {
        if let Err(error) = std::fs::remove_file(&temporary) {
            log::line(&format!("leftover {}: {error}", temporary.display()));
        }
    })
}

/// 读「请打开」那一行：`session <编号> …` 的交回那个会话，别的（`open …`、读不了的）是打开网页界面。
fn wanted_session(line: &str) -> Option<String> {
    let mut words = line.split_whitespace();
    match (words.next(), words.next()) {
        (Some("session"), Some(id)) => {
            crate::protocol::parse(&crate::protocol::link(Some(id))).flatten()
        }
        _ => None,
    }
}

/// 第一个：盯着「请打开」的文件，有人写了就叫 `asked`（带上要打开的会话，没有的是打开网页界面）。交回的盯着的东西要一直拿着。
/// 写一次文件系统会报好几下：同一行只交一回，读不到的、空的不交（审计 #098）。
///
/// # Errors
/// 盯不了。
pub fn listen(
    dir: &Path,
    key: &str,
    asked: impl Fn(Option<String>) + Send + 'static,
) -> notify::Result<RecommendedWatcher> {
    let wanted = request(dir, key);
    let mut last = String::new();
    let mut watcher =
        notify::recommended_watcher(move |event: notify::Result<notify::Event>| match event {
            Ok(event) if written(&event.kind) && event.paths.contains(&wanted) => {
                match std::fs::read_to_string(&wanted) {
                    Ok(line) if !line.is_empty() && line != last => {
                        asked(wanted_session(&line));
                        last = line;
                    }
                    Ok(_) => {}
                    Err(error) => {
                        log::now_and_then("instance read", &format!("reading the request: {error}"))
                    }
                }
            }
            Ok(_) => {}
            Err(error) => {
                log::now_and_then("instance watch", &format!("watching requests: {error}"))
            }
        })?;
    watcher.watch(dir, RecursiveMode::NonRecursive)?;
    Ok(watcher)
}

/// 写了东西的那几种变动（打开、读的不算）。
fn written(kind: &EventKind) -> bool {
    matches!(
        kind,
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Any
    )
}

/// macOS 上点程序坞的图标可能连着来：一秒里只算一次。带会话的（点了通知）不压，第二个起来的交来的已经一回一交（[`listen`]）。
#[derive(Debug, Default)]
pub struct Once {
    last: Option<Instant>,
}

/// 多久里只算一次。
const ONCE_IN: Duration = Duration::from_secs(1);

impl Once {
    /// 要打开 `session`（`None` 是网页界面）的这一下算不算：带会话的都算。
    pub fn pass(&mut self, now: Instant, session: Option<&str>) -> bool {
        session.is_some() || self.allow(now)
    }

    /// 这一下算不算。
    pub fn allow(&mut self, now: Instant) -> bool {
        if self
            .last
            .is_some_and(|last| now.saturating_duration_since(last) < ONCE_IN)
        {
            return false;
        }
        self.last = Some(now);
        true
    }
}

#[cfg(test)]
mod tests;
