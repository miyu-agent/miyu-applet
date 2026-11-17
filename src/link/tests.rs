use super::*;

use notify::event::{AccessKind, CreateKind, ModifyKind, RemoveKind};

#[test]
fn the_watch_falls_back_from_run_to_the_root_to_its_parent() {
    let root = Path::new("/home/u/.miyu");
    let run = root.join("run");
    let all = |_: &Path| true;
    assert_eq!(target(&run, root, all), Some(run.clone()));
    let no_run = |p: &Path| p != run;
    assert_eq!(target(&run, root, no_run), Some(root.to_path_buf()));
    let only_parent = |p: &Path| p == Path::new("/home/u");
    assert_eq!(
        target(&run, root, only_parent),
        Some(PathBuf::from("/home/u"))
    );
    assert_eq!(target(&run, root, |_| false), None);
}

#[test]
fn changes_inside_the_data_root_count() {
    let root = Path::new("/home/u/.miyu");
    let socket = vec![root.join("run").join("socket")];
    for kind in [
        EventKind::Create(CreateKind::File),
        EventKind::Modify(ModifyKind::Any),
        EventKind::Remove(RemoveKind::File),
        EventKind::Any,
    ] {
        assert!(relevant(&kind, &socket, root), "{kind:?}");
    }
    // 数据根本身刚建出来（盯的是上一级）。
    assert!(relevant(
        &EventKind::Create(CreateKind::Folder),
        &[root.to_path_buf()],
        root
    ));
}

#[test]
fn reads_and_things_outside_do_not_count() {
    let root = Path::new("/home/u/.miyu");
    let token = vec![root.join("run").join("token")];
    assert!(!relevant(&EventKind::Access(AccessKind::Any), &token, root));
    assert!(!relevant(
        &EventKind::Create(CreateKind::File),
        &[PathBuf::from("/home/u/notes.txt")],
        root
    ));
    assert!(!relevant(
        &EventKind::Create(CreateKind::File),
        &[PathBuf::from("/home/u/.miyu-old")],
        root
    ));
}

/// 两个真核心的测试用同一个数据根，一个一个跑。
static REAL_CORE: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// 等到报来一样合 `want` 的（中间别的照收不管），最多等 20 秒。
fn until(receive: &std::sync::mpsc::Receiver<Report>, want: impl Fn(&Report) -> bool) -> Report {
    loop {
        let got = receive.recv_timeout(Duration::from_secs(20)).unwrap();
        if want(&got) {
            return got;
        }
    }
}

/// 真核心的测试要的：主程序、只认指到临时目录的 `MIYU_HOME`（拉起的核心照环境找数据根，不设就是主人的 `~/.miyu`）。
fn real_core() -> (PathBuf, DataRoot) {
    let miyu = PathBuf::from(std::env::var_os("MIYU_TEST_PROGRAM").expect("MIYU_TEST_PROGRAM"));
    let home = PathBuf::from(std::env::var_os("MIYU_HOME").expect("MIYU_HOME"));
    assert!(
        home.starts_with(std::env::temp_dir()),
        "MIYU_HOME 要在临时目录里"
    );
    let root = DataRoot::locate(&miyu_client::places::Env::current()).unwrap();
    assert_eq!(root.path(), home);
    (miyu, root)
}

/// 拿真的核心走一遍：托盘那条连接起来先报连不上；拉起以后连上、报一次软件包的入口（出厂的包都没写 `[tray]`）和局域网（出厂不是开着的）；`miyu core stop`
/// 以后报断。要人手跑：`MIYU_HOME=$(mktemp -d) MIYU_RESOURCES=<资源目录> MIYU_TEST_PROGRAM=<miyu> cargo test -- --ignored a_real_core`。
/// 只靠盯 `run/` 的文件变动跟上（没人叫醒）。
#[test]
#[ignore = "要真的核心：MIYU_TEST_PROGRAM 指到 miyu，MIYU_HOME 指到临时的空目录"]
fn a_real_core_goes_up_and_down() {
    let _one = REAL_CORE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let (miyu, root) = real_core();
    let (send, receive) = std::sync::mpsc::channel();
    let _waker = spawn(root.clone(), move |got| send.send(got).unwrap());
    assert_eq!(until(&receive, |_| true), Report::Down, "起来先报连不上");
    crate::actions::start_core(&miyu, &root).unwrap();
    until(&receive, |got| *got == Report::Up);
    let packages = until(&receive, |got| matches!(got, Report::Packages(_)));
    assert_eq!(packages, Report::Packages(Vec::new()), "出厂的包都不挂托盘");
    let lan = until(&receive, |got| matches!(got, Report::Lan(_)));
    // 资源目录带网页包的报关着，不带的报不认（`None`）：都不是开着。
    assert_ne!(lan, Report::Lan(Some(true)), "出厂只听本机");
    let text = crate::text::Text::of(crate::text::Lang::En);
    crate::quit::stop_core(&miyu, &text).unwrap();
    until(&receive, |got| *got == Report::Down);
    // 核心 CS-5：「停止」「退出 Miyu」走 `core.stop`，留下人停的记号，别的头断了线自己重连不再拉起；「启动」是人要用的，照拉、删掉记号。
    let stopped = root.run().join("stopped");
    assert!(stopped.is_file(), "托盘叫停的是人停的");
    crate::actions::start_core(&miyu, &root).unwrap();
    until(&receive, |got| *got == Report::Up);
    assert!(!stopped.exists(), "「启动」以后不再是人停的");
    crate::quit::stop_core(&miyu, &text).unwrap();
    until(&receive, |got| *got == Report::Down);
}
