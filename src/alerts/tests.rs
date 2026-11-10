use super::*;

use serde_json::json;

fn table(entries: Value) -> Sessions {
    let mut sessions = Sessions::default();
    sessions.replace(&json!({ "sessions": entries }));
    sessions
}

fn changed(id: &str, entry: Value) -> Value {
    json!({"session": id, "entry": entry})
}

#[test]
fn a_turn_that_ends_is_done_and_one_that_waits_is_waiting() {
    let mut sessions = table(json!([{"session": "a", "busy": true, "title": "修 CI"}]));
    let waiting = sessions.change(&changed(
        "a",
        json!({"busy": true, "waiting": true, "title": "修 CI"}),
    ));
    assert_eq!(waiting.rise, Some(Rise::Waiting("a".into())));
    // 答了接着跑：不弹；一轮做完：回答完了。
    let answered = sessions.change(&changed("a", json!({"busy": true, "title": "修 CI"})));
    assert_eq!(answered.rise, None);
    let done = sessions.change(&changed("a", json!({"title": "修 CI"})));
    assert_eq!(done.rise, Some(Rise::Done("a".into())));
    // 又推一遍一样的：不再弹。
    assert_eq!(
        sessions
            .change(&changed("a", json!({"title": "修 CI"})))
            .rise,
        None
    );
}

#[test]
fn what_was_there_when_subscribing_is_not_news() {
    // 订阅时就在等人、刚做完的不报；第一次见到的会话照「以前没在跑」算。
    let mut sessions = table(json!([{"session": "a", "waiting": true, "busy": true}]));
    assert_eq!(
        sessions
            .change(&changed("a", json!({"waiting": true, "busy": true})))
            .rise,
        None
    );
    assert_eq!(
        sessions.change(&changed("new", json!({"busy": true}))).rise,
        None
    );
    assert_eq!(
        sessions.change(&changed("new", json!({}))).rise,
        Some(Rise::Done("new".into()))
    );
}

#[test]
fn sessions_a_web_page_has_open_are_left_to_it() {
    // 网页开着的（`viewed`）由网页弹；子会话、一次性的、整理记忆的不弹。
    for entry in [
        json!({"viewed": true}),
        json!({"parent": "p"}),
        json!({"oneshot": true}),
        json!({"kind": "dream"}),
    ] {
        let mut was = entry.clone();
        was["busy"] = json!(true);
        let mut sessions = table(json!([{"session": "a", "busy": true}]));
        sessions.change(&changed("a", was));
        assert_eq!(
            sessions.change(&changed("a", entry.clone())).rise,
            None,
            "{entry}"
        );
    }
    // 网页关了（`viewed` 没了）以后的照常弹。
    let mut sessions = table(json!([{"session": "a", "busy": true, "viewed": true}]));
    sessions.change(&changed("a", json!({"busy": true})));
    assert_eq!(
        sessions.change(&changed("a", json!({}))).rise,
        Some(Rise::Done("a".into()))
    );
}

#[test]
fn notes_carry_the_title_and_a_tag_per_session_and_kind() {
    let mut sessions = table(json!([{"session": "a", "title": " 修 CI "}]));
    let now = Instant::now();
    let note = sessions.note("a", Why::Ask, now);
    assert_eq!(
        note,
        Some(Note {
            session: Some("a".into()),
            why: Why::Ask,
            title: "修 CI".into(),
            tag: "a:ask".into(),
            persona: None,
            face: None,
        })
    );
    assert_eq!(
        sessions.note("a", Why::Approve, now).map(|n| n.tag),
        Some("a:approve".into())
    );
    assert_eq!(
        sessions.note("a", Why::Done, now).map(|n| n.tag),
        Some("a:done".into())
    );
}

#[test]
fn without_a_title_it_waits_for_one_then_falls_back_to_the_first_line() {
    let mut sessions = table(json!([{"session": "a", "preview": "帮我看看日志"}]));
    let now = Instant::now();
    assert_eq!(sessions.note("a", Why::Done, now), None);
    // 同一种不再等一条。
    assert_eq!(sessions.note("a", Why::Done, now), None);
    assert_eq!(sessions.next_due(), Some(now + TITLE_WAIT));
    assert!(sessions.due(now + TITLE_WAIT / 2).is_empty());
    // 标题来了：马上照它写。
    let got = sessions.change(&changed(
        "a",
        json!({"title": "看日志", "preview": "帮我看看日志"}),
    ));
    assert_eq!(got.ready.len(), 1);
    assert_eq!(got.ready[0].title, "看日志");
    assert_eq!(sessions.next_due(), None);

    // 一直没来：到点照第一句；第一句也没有的空着（正文只写为什么）。
    let mut sessions =
        table(json!([{"session": "b", "preview": "帮我看看日志"}, {"session": "c"}]));
    assert_eq!(sessions.note("b", Why::Done, now), None);
    assert_eq!(sessions.note("c", Why::Ask, now), None);
    let late = sessions.due(now + TITLE_WAIT);
    let titles: Vec<&str> = late.iter().map(|note| note.title.as_str()).collect();
    assert_eq!(titles, ["帮我看看日志", ""]);
    assert!(sessions.due(now + TITLE_WAIT * 2).is_empty());
}

#[test]
fn a_removed_session_drops_what_was_waiting_for_it() {
    let mut sessions = table(json!([{"session": "a"}]));
    let now = Instant::now();
    assert_eq!(sessions.note("a", Why::Done, now), None);
    sessions.change(&json!({"session": "a", "removed": true}));
    assert_eq!(sessions.next_due(), None);
}

#[test]
fn a_finished_schedule_shares_the_done_tag() {
    let sessions = table(json!([{"session": "s", "title": "日报"}]));
    let ran = json!({"kind": "schedule.ran", "schedule": "j1", "session": "s", "ok": true});
    assert_eq!(sessions.wants_prompt(&ran), None);
    let note = sessions.notice(&ran, None);
    assert_eq!(
        note,
        Some(Note {
            session: Some("s".into()),
            why: Why::ScheduleRan,
            title: "日报".into(),
            tag: "s:done".into(),
            persona: None,
            face: None,
        })
    );
    let failed = json!({"kind": "schedule.ran", "schedule": "j1", "session": "s", "ok": false});
    assert_eq!(
        sessions.notice(&failed, None).map(|n| n.why),
        Some(Why::ScheduleFailed)
    );
}

#[test]
fn a_schedule_without_a_title_goes_by_its_prompt() {
    // 会话表里还没有那个会话：照定时任务那句话，再没有空着。
    let sessions = Sessions::default();
    let ran = json!({"kind": "schedule.ran", "schedule": "j1", "session": "s", "ok": true});
    assert_eq!(sessions.wants_prompt(&ran), Some("j1".into()));
    assert_eq!(
        sessions.notice(&ran, Some("每天早上汇总")).map(|n| n.title),
        Some("每天早上汇总".into())
    );
    assert_eq!(
        sessions.notice(&ran, None).map(|n| n.title),
        Some(String::new())
    );
    // 网页开着那个会话的不弹。
    let viewed = table(json!([{"session": "s", "viewed": true, "title": "日报"}]));
    assert_eq!(viewed.notice(&ran, None), None);
}

#[test]
fn a_reset_schedule_table_is_an_account_notice() {
    let sessions = Sessions::default();
    let note = sessions.notice(&json!({"kind": "schedules.reset", "at": "…"}), None);
    assert_eq!(
        note,
        Some(Note {
            session: None,
            why: Why::SchedulesReset,
            title: String::new(),
            tag: "schedules.reset".into(),
            persona: None,
            face: None,
        })
    );
    assert_eq!(sessions.notice(&json!({"kind": "later.kind"}), None), None);
}

/// 照会话的人格（AP-20，2026-11-10 项目主人）：会话表那一项的 `persona` 跟着通知走；标题是人格的名字，没问到的、无人格的写 Miyu；
/// 正文是会话的标题接为什么，没有标题的只写为什么；有头像的才带图。
#[test]
fn notes_follow_the_persona_of_the_session() {
    let mut sessions = table(json!([
        {"session": "a", "title": "修 CI", "persona": "miyuki"},
        {"session": "b", "title": "看日志"}
    ]));
    let now = Instant::now();
    let a = sessions.note("a", Why::Done, now).expect("有标题");
    assert_eq!(a.persona.as_deref(), Some("miyuki"));
    assert_eq!(a.heading(), PROGRAM, "还没问到人格的写程序名");
    assert_eq!(a.body("回答完了"), "修 CI · 回答完了");
    let b = sessions.note("b", Why::Done, now).expect("有标题");
    assert_eq!(b.persona, None, "无人格的不带");
    let avatar = Avatar {
        version: "0123abcd".into(),
        ext: "png",
        bytes: vec![1, 2, 3],
    };
    let dressed = Note {
        face: Some(Face {
            name: "美雪".into(),
            avatar: Some(avatar.clone()),
        }),
        ..a.clone()
    };
    assert_eq!(dressed.heading(), "美雪");
    assert_eq!(dressed.avatar(), Some(&avatar));
    let bare = Note {
        face: Some(Face {
            name: "美雪".into(),
            avatar: None,
        }),
        ..a
    };
    assert_eq!(bare.avatar(), None, "没头像的不带图");
    let blank = Note {
        title: String::new(),
        ..bare
    };
    assert_eq!(blank.body("回答完了"), "回答完了");
    // 定时任务跑完的也带那个会话的人格；唤醒表坏了的没有人格
    let ran = json!({"kind": "schedule.ran", "schedule": "j1", "session": "a", "ok": true});
    assert_eq!(
        sessions.notice(&ran, None).and_then(|n| n.persona),
        Some("miyuki".into())
    );
    let reset = sessions
        .notice(&json!({"kind": "schedules.reset"}), None)
        .expect("提醒");
    assert_eq!(reset.heading(), PROGRAM);
    assert_eq!(reset.persona, None);
}

#[test]
fn the_last_open_question_or_approval_says_which_kind_of_wait() {
    let event = |kind: &str, call: &str| json!({"kind": kind, "body": {"call_id": call}});
    let page = |events: Vec<Value>| json!({ "events": events });
    assert_eq!(
        waiting_why(&page(vec![event("question.asked", "c1")])),
        Why::Ask
    );
    assert_eq!(
        waiting_why(&page(vec![event("tool.approval_requested", "c1")])),
        Why::Approve
    );
    // 了结了的不算：问完了又要确认的是要确认，确认完了又问的是在问。
    assert_eq!(
        waiting_why(&page(vec![
            event("question.asked", "c1"),
            event("question.answered", "c1"),
            event("tool.approval_requested", "c2"),
        ])),
        Why::Approve
    );
    assert_eq!(
        waiting_why(&page(vec![
            event("tool.approval_requested", "c1"),
            event("tool.result", "c1"),
            event("question.asked", "c2"),
        ])),
        Why::Ask
    );
    // 看不出的算要确认。
    assert_eq!(waiting_why(&page(vec![])), Why::Approve);
    assert_eq!(waiting_why(&Value::Null), Why::Approve);
}
