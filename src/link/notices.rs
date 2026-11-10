//! 托盘那条连接上的系统通知（图纸 `app.md` 第 5 条，施工 AP-4）：认领了后台通知（`notices.claim`，核心 TR-1）的连接订会话表
//! （`sessions`）和账号级的提醒（`notices`），照它们报 [`Report::Note`]，弹不弹、写什么在 `alerts` 里定。掉了队（`resync`）的
//! 重新订：会话表整份换上，掉队那一段里的变化不补；提醒本来就不补。
//!
//! 会话表只说「在等人」（`waiting`），不说在问还是要确认：冒出来的时候读那个会话最新一轮的事件（`view.page {turns: 1}`）看。
//!
//! 通知照会话的人格写标题、带头像（AP-20）：报之前照会话表那一项的 `persona` 问 `persona.get`（名字、头像的版本），有头像的再问
//! `persona.avatar` 读图；头像照版本记着，同一个版本不再读。问不到的（老核心、人格删了）照无人格的写。

use std::collections::HashMap;
use std::time::Instant;

use serde_json::{Value, json};
use tokio::io::{AsyncRead, AsyncWrite};

use super::{Report, soft};
use crate::alerts::{self, Avatar, Face, Note, Rise, Sessions};
use crate::rpc::{Failure, Rpc};

/// 跟着会话表、提醒的那一份。
#[derive(Default)]
pub(super) struct Follow {
    sessions: Sessions,
    /// 读过的头像，照人格编号记（换了头像的版本对不上，再读一次）。
    avatars: HashMap<String, Avatar>,
}

impl Follow {
    /// 订会话表和提醒。订不上的（老核心）记一行，这条连接照用。
    pub(super) async fn start<R: AsyncRead + Unpin, W: AsyncWrite + Unpin>(
        rpc: &mut Rpc<R, W>,
        slow: &mut dyn FnMut(bool),
    ) -> Result<Self, Failure> {
        let mut follow = Self::default();
        follow.subscribe_sessions(rpc, slow).await?;
        subscribe_notices(rpc, slow).await?;
        Ok(follow)
    }

    async fn subscribe_sessions<R: AsyncRead + Unpin, W: AsyncWrite + Unpin>(
        &mut self,
        rpc: &mut Rpc<R, W>,
        slow: &mut dyn FnMut(bool),
    ) -> Result<(), Failure> {
        let params = json!({"stream": "sessions"});
        if let Some(reply) = soft(
            "subscribe sessions",
            rpc.call("subscribe", params, slow).await,
        )? {
            self.sessions.replace(&reply);
        }
        Ok(())
    }

    /// 最早一条在等标题的等到什么时候。
    pub(super) fn next_due(&self) -> Option<Instant> {
        self.sessions.next_due()
    }

    /// 到点了还没等到标题的，照第一句发。
    pub(super) async fn due<R: AsyncRead + Unpin, W: AsyncWrite + Unpin>(
        &mut self,
        rpc: &mut Rpc<R, W>,
        report: &dyn Fn(Report),
        slow: &mut dyn FnMut(bool),
    ) -> Result<(), Failure> {
        for note in self.sessions.due(Instant::now()) {
            self.send(rpc, note, report, slow).await?;
        }
        Ok(())
    }

    /// 照会话的人格填上名字、头像（[`Follow::face`]），报出去。
    async fn send<R: AsyncRead + Unpin, W: AsyncWrite + Unpin>(
        &mut self,
        rpc: &mut Rpc<R, W>,
        mut note: Note,
        report: &dyn Fn(Report),
        slow: &mut dyn FnMut(bool),
    ) -> Result<(), Failure> {
        if let Some(persona) = note.persona.clone() {
            note.face = self.face(rpc, &persona, slow).await?;
        }
        report(Report::Note(note));
        Ok(())
    }

    /// 人格在通知上的样子：`persona.get` 的名字（没有的照编号）、头像（`avatar` 是版本，图照 `persona.avatar` 读，同一个版本记着不再读）。
    /// 问不到的（老核心、人格删了，`soft` 记一行）是 `None`。
    async fn face<R: AsyncRead + Unpin, W: AsyncWrite + Unpin>(
        &mut self,
        rpc: &mut Rpc<R, W>,
        persona: &str,
        slow: &mut dyn FnMut(bool),
    ) -> Result<Option<Face>, Failure> {
        let params = json!({"persona": persona});
        let Some(got) = soft(
            "persona.get",
            rpc.call("persona.get", params.clone(), slow).await,
        )?
        else {
            return Ok(None);
        };
        let name = got["name"]
            .as_str()
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .unwrap_or(persona)
            .to_owned();
        let Some(version) = got["avatar"].as_str() else {
            self.avatars.remove(persona);
            return Ok(Some(Face { name, avatar: None }));
        };
        if let Some(known) = self
            .avatars
            .get(persona)
            .filter(|known| known.version == version)
        {
            let avatar = Some(known.clone());
            return Ok(Some(Face { name, avatar }));
        }
        let read = soft(
            "persona.avatar",
            rpc.call("persona.avatar", params, slow).await,
        )?;
        let avatar = read.as_ref().and_then(|read| avatar_of(read, version));
        match &avatar {
            Some(avatar) => {
                self.avatars.insert(persona.to_owned(), avatar.clone());
            }
            None => {
                self.avatars.remove(persona);
            }
        }
        Ok(Some(Face { name, avatar }))
    }

    /// 推来的一条：会话表的变动、提醒、掉了队。别的不理。
    pub(super) async fn push<R: AsyncRead + Unpin, W: AsyncWrite + Unpin>(
        &mut self,
        rpc: &mut Rpc<R, W>,
        push: &Value,
        report: &dyn Fn(Report),
        slow: &mut dyn FnMut(bool),
    ) -> Result<(), Failure> {
        let params = &push["params"];
        match push["method"].as_str().unwrap_or_default() {
            "sessions.changed" => {
                let changed = self.sessions.change(params);
                for note in changed.ready {
                    self.send(rpc, note, report, slow).await?;
                }
                let note = match changed.rise {
                    Some(Rise::Done(id)) => {
                        self.sessions.note(&id, alerts::Why::Done, Instant::now())
                    }
                    Some(Rise::Waiting(id)) => {
                        let page = json!({"session": id, "turns": 1});
                        let got = soft("view.page", rpc.call("view.page", page, slow).await)?;
                        let why = alerts::waiting_why(&got.unwrap_or_default());
                        self.sessions.note(&id, why, Instant::now())
                    }
                    None => None,
                };
                if let Some(note) = note {
                    self.send(rpc, note, report, slow).await?;
                }
            }
            "notice" => {
                let prompt = match self.sessions.wants_prompt(params) {
                    Some(schedule) => prompt_of(rpc, &schedule, slow).await?,
                    None => None,
                };
                if let Some(note) = self.sessions.notice(params, prompt.as_deref()) {
                    self.send(rpc, note, report, slow).await?;
                }
            }
            "resync" => match params["stream"].as_str() {
                Some("sessions") => self.subscribe_sessions(rpc, slow).await?,
                Some("notices") => subscribe_notices(rpc, slow).await?,
                _ => {}
            },
            _ => {}
        }
        Ok(())
    }
}

/// 订提醒。回应里的 `claimed`（有没有人认领）托盘不用看：认领的就是它自己。
async fn subscribe_notices<R: AsyncRead + Unpin, W: AsyncWrite + Unpin>(
    rpc: &mut Rpc<R, W>,
    slow: &mut dyn FnMut(bool),
) -> Result<(), Failure> {
    let params = json!({"stream": "notices"});
    soft(
        "subscribe notices",
        rpc.call("subscribe", params, slow).await,
    )?;
    Ok(())
}

/// 定时任务那句话（会话表里还没有那个会话的标题时当标题）：照 `schedule.list` 找；找不到的是 `None`。
async fn prompt_of<R: AsyncRead + Unpin, W: AsyncWrite + Unpin>(
    rpc: &mut Rpc<R, W>,
    schedule: &str,
    slow: &mut dyn FnMut(bool),
) -> Result<Option<String>, Failure> {
    let list = soft(
        "schedule.list",
        rpc.call("schedule.list", json!({}), slow).await,
    )?;
    Ok(list.and_then(|list| {
        list["schedules"]
            .as_array()?
            .iter()
            .find(|entry| entry["id"] == schedule)?["prompt"]
            .as_str()
            .map(str::to_owned)
    }))
}
/// `persona.avatar` 的回应（`{"avatar", "media_type", "data"}`，`data` 是 base64）读成头像；版本对不上、不是这三种图、解不开的是 `None`。
fn avatar_of(read: &Value, version: &str) -> Option<Avatar> {
    if read["avatar"].as_str().is_some_and(|got| got != version) {
        return None;
    }
    let ext = match read["media_type"].as_str()? {
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/webp" => "webp",
        _ => return None,
    };
    let bytes = base64(read["data"].as_str()?)?;
    Some(Avatar {
        version: version.to_owned(),
        ext,
        bytes,
    })
}

/// 标准 base64（带 `=` 补齐、不带换行）解开；有别的字的是 `None`。
fn base64(text: &str) -> Option<Vec<u8>> {
    let value = |c: u8| -> Option<u32> {
        Some(u32::from(match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        }))
    };
    let text = text.trim_end_matches('=').as_bytes();
    let mut out = Vec::with_capacity(text.len() * 3 / 4);
    for chunk in text.chunks(4) {
        if chunk.len() == 1 {
            return None;
        }
        let mut bits = 0u32;
        for (at, &c) in chunk.iter().enumerate() {
            bits |= value(c)? << (18 - 6 * at);
        }
        let bytes = bits.to_be_bytes();
        out.extend_from_slice(&bytes[1..chunk.len()]);
    }
    Some(out)
}

#[cfg(test)]
mod tests;
