use super::*;

use std::path::Path;

#[test]
fn a_windows_path_becomes_a_file_url() {
    assert_eq!(
        file_url(Path::new(
            r"C:\Users\me\AppData\Local\miyu-applet\avatars\0123abcd.png"
        )),
        "file:///C:/Users/me/AppData/Local/miyu-applet/avatars/0123abcd.png"
    );
}

/// 审计 #099：用户名、目录名里的 `#`、`%` 在地址里另有意思（`#` 后面是片段、`%` 开头是转义），要转成 `%23`、`%25`；
/// `\\?\` 开头的长路径写法去掉前缀（原来拼成 `file:////?/C:/…`，不是合法的地址）。
#[test]
fn hashes_percents_and_long_path_prefixes_are_handled() {
    assert_eq!(
        file_url(Path::new(
            r"C:\Users\a#b\AppData\Local\miyu-applet\avatars\x.png"
        )),
        "file:///C:/Users/a%23b/AppData/Local/miyu-applet/avatars/x.png"
    );
    assert_eq!(
        file_url(Path::new(r"C:\Users\50%off\x.png")),
        "file:///C:/Users/50%25off/x.png"
    );
    assert_eq!(
        file_url(Path::new(r"\\?\C:\Users\me\x.png")),
        "file:///C:/Users/me/x.png"
    );
    assert_eq!(
        file_url(Path::new(r"\\?\UNC\server\share\x.png")),
        "file://server/share/x.png"
    );
}
