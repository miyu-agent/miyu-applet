//! 核心起不来、连不上的原因写给人看（核心 LC-1、EN-1、EN-2）：核心的错交说法（`said()`，`core/start/<代码>` 和要填的几样；
//! EN-2 起核心起不来的每一步都带代码，原因只是会变的那一段），照核心资源目录里的字、托盘的语言写整句；没说法的（读写出错）、
//! 资源目录找不到、那一句没有的，照原话。原来只有数据根认不出的三种照说法，别的照原话写死中文（EN-2 测出来的）。

use miyu_client::connect::StartError;
use miyu_client::places::{Env, ResourceRoot};
use miyu_client::protocol::Said;
use miyu_client::texts::Human;

use crate::text::Lang;

/// 有 `said` 的照说法写，别的照 `fallback`（错的原话）。
pub fn say(said: Option<Said>, fallback: &str, lang: Lang) -> String {
    say_in(&Env::current(), said, fallback, lang)
}

/// 拉起核心没成（[`StartError`]）：照说法写整句，没说法的照原话。
pub fn start_failed(error: &StartError, lang: Lang) -> String {
    start_failed_in(&Env::current(), error, lang)
}

/// 同 [`start_failed`]，资源目录照 `env` 找。
fn start_failed_in(env: &Env, error: &StartError, lang: Lang) -> String {
    say_in(env, error.said(), &error.to_string(), lang)
}

/// 同 [`say`]，资源目录照 `env` 找。
fn say_in(env: &Env, said: Option<Said>, fallback: &str, lang: Lang) -> String {
    said.and_then(|said| spoken(env, &said, lang))
        .unwrap_or_else(|| fallback.to_owned())
}

/// 照说法找那一句；资源目录、那种语言的字、那一句、要填的有一样没有的是空的。
fn spoken(env: &Env, said: &Said, lang: Lang) -> Option<String> {
    let resources = ResourceRoot::locate(env).ok()?;
    let human = Human::load(&resources, lang.code()).ok()?;
    human.say(said)
}

#[cfg(test)]
mod tests;
