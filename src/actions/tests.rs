use super::*;

use crate::text::Lang;

fn ran(code: i32, stderr: &str) -> Result<Ran, RunError> {
    Ok(Ran {
        code: Some(code),
        stdout: Vec::new(),
        stderr: stderr.into(),
    })
}

#[test]
fn only_exit_code_two_falls_back_to_plain_web() {
    assert!(not_understood(&ran(2, "usage: miyu-web open …")));
    assert!(!not_understood(&ran(0, "")));
    assert!(!not_understood(&ran(1, "miyu: …")));
    assert!(!not_understood(&Err(RunError::Timeout)));
}

#[test]
fn success_says_nothing_and_failures_say_why() {
    let text = Text::of(Lang::Zh);
    assert_eq!(failure(&ran(0, "ignored"), &text), None);
    assert_eq!(
        failure(&ran(1, "a\nmiyu: 核心起不来\n"), &text),
        Some("a\nmiyu: 核心起不来".into())
    );
    assert_eq!(
        failure(&Err(RunError::Timeout), &text),
        Some(text.get(key::TIMEOUT).to_owned())
    );
    let missing = std::io::Error::new(std::io::ErrorKind::NotFound, "no such file");
    assert_eq!(
        failure(&Err(RunError::Spawn(missing)), &text),
        Some("no such file".into())
    );
    let killed = Ok(Ran {
        code: None,
        stdout: Vec::new(),
        stderr: Vec::new(),
    });
    assert_eq!(failure(&killed, &text), Some(String::new()));
}

#[test]
fn each_page_has_its_arguments() {
    assert_eq!(Page::Home.args(), ["web"]);
    assert_eq!(Page::Settings.args(), ["web", "--settings"]);
    assert_eq!(Page::Access.args(), ["web", "--settings", "access"]);
    assert_eq!(Page::Package("im".into()).args(), ["pkg", "page", "im"]);
    assert_eq!(
        Page::Session("019a-1".into()).args(),
        ["web", "--session", "019a-1"]
    );
}
