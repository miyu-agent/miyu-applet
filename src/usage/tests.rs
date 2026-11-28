use super::*;

fn args(list: &[&str]) -> Vec<String> {
    list.iter().map(|arg| (*arg).to_owned()).collect()
}

#[test]
fn version_and_help_are_answered_alone() {
    assert_eq!(early(args(&["--version"])), Some(Early::Version));
    assert_eq!(early(args(&["-V"])), Some(Early::Version));
    assert_eq!(early(args(&["--help"])), Some(Early::Help));
    assert_eq!(early(args(&["-h"])), Some(Early::Help));
}

#[test]
fn help_wins_and_other_arguments_do_not_matter() {
    assert_eq!(early(args(&["--version", "--help"])), Some(Early::Help));
    assert_eq!(early(args(&["-V", "-h"])), Some(Early::Help));
    assert_eq!(
        early(args(&["--autostart", "--version"])),
        Some(Early::Version)
    );
    assert_eq!(early(args(&["--bogus", "-h"])), Some(Early::Help));
}

#[test]
fn the_usual_starts_are_not_answered() {
    assert_eq!(early(Vec::new()), None);
    assert_eq!(early(args(&["--autostart"])), None);
    assert_eq!(early(args(&["--bogus"])), None, "不认识的照旧起托盘");
    assert_eq!(
        early(args(&[
            "--home",
            "/m",
            "--open",
            "miyu-applet://open/?session=1"
        ])),
        None
    );
}

#[test]
fn the_values_of_open_and_home_are_not_flags() {
    assert_eq!(early(args(&["--home", "-V"])), None);
    assert_eq!(early(args(&["--open", "--help"])), None);
    assert_eq!(
        early(args(&["--home", "-h", "--version"])),
        Some(Early::Version)
    );
}

#[test]
fn the_version_line_names_the_program() {
    assert_eq!(
        version_line(),
        format!("miyu-applet {}", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn every_help_page_names_both_flags() {
    for (lang, usage) in [
        (Lang::Zh, "用法：miyu-applet"),
        (Lang::En, "Usage: miyu-applet"),
        (Lang::Ja, "使い方：miyu-applet"),
    ] {
        let page = help(lang);
        assert!(page.starts_with(usage), "{lang:?}");
        assert!(page.contains("-h, --help"), "{lang:?}");
        assert!(page.contains("-V, --version"), "{lang:?}");
        assert!(page.ends_with('\n') && !page.ends_with("\n\n"), "{lang:?}");
    }
}
