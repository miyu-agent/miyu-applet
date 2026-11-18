//! 点通知用的网址（AP-4 补）：拼、读、照命令行找；外面交进来的写法不对的不认。

use super::*;

const ID: &str = "01a1a520-30b6-702a-ada5-603e4d8a21e8";

#[test]
fn links_round_trip() {
    assert_eq!(link(Some(ID)), format!("miyu-applet://open/?session={ID}"));
    assert_eq!(parse(&link(Some(ID))), Some(Some(ID.to_owned())));
    assert_eq!(link(None), "miyu-applet://open/");
    assert_eq!(parse(&link(None)), Some(None));
    assert_eq!(
        parse("miyu-applet://open"),
        Some(None),
        "末尾没有斜杠的也认"
    );
}

#[test]
fn strangers_are_refused() {
    assert_eq!(parse("http://open/?session=x"), None);
    assert_eq!(parse("miyu-applet://delete/"), None);
    assert_eq!(
        parse("miyu-applet://open/?session=../../x"),
        None,
        "编号写法不对"
    );
    assert_eq!(
        parse("miyu-applet://open/?session=01a1a520-30b6-702a-ada5-603e4d8a21e8&x=1"),
        None
    );
}

#[test]
fn the_data_root_comes_from_the_command_line() {
    let args = [
        HOME_ARG.to_owned(),
        r"C:\mt\tray-home".to_owned(),
        ARG.to_owned(),
        link(None),
    ];
    assert_eq!(
        home_from_args(args.clone()),
        Some(std::path::PathBuf::from(r"C:\mt\tray-home"))
    );
    assert_eq!(from_args(args), Some(None));
    assert_eq!(
        home_from_args([HOME_ARG.to_owned(), String::new()]),
        None,
        "空的不算"
    );
}

#[test]
fn the_link_comes_from_the_command_line() {
    let args = ["--autostart".to_owned(), ARG.to_owned(), link(Some(ID))];
    assert_eq!(from_args(args), Some(Some(ID.to_owned())));
    assert_eq!(from_args([ARG.to_owned()]), None, "后面没有网址");
    assert_eq!(from_args(["x".to_owned()]), None);
}

/// 审计 #099：数据根末尾带反斜杠的（`D:\miyu\`、盘的根 `D:\`），系统拆参数时 `\"` 是字面的引号，原来吞掉了后面的 `--open "%1"`。
#[test]
fn a_trailing_backslash_in_the_data_root_keeps_the_link_argument() {
    let exe = std::path::Path::new(r"C:\Program Files\Miyu\miyu-applet.exe");
    for home in [r"D:\miyu\", r"D:\", r"D:\Miyu data"] {
        assert_eq!(
            crate::argv::split(&open_command(exe, std::path::Path::new(home))),
            [
                r"C:\Program Files\Miyu\miyu-applet.exe",
                HOME_ARG,
                home,
                ARG,
                "%1"
            ],
            "{home}"
        );
    }
}

#[test]
fn quoted_arguments_come_back_as_they_were() {
    for arg in [r"a\b", r"a\\", "a\"b", r"a\", "", "a b", r"\\server\share\"] {
        assert_eq!(
            crate::argv::split(&format!("x {}", quoted(arg))),
            ["x", arg],
            "{arg}"
        );
    }
}
