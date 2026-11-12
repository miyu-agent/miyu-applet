use super::*;

use crate::text::Lang;

#[test]
fn the_pins_in_the_repository_are_read() {
    let pins = pins(include_str!("../../pins.toml"));
    let core = pins.core.unwrap();
    assert_eq!(core.repo, "miyu-agent/miyu");
    assert_eq!(core.rev.len(), 40);
    assert_eq!(pins.web.unwrap().repo, "miyu-agent/miyu-web");
}

#[test]
fn broken_pins_are_left_out() {
    assert_eq!(pins("not toml ["), Pins::default());
    let half = pins("[core]\nrepo = \"a/b\"\n[web]\nrepo = \"c/d\"\nrev = \"123\"\n");
    assert_eq!(half.core, None);
    assert_eq!(half.web.unwrap().rev, "123");
}

#[test]
fn the_core_version_drops_the_program_name() {
    assert_eq!(core_version(b"miyu 0.0.0\n"), Some("0.0.0".into()));
    assert_eq!(core_version(b"1.2.3"), Some("1.2.3".into()));
    assert_eq!(core_version(b""), None);
    assert_eq!(core_version(b"miyu \n"), None);
}

#[test]
fn the_web_version_comes_from_the_package_table() {
    let manifest = "[package]\nversion = \"0.4.1\"\nid = \"webui\"\n";
    assert_eq!(web_version(manifest), Some("0.4.1".into()));
    assert_eq!(web_version("[package]\nid = \"x\"\n"), None);
    assert_eq!(web_version("version = \"1\"\n"), None);
}

#[test]
fn the_message_lists_all_three_with_their_pins() {
    let text = Text::of(Lang::Zh);
    let pins = Pins {
        core: Some(Pin {
            repo: "miyu-agent/miyu".into(),
            rev: "abc".into(),
        }),
        web: None,
    };
    let message = describe(&text, "0.1.0", Some("0.0.0"), None, &pins);
    assert_eq!(
        message,
        "托盘: 0.1.0\n核心: 0.0.0\n  miyu-agent/miyu@abc\n网页: 不知道"
    );
}
