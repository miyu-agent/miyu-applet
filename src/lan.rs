//! 「局域网访问」点了（图纸 `app.md` 第 4 条「局域网」）：临时连一次核心（不拉起），照网页设置页的做法改 `web.listen`：打开写
//! `0.0.0.0`，关掉恢复默认（`unset`，出厂的 `127.0.0.1`）。网页软件自己跟着配置换监听（网页 `follow.rs`），托盘那条连接收到
//! `config.changed` 再读一遍、改打勾。
//!
//! 打开之前先看设没设访问密码：握手回应的 `password_set`（核心 SU-1 起有；核心参考 `connection.md`「hello」：「头照它决定『允许局域网
//! 访问』开不开得了」）。每次点都现连现问，不记着：密码是在网页里设的，托盘收不到推送。没设的、老核心不说的，不改配置，交回
//! [`Outcome::NeedsPassword`]，由界面打开网页设置「访问」那一组先设密码。

use std::time::Duration;

use miyu_client::places::DataRoot;
use serde_json::{Value, json};

use crate::extras;
use crate::rpc::{self, Rpc};

/// 连、问、改一共最多等多久。
const LIMIT: Duration = Duration::from_secs(30);

/// 点了以后怎样。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// 改好了。
    Done,
    /// 要打开，可是还没设访问密码（老核心不说设没设的也算）：没改。
    NeedsPassword,
}

/// 没改成。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Failed {
    /// 过了 [`LIMIT`] 还没做完。
    Timeout,
    /// 连不上核心、握手被拒、核心不收：带为什么（核心的原话）。
    Said(String),
}

/// 照 `on` 打开、关掉局域网访问。
///
/// # Errors
/// 连不上核心、握手被拒、核心不收、等太久。
pub fn set(root: &DataRoot, on: bool) -> Result<Outcome, Failed> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| Failed::Said(error.to_string()))?;
    runtime.block_on(async {
        tokio::time::timeout(LIMIT, change(root, on))
            .await
            .unwrap_or(Err(Failed::Timeout))
    })
}

async fn change(root: &DataRoot, on: bool) -> Result<Outcome, Failed> {
    // 连不上的照托盘的语言写核心给的那一句（核心 EN-1），没说法的照原话。
    let lang = crate::text::Lang::of_locale(&crate::text::system_locale());
    let (connection, token) = miyu_client::connect::connect(root).await.map_err(|error| {
        Failed::Said(crate::reason::say(error.said(), &error.to_string(), lang))
    })?;
    let (read, write) = tokio::io::split(connection);
    let mut rpc = Rpc::new(read, write);
    // 这里慢不慢不用报：外面整个限了时。
    let mut slow = |_: bool| {};
    let said = |failure: rpc::Failure| Failed::Said(failure.said().to_string());
    let hello = rpc::hello(&token, &crate::text::system_locale());
    let shaken = rpc.call("hello", hello, &mut slow).await.map_err(said)?;
    if on && !password_set(&shaken) {
        return Ok(Outcome::NeedsPassword);
    }
    rpc.call("config.set", change_params(on), &mut slow)
        .await
        .map_err(said)?;
    Ok(Outcome::Done)
}

/// 握手回应说设过访问密码没有；没说的（老核心）当没设。
fn password_set(hello: &Value) -> bool {
    hello["password_set"] == Value::Bool(true)
}

/// `config.set` 的参数：打开写 `0.0.0.0`，关掉恢复默认（同网页设置页）。
fn change_params(on: bool) -> Value {
    let change = if on {
        json!({"key": extras::LISTEN, "value": extras::ANY})
    } else {
        json!({"key": extras::LISTEN, "unset": true})
    };
    json!({"changes": [change]})
}

#[cfg(test)]
mod tests;
