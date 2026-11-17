//! 起不来、连不上的原因（核心 LC-1、EN-2）：没说法的照原话；有说法的照核心资源里的整句。

use miyu_client::connect::{ConnectError, StartError};

use super::*;

/// 一份假的核心资源目录：`core/human/en.json` 里有起不来的几句。
fn resources(name: &str) -> std::path::PathBuf {
    let dir =
        std::env::temp_dir().join(format!("miyu-applet-reason-{name}-{}", std::process::id()));
    let human = dir.join("core/human");
    std::fs::create_dir_all(&human).unwrap();
    std::fs::write(
        human.join("en.json"),
        r#"{"said": {"start/log_failed": "The core couldn't start: cannot write its log: {reason}",
            "start/timeout": "Timed out waiting for the core to start",
            "start/not_running": "The core isn't running"}}"#,
    )
    .unwrap();
    dir
}

#[test]
fn a_core_that_cannot_start_is_said_whole_in_the_tray_language() {
    // 核心 EN-2：起不来的每一步都带代码，原因只是会变的那一段；整句照核心资源里的那一句，不是原话（原来英文系统上写
    // 「File exists (os error 17)」半句，等太久的写中文）。
    let dir = resources("start");
    let mut env = Env::current();
    env.miyu_resources = Some(dir.clone().into_os_string());
    let started = |error: StartError| start_failed_in(&env, &error, Lang::En);
    let refused = StartError::Refused {
        code: Some("log_failed".into()),
        reason: "File exists (os error 17)".into(),
    };
    assert_eq!(
        started(refused),
        "The core couldn't start: cannot write its log: File exists (os error 17)"
    );
    assert_eq!(
        started(StartError::Timeout),
        "Timed out waiting for the core to start"
    );
    assert_eq!(
        started(StartError::Connect(ConnectError::NotRunning)),
        "The core isn't running"
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn without_words_the_reason_is_kept() {
    assert_eq!(say(None, "磁盘满了", Lang::En), "磁盘满了");
}

/// 照真的资源目录找那一句（要 `MIYU_RESOURCES` 指到核心检出的 resources/，`cargo test -- --ignored`）。
#[test]
#[ignore = "要核心的资源目录"]
fn a_coded_reason_reads_the_core_phrase_in_the_tray_language() {
    let old = || Said::new("core/start/data_root_old_layout").with("path", "/data/miyu");
    let en = say(Some(old()), "原话", Lang::En);
    let ja = say(Some(old()), "原话", Lang::Ja);
    println!("en: {en}\nja: {ja}");
    assert!(en.contains("/data/miyu") && en != "原话", "{en}");
    assert!(ja.contains("/data/miyu") && ja != en, "{ja}");
    assert_eq!(
        say(Some(Said::new("core/start/no_such_code")), "原话", Lang::En),
        "原话",
        "没有那一句的照原话"
    );
}
