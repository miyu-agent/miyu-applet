//! 在等人点的通知（审计 #100）：一条一个后台线程等通知服务回话，通知服务不说「关掉了」（mako 默认不过期、没人点就一直开着）的时候，
//! 线程和它那条 D-Bus 连接一直留着。这里记着哪些还在等，多了叫最早的那几个别等了：
//! - 同一个 `tag` 的下一条换掉了上一条（Linux 带上一条的编号发），上一条不用再等：点了是同一个编号，新的那一条等得到；
//! - 一共最多 [`LIMIT`] 条在等，再多的叫最早的那一条别等了（它还在屏幕上，点了不再打开会话）。
//!
//! 叫停只在等得停的平台上有用（Linux 照异步的等法，`toast::platform::wait`）；macOS 的库等的时候停不下来，照旧等到人点、关。

use std::collections::VecDeque;

use tokio::sync::oneshot;

/// 一共最多几条在等。
pub const LIMIT: usize = 32;

/// 一张等着的票：交回 [`Waits::done`] 时认它。
pub type Ticket = u64;

/// 在等的那几条，照先后。
#[derive(Debug, Default)]
pub struct Waits {
    next: Ticket,
    open: VecDeque<(Ticket, String, oneshot::Sender<()>)>,
}

impl Waits {
    /// `tag` 的一条开始等：同一个 `tag` 在等的叫停，多过 [`LIMIT`] 的叫停最早的。交回票和「别等了」的那一头。
    pub fn start(&mut self, tag: &str) -> (Ticket, oneshot::Receiver<()>) {
        let mut kept = VecDeque::with_capacity(self.open.len() + 1);
        for (ticket, open, stop) in self.open.drain(..) {
            if open == tag {
                halt(stop);
            } else {
                kept.push_back((ticket, open, stop));
            }
        }
        self.open = kept;
        while self.open.len() >= LIMIT {
            if let Some((_, _, stop)) = self.open.pop_front() {
                halt(stop);
            }
        }
        self.next += 1;
        let (stop, stopped) = oneshot::channel();
        self.open.push_back((self.next, tag.to_owned(), stop));
        (self.next, stopped)
    }

    /// 这一张不等了（人点了、关了，或者被叫停了）。
    pub fn done(&mut self, ticket: Ticket) {
        self.open.retain(|(open, _, _)| *open != ticket);
    }

    /// 还有几条在等。
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.open.len()
    }
}

/// 叫一条别等了：它已经自己等完、走了的，没人收也不要紧。
fn halt(stop: oneshot::Sender<()>) {
    if stop.send(()).is_err() {
        // 那一条已经不在等了。
    }
}

#[cfg(test)]
mod tests;
