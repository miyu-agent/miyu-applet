use super::*;

/// 路径照 Unix 的写法（Windows 上 `/x` 不算绝对路径）。
#[cfg(unix)]
#[test]
fn the_linux_file_follows_xdg_config_home() {
    assert_eq!(
        desktop_file(Some("/x/conf".into()), Some("/home/u".into())),
        Some(PathBuf::from("/x/conf/autostart/miyu-applet.desktop"))
    );
    assert_eq!(
        desktop_file(None, Some("/home/u".into())),
        Some(PathBuf::from(
            "/home/u/.config/autostart/miyu-applet.desktop"
        ))
    );
    // 相对的 XDG_CONFIG_HOME 不认（规范：只认绝对路径）。
    assert_eq!(
        desktop_file(Some("conf".into()), Some("/home/u".into())),
        Some(PathBuf::from(
            "/home/u/.config/autostart/miyu-applet.desktop"
        ))
    );
    assert_eq!(desktop_file(None, None), None);
}

#[test]
fn the_desktop_entry_starts_the_tray_without_a_browser() {
    let entry = desktop_entry(Path::new("/usr/bin/miyu-applet"), None);
    assert!(entry.starts_with("[Desktop Entry]\n"));
    assert!(entry.contains("\nExec=/usr/bin/miyu-applet --autostart\n"));
    assert!(entry.contains("\nType=Application\n"));
    assert!(entry.contains("\nName=Miyu\n"));
}

#[test]
fn awkward_program_paths_are_quoted() {
    assert_eq!(
        desktop_exec("/opt/My Apps/miyu-applet"),
        "\"/opt/My Apps/miyu-applet\""
    );
    assert_eq!(
        desktop_exec("/opt/$x/miyu-applet"),
        "\"/opt/\\\\$x/miyu-applet\""
    );
    assert_eq!(
        desktop_exec("/opt/100%/miyu-applet"),
        "/opt/100%%/miyu-applet"
    );
}

#[cfg(unix)]
#[test]
fn the_launch_agent_file_is_in_the_home() {
    assert_eq!(
        launch_agent_file(Some("/Users/u".into())),
        Some(PathBuf::from(
            "/Users/u/Library/LaunchAgents/io.github.miyu-agent.miyu-applet.plist"
        ))
    );
    assert_eq!(launch_agent_file(None), None);
}

#[test]
fn the_launch_agent_runs_at_login_with_the_argument() {
    let plist = launch_agent(
        Path::new("/Applications/Miyu & Co.app/Contents/MacOS/miyu-applet"),
        None,
    );
    assert!(plist.contains("<string>io.github.miyu-agent.miyu-applet</string>"));
    assert!(plist.contains(
        "<string>/Applications/Miyu &amp; Co.app/Contents/MacOS/miyu-applet</string>\n    <string>--autostart</string>"
    ));
    assert!(plist.contains("<key>RunAtLoad</key>\n  <true/>"));
}

/// 托盘照别的数据根起的（施工 AP-14、AP-17）：开机也照它起，三个平台都写 `--home`；默认的不加。
#[test]
fn a_data_root_set_at_start_goes_into_the_login_item() {
    let entry = desktop_entry(
        Path::new("/home/u/.local/lib/miyu-applet/bin/miyu-applet"),
        Some(Path::new("/home/u/.cache/miyu home")),
    );
    assert!(entry.contains(
        "\nExec=/home/u/.local/lib/miyu-applet/bin/miyu-applet --home \"/home/u/.cache/miyu home\" --autostart\n"
    ));
    assert!(!desktop_entry(Path::new("/usr/bin/miyu-applet"), None).contains("--home"));
    let plist = launch_agent(
        Path::new("/Applications/Miyu.app/Contents/MacOS/miyu-applet"),
        Some(Path::new("/Users/u/Miyu & data")),
    );
    assert!(plist.contains(
        "<string>/Applications/Miyu.app/Contents/MacOS/miyu-applet</string>\n    <string>--home</string>\n    <string>/Users/u/Miyu &amp; data</string>\n    <string>--autostart</string>"
    ));
    assert!(!plist.contains("EnvironmentVariables"));
    assert!(
        !launch_agent(
            Path::new("/Applications/Miyu.app/Contents/MacOS/miyu-applet"),
            None
        )
        .contains("--home")
    );
}

#[test]
fn the_run_value_quotes_the_program() {
    assert_eq!(
        run_value(Path::new(r"C:\Program Files\Miyu\miyu-applet.exe"), None),
        r#""C:\Program Files\Miyu\miyu-applet.exe" --autostart"#
    );
    // 明说的数据根（施工 AP-17）：同网址协议登记的写法
    assert_eq!(
        run_value(
            Path::new(r"C:\Program Files\Miyu\miyu-applet.exe"),
            Some(Path::new(r"D:\Miyu data"))
        ),
        r#""C:\Program Files\Miyu\miyu-applet.exe" --home "D:\Miyu data" --autostart"#
    );
}

#[test]
fn removing_what_is_not_there_is_fine_and_writing_makes_the_folder() {
    let dir = std::env::temp_dir().join(format!("miyu-applet-autostart-{}", std::process::id()));
    let file = dir.join("a").join("miyu-applet.desktop");
    remove(&file).unwrap();
    write(&file, "x").unwrap();
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "x");
    remove(&file).unwrap();
    assert!(!file.exists());
    std::fs::remove_dir_all(&dir).unwrap();
}
