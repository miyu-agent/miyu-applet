use super::*;

#[test]
fn the_next_note_with_the_same_tag_replaces_the_open_one() {
    let mut shown = Shown::default();
    assert_eq!(shown.replaces("a:done"), None);
    shown.sent("a:done", 7);
    assert_eq!(shown.replaces("a:done"), Some(7));
    assert_eq!(shown.replaces("a:ask"), None);
}

#[test]
fn only_the_latest_send_of_a_replaced_note_counts_a_click() {
    let mut shown = Shown::default();
    let first = shown.sent("a:done", 7);
    // 换掉：通知服务照旧给编号 7。
    let second = shown.sent("a:done", 7);
    // 点了：两个线程都收到，只有后一次算。
    assert!(!shown.answered(7, first, false));
    assert!(shown.answered(7, second, false));
}

#[test]
fn a_closed_note_is_forgotten_and_the_next_one_starts_fresh() {
    let mut shown = Shown::default();
    let first = shown.sent("a:done", 7);
    let second = shown.sent("a:done", 7);
    // 旧那一次收到关掉了：不算数，也不忘（最新那一次还开着）。
    assert!(!shown.answered(7, first, true));
    assert_eq!(shown.replaces("a:done"), Some(7));
    assert!(shown.answered(7, second, true));
    assert_eq!(shown.replaces("a:done"), None);
}

/// 人格的头像写进托盘目录的 `avatars/<版本>.<扩展名>`（AP-20）；同样的不再写；版本里只留字母数字；没有托盘目录的不带图。
#[test]
fn an_avatar_is_written_once_under_avatars() {
    let own = std::env::temp_dir().join(format!("miyu-applet-avatar-{}", std::process::id()));
    let avatar = Avatar {
        version: "01/23..ab".into(),
        ext: "png",
        bytes: vec![1, 2, 3],
    };
    let path = avatar_file(Some(&own), &avatar).expect("写得进");
    assert_eq!(path, own.join("avatars").join("0123ab.png"));
    assert_eq!(std::fs::read(&path).unwrap(), [1, 2, 3]);
    assert_eq!(avatar_file(Some(&own), &avatar), Some(path));
    assert_eq!(avatar_file(None, &avatar), None);
    std::fs::remove_dir_all(&own).expect("删得掉临时目录");
}

/// 审计 #100：通知服务不发「关掉了」（mako 默认不过期、没人点就一直开着）的时候，等人点的线程不跟着通知越攒越多：同一个 `tag` 的
/// 换掉上一条，上一条不再等；一共开着的有上限。要一条只有假通知服务的会话总线：
/// `dbus-run-session -- cargo test -- --ignored waiting_for_clicks`（主人的总线上通知服务的名字被占着，起不来、不弹）。
#[cfg(all(unix, not(target_os = "macos")))]
#[test]
#[ignore = "要单独的会话总线：dbus-run-session -- cargo test -- --ignored waiting_for_clicks"]
fn waiting_for_clicks_does_not_pile_up_threads() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let _server = runtime
        .block_on(fake_server::start())
        .expect("占不到通知服务的名字：要在 dbus-run-session 里跑");
    let toaster = Toaster::new(
        Arc::new(crate::text::Text::of(crate::text::Lang::En)),
        None,
        |_| {},
    );
    let note = |tag: String| Note {
        session: Some("01a1a520-30b6-702a-ada5-603e4d8a21e8".into()),
        why: crate::alerts::Why::Done,
        title: "t".into(),
        tag,
        persona: None,
        face: None,
    };
    for _ in 0..20 {
        toaster.show(note("s:done".into()));
    }
    std::thread::sleep(std::time::Duration::from_secs(2));
    let same = threads("notification");
    assert!(same <= 1, "同一个 tag 的 20 条：还有 {same} 个线程在等");
    for n in 0..60 {
        toaster.show(note(format!("s{n}:done")));
    }
    std::thread::sleep(std::time::Duration::from_secs(2));
    let many = threads("notification");
    assert!(many <= 32, "60 个 tag：还有 {many} 个线程在等");
}

/// 这个进程里名字以 `prefix` 开头的线程有几个。
#[cfg(all(unix, not(target_os = "macos")))]
fn threads(prefix: &str) -> usize {
    std::fs::read_dir("/proc/self/task")
        .into_iter()
        .flatten()
        .flatten()
        .filter(|task| {
            std::fs::read_to_string(task.path().join("comm"))
                .is_ok_and(|name| name.starts_with(prefix))
        })
        .count()
}

/// 假的通知服务：照规范答 `Notify`（换掉的交回原来的编号），从不发「关掉了」、「点了」。
#[cfg(all(unix, not(target_os = "macos")))]
mod fake_server {
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicU32, Ordering};

    use zbus::zvariant::OwnedValue;

    struct Fake {
        next: AtomicU32,
    }

    #[zbus::interface(name = "org.freedesktop.Notifications")]
    impl Fake {
        #[allow(clippy::too_many_arguments, reason = "规范定的八个参数")]
        fn notify(
            &self,
            _app: String,
            replaces: u32,
            _icon: String,
            _summary: String,
            _body: String,
            _actions: Vec<String>,
            _hints: HashMap<String, OwnedValue>,
            _timeout: i32,
        ) -> u32 {
            if replaces != 0 {
                replaces
            } else {
                self.next.fetch_add(1, Ordering::SeqCst) + 1
            }
        }

        fn get_capabilities(&self) -> Vec<String> {
            vec!["actions".into(), "body".into()]
        }

        fn get_server_information(&self) -> (String, String, String, String) {
            ("fake".into(), "test".into(), "0".into(), "1.2".into())
        }

        fn close_notification(&self, _id: u32) {}
    }

    pub async fn start() -> zbus::Result<zbus::Connection> {
        zbus::connection::Builder::session()?
            .name("org.freedesktop.Notifications")?
            .serve_at(
                "/org/freedesktop/Notifications",
                Fake {
                    next: AtomicU32::new(0),
                },
            )?
            .build()
            .await
    }
}
