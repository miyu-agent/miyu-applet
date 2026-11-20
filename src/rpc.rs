//! 和核心一问一答（核心协议「一行一条」，`docs/zh/reference/connection.md`）：一行一条 JSON，问的带编号，回应照编号对上，推来的没有
//! 编号、先攒着。托盘那条一直连着的连接（`link`）和「局域网访问」点了临时连的那一条（`lan`）都用它。
//!
//! 编号每条连接取一个随机的前缀、后面接递增的数字：核心照编号认重发，不分连接，用过的编号再发拿到的是上一次的回应、什么都不做。
//!
//! 问出去以后迟迟不回（[`SLOW`]）的告诉调的人一声「问不出话」，接着等：核心忙过来了照旧回，回了再说一声；连接断了就是断了。不另定时
//! 去问。

use std::collections::VecDeque;
use std::hash::{BuildHasher, RandomState};
use std::time::Duration;

use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader};

/// 问出去多久没回算「问不出话」（状态行写「没有响应」）：核心平常几毫秒就回，忙着写盘、整理的时候也在一两秒里。
pub const SLOW: Duration = Duration::from_secs(10);

/// 一行最多攒多少字节：协议一行最长 1 MiB（核心 `protocol.md`「一行一条」），留足余量；过了的当断了，不跟着攒内存（审计 #100）。
const LINE_LIMIT: usize = 16 * 1024 * 1024;

/// 一问没问成。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Failure {
    /// 连接断了、读写出错。带为什么。
    Closed(String),
    /// 核心拒了：`reason` 是给程序看的原因码（`data.reason`），`message` 是给人看的那一句（照连接的语言）。
    Refused { reason: String, message: String },
}

impl Failure {
    /// 给日志、给人看的一句。
    pub fn said(&self) -> &str {
        match self {
            Failure::Closed(why) => why,
            Failure::Refused { message, .. } => message,
        }
    }
}

/// 一条到核心的连接，一行一条。
pub struct Rpc<R, W> {
    lines: BufReader<R>,
    write: W,
    prefix: String,
    next: u64,
    /// 读了一半的那一行：等回应时过了 [`SLOW`] 会丢下正在读的，读到的留在这里，接着读。
    partial: Vec<u8>,
    /// 等回应时推来的，[`Rpc::push`] 先交这些。
    pushes: VecDeque<Value>,
}

/// 读到的一行是什么。
#[derive(Debug, PartialEq)]
enum Line {
    /// 带编号的：回应。
    Reply(Value),
    /// 没编号、有方法名的：推来的。
    Push(Value),
    /// 读不懂的：不理。
    Other,
}

fn classify(line: &str) -> Line {
    let Ok(message) = serde_json::from_str::<Value>(line) else {
        return Line::Other;
    };
    if message.get("id").is_some_and(|id| !id.is_null()) {
        Line::Reply(message)
    } else if message["method"].is_string() {
        Line::Push(message)
    } else {
        Line::Other
    }
}

/// 回应里交回结果，或者核心拒了的那两样。
fn answer(reply: &Value) -> Result<Value, Failure> {
    match reply.get("result") {
        Some(result) => Ok(result.clone()),
        None => Err(Failure::Refused {
            reason: reply["error"]["data"]["reason"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            message: reply["error"]["message"]
                .as_str()
                .unwrap_or("refused")
                .to_string(),
        }),
    }
}

/// 这条连接的编号前缀：系统给的随机数。
fn prefix() -> String {
    format!(
        "app-{:016x}",
        RandomState::new().hash_one(std::process::id())
    )
}

impl<R: AsyncRead + Unpin, W: AsyncWrite + Unpin> Rpc<R, W> {
    pub fn new(read: R, write: W) -> Self {
        Self {
            lines: BufReader::new(read),
            write,
            prefix: prefix(),
            next: 0,
            partial: Vec::new(),
            pushes: VecDeque::new(),
        }
    }

    /// 问一句，等它的回应。过了 [`SLOW`] 还没回的调一次 `slow(true)`，后来回了再调一次 `slow(false)`。
    ///
    /// # Errors
    /// 断了、读写出错；核心拒了。
    pub async fn call(
        &mut self,
        method: &str,
        params: Value,
        slow: &mut dyn FnMut(bool),
    ) -> Result<Value, Failure> {
        self.next += 1;
        let id = format!("{}-{}", self.prefix, self.next);
        let request = json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params});
        self.send(&request).await?;
        let mut late = false;
        loop {
            let line = if late {
                self.line().await?
            } else {
                match tokio::time::timeout(SLOW, self.line()).await {
                    Ok(line) => line?,
                    Err(_) => {
                        late = true;
                        slow(true);
                        continue;
                    }
                }
            };
            match classify(&line) {
                Line::Reply(reply) if reply["id"] == json!(id) => {
                    if late {
                        slow(false);
                    }
                    return answer(&reply);
                }
                Line::Push(push) => self.pushes.push_back(push),
                Line::Reply(_) | Line::Other => {}
            }
        }
    }

    /// 等下一条推来的（等回应时攒下的先交）。
    ///
    /// # Errors
    /// 断了、读写出错。
    pub async fn push(&mut self) -> Result<Value, Failure> {
        if let Some(push) = self.pushes.pop_front() {
            return Ok(push);
        }
        loop {
            if let Line::Push(push) = classify(&self.line().await?) {
                return Ok(push);
            }
        }
    }

    async fn send(&mut self, message: &Value) -> Result<(), Failure> {
        let closed = |error: std::io::Error| Failure::Closed(error.to_string());
        self.write
            .write_all(format!("{message}\n").as_bytes())
            .await
            .map_err(closed)?;
        self.write.flush().await.map_err(closed)
    }

    /// 读一行；读到头就是断了。丢下了也不丢字：读到的都先放进 `partial`。一行过了 [`LINE_LIMIT`] 还没完的当断了。
    async fn line(&mut self) -> Result<String, Failure> {
        loop {
            let room = LINE_LIMIT.saturating_sub(self.partial.len());
            if room == 0 {
                self.partial.clear();
                return Err(Failure::Closed("line too long".into()));
            }
            // 一次最多读到还放得下的那么多（`take` 不攒，读到的照旧先进 `partial`）。
            let mut limited = (&mut self.lines).take(room as u64);
            match limited.read_until(b'\n', &mut self.partial).await {
                Ok(_) if self.partial.ends_with(b"\n") => {
                    let line = String::from_utf8_lossy(&self.partial).into_owned();
                    self.partial.clear();
                    return Ok(line);
                }
                Ok(0) => return Err(Failure::Closed("disconnected".into())),
                Ok(_) => {}
                Err(error) => return Err(Failure::Closed(error.to_string())),
            }
        }
    }
}

/// 握手那一句的参数：出示本机令牌，报自己是托盘（同核心门面 `Core::shake` 的写法）。
pub fn hello(token: &str, locale: &str) -> Value {
    json!({
        "protocol": [1, 1],
        "token": token,
        "head": {"kind": crate::link::KIND, "version": env!("CARGO_PKG_VERSION")},
        "locale": locale,
    })
}

#[cfg(test)]
mod tests;
