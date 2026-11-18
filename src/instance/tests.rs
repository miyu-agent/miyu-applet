use super::*;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "miyu-applet-instance-{name}-{}",
        std::process::id()
    ));
    if dir.exists() {
        std::fs::remove_dir_all(&dir).unwrap();
    }
    dir
}

#[test]
fn the_key_is_stable_and_differs_by_root() {
    let a = key(Path::new("/home/u/.miyu"));
    assert_eq!(a, key(Path::new("/home/u/.miyu")));
    assert_ne!(a, key(Path::new("/tmp/other")));
    assert_eq!(a.len(), 16);
}

#[test]
fn the_second_claim_on_the_same_root_is_second() {
    let dir = scratch("claim");
    let first = claim(&dir, "k").unwrap();
    assert!(matches!(first, Claim::First(_)));
    assert!(matches!(claim(&dir, "k").unwrap(), Claim::Second));
    // 别的数据根不碍事。
    assert!(matches!(claim(&dir, "other").unwrap(), Claim::First(_)));
    drop(first);
    // 第一个退了，再起来的又是第一个。
    assert!(matches!(claim(&dir, "k").unwrap(), Claim::First(_)));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn asking_writes_something_new_each_time() {
    let dir = scratch("ask");
    std::fs::create_dir_all(&dir).unwrap();
    ask(&dir, "k", None).unwrap();
    let first = std::fs::read_to_string(request(&dir, "k")).unwrap();
    std::thread::sleep(Duration::from_millis(2));
    ask(&dir, "k", None).unwrap();
    assert_ne!(first, std::fs::read_to_string(request(&dir, "k")).unwrap());
    assert_eq!(wanted_session(&first), None, "打开网页界面");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_click_on_a_notification_asks_for_its_session() {
    let dir = scratch("ask-session");
    std::fs::create_dir_all(&dir).unwrap();
    let id = "01a1a520-30b6-702a-ada5-603e4d8a21e8";
    ask(&dir, "k", Some(id)).unwrap();
    let line = std::fs::read_to_string(request(&dir, "k")).unwrap();
    assert_eq!(wanted_session(&line), Some(id.to_owned()));
    assert_eq!(
        wanted_session("session ../x 1"),
        None,
        "编号写法不对的照打开网页界面"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn only_writes_count_as_asking() {
    use notify::event::{AccessKind, CreateKind, DataChange, ModifyKind, RemoveKind};
    assert!(written(&EventKind::Create(CreateKind::File)));
    assert!(written(&EventKind::Modify(ModifyKind::Data(
        DataChange::Any
    ))));
    assert!(!written(&EventKind::Access(AccessKind::Any)));
    assert!(!written(&EventKind::Remove(RemoveKind::File)));
}

#[test]
fn a_burst_counts_once() {
    let mut once = Once::default();
    let start = Instant::now();
    assert!(once.allow(start));
    assert!(!once.allow(start + Duration::from_millis(10)));
    assert!(!once.allow(start + Duration::from_millis(900)));
    assert!(once.allow(start + Duration::from_millis(1100)));
}

/// 审计 #098：「请打开」的文件写的时候别人读不到半截（原来先截成空的再写，读到空的当成打开首页）。
#[test]
fn a_reader_never_sees_a_half_written_request() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    let dir = scratch("atomic");
    std::fs::create_dir_all(&dir).unwrap();
    ask(&dir, "k", None).unwrap();
    let file = request(&dir, "k");
    let stop = Arc::new(AtomicBool::new(false));
    let torn = Arc::new(AtomicUsize::new(0));
    let reader = {
        let (stop, torn, file) = (Arc::clone(&stop), Arc::clone(&torn), file.clone());
        std::thread::spawn(move || {
            while !stop.load(Ordering::SeqCst) {
                if let Ok(line) = std::fs::read_to_string(&file)
                    && !line.ends_with('\n')
                {
                    torn.fetch_add(1, Ordering::SeqCst);
                }
            }
        })
    };
    let id = "01a1a520-30b6-702a-ada5-603e4d8a21e8";
    for _ in 0..3000 {
        ask(&dir, "k", Some(id)).unwrap();
    }
    stop.store(true, Ordering::SeqCst);
    reader.join().unwrap();
    assert_eq!(torn.load(Ordering::SeqCst), 0, "读到了写了一半的");
    std::fs::remove_dir_all(&dir).unwrap();
}

/// 审计 #098：连着点两条通知（两个会话），盯着的那一个每一次只交一回、照先后（原来写一次报好几下，靠一秒只认一次压住，
/// 后点的那一条被压掉）。
#[test]
fn each_ask_is_passed_on_once_in_order() {
    // macOS 的 `/var` 是链接，系统报来的是真实路径（`/private/var/…`）。
    let dir = scratch("listen");
    std::fs::create_dir_all(&dir).unwrap();
    let dir = dir.canonicalize().unwrap();
    let (send, receive) = std::sync::mpsc::channel();
    let _watching = listen(&dir, "k", move |session| {
        if send.send(session).is_err() {
            // 测试已经看完了。
        }
    })
    .unwrap();
    let a = "01a1a520-30b6-702a-ada5-603e4d8a21e8";
    let b = "02b2b520-30b6-702a-ada5-603e4d8a21e8";
    ask(&dir, "k", Some(a)).unwrap();
    std::thread::sleep(Duration::from_millis(300));
    ask(&dir, "k", Some(b)).unwrap();
    std::thread::sleep(Duration::from_millis(500));
    let got: Vec<Option<String>> = receive.try_iter().collect();
    // 一次一交；系统把两下并成一下报的（macOS 会）只交后一个，后点的总是最后交、不被压掉。
    assert!(got.windows(2).all(|pair| pair[0] != pair[1]), "{got:?}");
    assert_eq!(got.last(), Some(&Some(b.to_owned())), "{got:?}");
    assert!(got.len() <= 2, "{got:?}");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_session_is_never_held_back_by_the_burst_guard() {
    let mut once = Once::default();
    let start = Instant::now();
    let id = "01a1a520-30b6-702a-ada5-603e4d8a21e8";
    assert!(once.pass(start, None));
    assert!(once.pass(start + Duration::from_millis(10), Some(id)));
    assert!(!once.pass(start + Duration::from_millis(20), None));
}
