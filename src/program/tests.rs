use super::*;

fn exe(name: &str) -> String {
    format!("{name}{}", std::env::consts::EXE_SUFFIX)
}

#[test]
fn the_environment_variable_wins() {
    let got = locate(
        Some("/dev/miyu".into()),
        Some(Path::new("/opt/m")),
        None,
        |_| true,
    );
    assert_eq!(got, Some(PathBuf::from("/dev/miyu")));
}

#[test]
fn an_empty_environment_variable_is_not_set() {
    let got = locate(Some("".into()), Some(Path::new("/opt/m")), None, |_| true);
    assert_eq!(got, Some(Path::new("/opt/m").join(exe("miyu"))));
}

#[test]
fn beside_the_shell_comes_before_the_path() {
    let path = std::env::join_paths(["/usr/bin"]).unwrap();
    let got = locate(None, Some(Path::new("/opt/m")), Some(path), |_| true);
    assert_eq!(got, Some(Path::new("/opt/m").join(exe("miyu"))));
}

#[test]
fn the_path_is_searched_in_order() {
    // 绝对目录：Windows 上 `/b` 这种没有盘符的不算绝对，照临时目录拼。
    let root = std::env::temp_dir();
    let path = std::env::join_paths([root.join("a"), root.join("b"), root.join("c")]).unwrap();
    let wanted = root.join("b").join(exe("miyu"));
    let got = locate(None, Some(Path::new("/opt/m")), Some(path), |p| p == wanted);
    assert_eq!(got, Some(wanted));
}

#[test]
fn nowhere_is_none() {
    let path = std::env::join_paths(["/a"]).unwrap();
    assert_eq!(
        locate(None, Some(Path::new("/opt/m")), Some(path), |_| false),
        None
    );
    assert_eq!(locate(None, None, None, |_| true), None);
}

#[cfg(unix)]
#[test]
fn run_collects_output_and_the_exit_code() {
    let got = run(
        Path::new("/bin/sh"),
        &["-c", "echo out; echo err >&2; exit 3"],
        Duration::from_secs(10),
    )
    .unwrap();
    assert_eq!(got.code, Some(3));
    assert_eq!(got.stdout, b"out\n");
    assert_eq!(got.stderr, b"err\n");
}

#[cfg(unix)]
#[test]
fn run_kills_what_runs_too_long() {
    let started = Instant::now();
    let got = run(
        Path::new("/bin/sh"),
        &["-c", "sleep 30"],
        Duration::from_millis(200),
    );
    assert!(matches!(got, Err(RunError::Timeout)));
    assert!(started.elapsed() < Duration::from_secs(10));
}

#[test]
fn run_says_when_it_cannot_start() {
    let got = run(
        Path::new("/nowhere/miyu-not-here"),
        &[],
        Duration::from_secs(1),
    );
    assert!(matches!(got, Err(RunError::Spawn(_))));
}

#[cfg(unix)]
#[test]
fn run_does_not_wait_for_a_child_that_keeps_the_pipe() {
    // `miyu web` 顺带拉起的核心、网页软件继承了管道（施工 DT-6 在 Windows 上碰到）：程序退了就交，不等管道读到头。
    let started = Instant::now();
    let got = run(
        Path::new("/bin/sh"),
        &["-c", "echo opened; sleep 30 & exit 0"],
        Duration::from_secs(10),
    )
    .unwrap();
    assert_eq!(got.code, Some(0));
    assert_eq!(got.stdout, b"opened\n");
    assert!(started.elapsed() < Duration::from_secs(5));
}

#[test]
fn the_tail_keeps_the_last_lines_without_blanks() {
    assert_eq!(tail(b"a\n\nb\nc\nd\ne\nf\ng\n"), "b\nc\nd\ne\nf\ng");
    assert_eq!(tail(b""), "");
}

/// 审计第五轮 D5-4（红）：`PATH` 里的空项、`.`、相对目录照外壳的当前目录解析（图形程序的当前目录说不准：从文件管理器、
/// 别的程序拉起来时是那边的目录），那里放一个 `miyu` 就被当成主程序调。只该照绝对目录找。
#[test]
fn audit5_relative_path_entries_are_not_searched() {
    let absolute = std::env::temp_dir();
    let path = std::env::join_paths([
        Path::new(""),
        Path::new("."),
        Path::new("bin"),
        absolute.as_path(),
    ])
    .unwrap();
    let got = locate(None, None, Some(path), |_| true);
    assert_eq!(got, Some(absolute.join(exe("miyu"))));
}

#[test]
fn the_fallback_dirs_are_searched_in_order_after_the_path() {
    let root = std::env::temp_dir();
    let (a, b) = (root.join("fa"), root.join("fb"));
    let wanted = b.join(exe("miyu"));
    let dirs = [a, b];
    assert_eq!(in_dirs(&dirs, |p| p == wanted), Some(wanted.clone()));
    assert_eq!(in_dirs(&dirs, |_| false), None);
    assert_eq!(in_dirs(&[], |_| true), None);
}

/// 安装脚本装在家目录下（`~/.local/lib/miyu/miyu`，`~/.local/bin/miyu` 链过去）：从访达拉起、`PATH` 里没有的也找得到，先于 Homebrew
/// 的前缀（AP-19）；家目录不是绝对路径的不看。Windows 上不看这几处。
#[test]
fn the_install_script_places_under_home_come_first() {
    let home = Path::new("/Users/me");
    let dirs = fallback_dirs(Some(home));
    if cfg!(windows) {
        assert!(dirs.is_empty());
        return;
    }
    assert_eq!(dirs[0], home.join(".local/lib/miyu"));
    assert_eq!(dirs[1], home.join(".local/bin"));
    let brew = dirs
        .iter()
        .position(|d| d == Path::new("/opt/homebrew/bin"));
    assert_eq!(brew.is_some(), cfg!(target_os = "macos"));
    let found = in_dirs(&dirs, |p| p == home.join(".local/bin/miyu"));
    assert_eq!(found, Some(home.join(".local/bin/miyu")));
    assert!(
        fallback_dirs(Some(Path::new("relative")))
            .iter()
            .all(|d| d.is_absolute())
    );
    assert!(!fallback_dirs(None).contains(&home.join(".local/bin")));
}
