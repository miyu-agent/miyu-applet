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
