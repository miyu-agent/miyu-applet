//! 系统通知发什么（图纸 `app.md` 第 5 条，施工 AP-4）：纯的，连接在 `link::notices` 里。托盘认领了后台通知（核心 TR-1）以后，
//! 没有网页开着的会话（会话表那一项 `viewed` 不为真）回答完了、在问、要确认，和账号级的提醒（`notices` 流），都由托盘弹。
//!
//! 什么时候、写什么照网页的 `notify` 包（网页 `web.md`「系统通知」第 1、2、6、7 条，两边一样）：
//! - 会话表推来的一项和手上的比：`busy` 去掉了是「回答完了」，`waiting` 冒出来是「等你回答」或「等你确认」（哪一种会话表不说，
//!   照那一轮的事件看，见 [`waiting_why`]）；读进来、重连时换上的整份不算（不会把旧的再报一遍）；子会话、一次性的会话、整理记忆的
//!   会话不弹。
//! - 标题、图标照会话的人格（AP-20，2026-11-10 项目主人：系统通知照会话的人格来，同网页 W-1143）：标题写人格的名字，人格有头像的
//!   带头像，没头像的不带图标；无人格的会话、账号级的提醒标题写「Miyu」、不带图标。人格的名字、头像连接那边照 `persona.get`、
//!   `persona.avatar` 问好（[`Face`]），这里只记会话表那一项的 `persona`。
//! - 正文是会话的标题（没有的照第一句 `preview`）接为什么通知：「修 CI · 回答完了」，都没有的只写为什么；不带 AI 写的字。新会话第一轮
//!   做完时核心起的标题多半还没到，等它最多 [`TITLE_WAIT`]，到了照标题写，没到照第一句。
//! - `tag`：同一个会话的同一种换掉上一条（`<会话>:done`、`<会话>:ask`、`<会话>:approve`）；定时任务跑完了同那个会话的「回答完了」，
//!   两条互相换掉；定时任务的记录坏了是 `schedules.reset`。

use std::collections::HashMap;
use std::time::{Duration, Instant};

use serde_json::Value;

use crate::text::key;

/// 没有标题的会话等核心起的标题最多多久（同网页）。
pub const TITLE_WAIT: Duration = Duration::from_secs(5);

/// 为什么通知。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Why {
    /// 一轮做完了。
    Done,
    /// 她在问。
    Ask,
    /// 要人确认。
    Approve,
    /// 每次新开会话的定时任务那一轮答完了。
    ScheduleRan,
    /// 那一轮出错了、被打断了。
    ScheduleFailed,
    /// 唤醒表坏了，换了一份空的。
    SchedulesReset,
}

impl Why {
    /// 正文的那一句（`resources/text/`）。
    pub fn key(self) -> &'static str {
        match self {
            Why::Done => key::NOTIFY_DONE,
            Why::Ask => key::NOTIFY_ASK,
            Why::Approve => key::NOTIFY_APPROVE,
            Why::ScheduleRan => key::NOTIFY_SCHEDULE_RAN,
            Why::ScheduleFailed => key::NOTIFY_SCHEDULE_FAILED,
            Why::SchedulesReset => key::NOTIFY_SCHEDULES_RESET,
        }
    }
}

/// 一条要弹的通知。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    /// 点了打开哪个会话；账号级的、没有会话的是 `None`（打开网页界面）。
    pub session: Option<String>,
    pub why: Why,
    /// 会话的标题（没有的照第一句、定时任务那句话），写进正文；都没有的是空的。
    pub title: String,
    /// 同一个 `tag` 的换掉上一条。
    pub tag: String,
    /// 这个会话用的人格（会话表那一项的 `persona`）；无人格的、账号级的是 `None`。
    pub persona: Option<String>,
    /// 人格的名字、头像（连接那边照 [`Note::persona`] 问好了填上）；没问到的、无人格的是 `None`。
    pub face: Option<Face>,
}

/// 一个人格在通知上的样子：名字、头像。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Face {
    pub name: String,
    pub avatar: Option<Avatar>,
}

/// 人格的头像：版本（核心给的，图的字节的 SHA-256 前 16 位）、扩展名（`png`、`jpg`、`webp`）、图本身。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Avatar {
    pub version: String,
    pub ext: &'static str,
    pub bytes: Vec<u8>,
}

/// 无人格的会话、账号级的提醒的标题：程序名。
pub const PROGRAM: &str = "Miyu";

impl Note {
    /// 通知的标题：人格的名字，没有人格（或者没问到）的写程序名。
    pub fn heading(&self) -> &str {
        self.face
            .as_ref()
            .map(|face| face.name.trim())
            .filter(|name| !name.is_empty())
            .unwrap_or(PROGRAM)
    }

    /// 通知的正文：会话的标题接为什么（`why` 是那一句的字），没有标题的只写为什么。
    pub fn body(&self, why: &str) -> String {
        if self.title.is_empty() {
            why.to_owned()
        } else {
            format!("{} · {why}", self.title)
        }
    }

    /// 要带的头像：有人格、有头像的才有。
    pub fn avatar(&self) -> Option<&Avatar> {
        self.face.as_ref()?.avatar.as_ref()
    }
}

/// 会话表里一项，只留要用的。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Seen {
    busy: bool,
    waiting: bool,
    viewed: bool,
    /// 子会话、一次性的会话、整理记忆的会话：不弹。
    quiet: bool,
    title: Option<String>,
    preview: Option<String>,
    /// 这个会话用的人格；无人格的是 `None`。
    persona: Option<String>,
}

fn seen(entry: &Value) -> Seen {
    let text = |name: &str| {
        entry[name]
            .as_str()
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(str::to_owned)
    };
    Seen {
        busy: entry["busy"] == true,
        waiting: entry["waiting"] == true,
        viewed: entry["viewed"] == true,
        quiet: entry["parent"].is_string() || entry["oneshot"] == true || entry["kind"] == "dream",
        title: text("title"),
        preview: text("preview"),
        persona: text("persona"),
    }
}

/// 会话表推来的一项让一个会话要人留意了。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Rise {
    /// 一轮做完了：直接照 [`Sessions::note`] 写。
    Done(String),
    /// 开始等人了：先看是在问还是要确认（[`waiting_why`]），再照 [`Sessions::note`] 写。
    Waiting(String),
}

/// 一条推来的会话表变动：要人留意的那一下，和因为有了标题、能发了的那几条。
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Changed {
    pub rise: Option<Rise>,
    pub ready: Vec<Note>,
}

/// 在等标题的一条。
#[derive(Debug)]
struct Held {
    note: Note,
    until: Instant,
}

/// 手上的会话表，和在等标题的几条。一条连接一份：重连时整个换新的。
#[derive(Debug, Default)]
pub struct Sessions {
    known: HashMap<String, Seen>,
    held: Vec<Held>,
}

impl Sessions {
    /// 订阅的回应（整张表 `{"sessions": […]}`）：整份换上，不弹。
    pub fn replace(&mut self, reply: &Value) {
        self.known = reply["sessions"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|entry| Some((entry["session"].as_str()?.to_owned(), seen(entry))))
            .collect();
    }

    /// 推来的一条 `sessions.changed`（`{session, entry}`，删了的 `{session, removed: true}`）。
    pub fn change(&mut self, params: &Value) -> Changed {
        let Some(id) = params["session"].as_str() else {
            return Changed::default();
        };
        if params["removed"] == true || !params["entry"].is_object() {
            self.known.remove(id);
            self.held
                .retain(|held| held.note.session.as_deref() != Some(id));
            return Changed::default();
        }
        let now = seen(&params["entry"]);
        let was = self
            .known
            .insert(id.to_owned(), now.clone())
            .unwrap_or_default();
        let ready = match &now.title {
            Some(title) => self.release(id, title),
            None => Vec::new(),
        };
        let rise = if now.quiet || now.viewed {
            None
        } else if was.busy && !now.busy {
            Some(Rise::Done(id.to_owned()))
        } else if !was.waiting && now.waiting {
            Some(Rise::Waiting(id.to_owned()))
        } else {
            None
        };
        Changed { rise, ready }
    }

    /// 有了标题：等着它的那几条照它写，交出去。
    fn release(&mut self, id: &str, title: &str) -> Vec<Note> {
        let (mine, rest) = std::mem::take(&mut self.held)
            .into_iter()
            .partition(|held| held.note.session.as_deref() == Some(id));
        self.held = rest;
        mine.into_iter()
            .map(|held: Held| Note {
                title: title.to_owned(),
                ..held.note
            })
            .collect()
    }

    /// 给会话 `id` 写一条 `why`。有标题的马上交；没有的先等着（同一个 `tag` 已经在等的不再等一条），到了标题或者到点再交。
    pub fn note(&mut self, id: &str, why: Why, now: Instant) -> Option<Note> {
        let tag = format!("{id}:{}", tag_of(why));
        let entry = self.known.get(id);
        let persona = entry.and_then(|seen| seen.persona.clone());
        if let Some(title) = entry.and_then(|seen| seen.title.clone()) {
            return Some(Note {
                session: Some(id.to_owned()),
                why,
                title,
                tag,
                persona,
                face: None,
            });
        }
        if !self.held.iter().any(|held| held.note.tag == tag) {
            let note = Note {
                session: Some(id.to_owned()),
                why,
                title: String::new(),
                tag,
                persona,
                face: None,
            };
            self.held.push(Held {
                note,
                until: now + TITLE_WAIT,
            });
        }
        None
    }

    /// 最早一条等到什么时候；没有在等的是 `None`。
    pub fn next_due(&self) -> Option<Instant> {
        self.held.iter().map(|held| held.until).min()
    }

    /// 到点了还没等到标题的：照第一句写，再没有的正文只写为什么。
    pub fn due(&mut self, now: Instant) -> Vec<Note> {
        let (late, rest) = std::mem::take(&mut self.held)
            .into_iter()
            .partition(|held| held.until <= now);
        self.held = rest;
        late.into_iter()
            .map(|held: Held| {
                let id = held.note.session.as_deref().unwrap_or_default();
                Note {
                    title: self.title(id, None),
                    ..held.note
                }
            })
            .collect()
    }

    /// 会话 `id` 的标题：标题，第一句，`fallback`（定时任务那句话），再没有是空的。
    fn title(&self, id: &str, fallback: Option<&str>) -> String {
        let seen = self.known.get(id);
        seen.and_then(|seen| seen.title.as_deref().or(seen.preview.as_deref()))
            .or(fallback.map(str::trim).filter(|text| !text.is_empty()))
            .unwrap_or_default()
            .to_owned()
    }

    /// 推来的 `notice` 要不要先查定时任务那句话：`schedule.ran` 那个会话会话表里没有标题、第一句的，交定时任务的编号。
    pub fn wants_prompt(&self, params: &Value) -> Option<String> {
        if params["kind"] != "schedule.ran" {
            return None;
        }
        let id = params["session"].as_str()?;
        let named = self
            .known
            .get(id)
            .is_some_and(|seen| seen.title.is_some() || seen.preview.is_some());
        if named {
            return None;
        }
        params["schedule"].as_str().map(str::to_owned)
    }

    /// 推来的一条 `notice`（核心 J-3 中）写成什么；不弹的、认不得的是 `None`。`prompt` 是定时任务那句话（[`Sessions::wants_prompt`]）。
    pub fn notice(&self, params: &Value, prompt: Option<&str>) -> Option<Note> {
        match params["kind"].as_str()? {
            "schedule.ran" => {
                let id = params["session"].as_str()?;
                // 有网页开着那个会话的由它弹「回答完了」（同网页第 6 条：别的头开着它的不弹）。
                if self
                    .known
                    .get(id)
                    .is_some_and(|seen| seen.quiet || seen.viewed)
                {
                    return None;
                }
                let why = if params["ok"] == false {
                    Why::ScheduleFailed
                } else {
                    Why::ScheduleRan
                };
                Some(Note {
                    session: Some(id.to_owned()),
                    why,
                    title: self.title(id, prompt),
                    tag: format!("{id}:{}", tag_of(Why::Done)),
                    persona: self.known.get(id).and_then(|seen| seen.persona.clone()),
                    face: None,
                })
            }
            "schedules.reset" => Some(Note {
                session: None,
                why: Why::SchedulesReset,
                title: String::new(),
                tag: "schedules.reset".to_owned(),
                persona: None,
                face: None,
            }),
            _ => None,
        }
    }
}

/// `tag` 里会话编号后面那一段：定时任务跑完了同「回答完了」。
fn tag_of(why: Why) -> &'static str {
    match why {
        Why::Done | Why::ScheduleRan | Why::ScheduleFailed => "done",
        Why::Ask => "ask",
        Why::Approve => "approve",
        Why::SchedulesReset => "schedules.reset",
    }
}

/// 开始等人的会话是在问还是要确认（会话表的 `waiting` 不分）：照 `view.page {turns: 1}` 交回的那一轮的事件，最后一个还没了结的
/// `question.asked` 是在问，`tool.approval_requested` 是要确认（了结是同一个 `call_id` 后来有了 `question.answered`、
/// `tool.approval_decided`、`tool.result`，同网页 `asking` 包的认法）。看不出的（读不到、脚本里调的工具在等确认没记成这两种）算要确认：
/// 等人多半是在等确认。
pub fn waiting_why(page: &Value) -> Why {
    let mut open: Vec<(&str, Why)> = Vec::new();
    for event in page["events"].as_array().into_iter().flatten() {
        let Some(call) = event["body"]["call_id"].as_str() else {
            continue;
        };
        match event["kind"].as_str().unwrap_or_default() {
            "question.asked" => open.push((call, Why::Ask)),
            "tool.approval_requested" => open.push((call, Why::Approve)),
            "question.answered" | "tool.approval_decided" | "tool.result" => {
                open.retain(|(waiting, _)| *waiting != call);
            }
            _ => {}
        }
    }
    open.last().map_or(Why::Approve, |(_, why)| *why)
}

#[cfg(test)]
mod tests;
