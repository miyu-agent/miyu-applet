//! 本机文件的地址（`file:///C:/…`）：Windows 的通知照它读头像（`winrt`）。纯的字符串处理，测试在哪个系统上都跑。

/// 本机文件的地址（`file:///C:/…`）：反斜杠换成斜杠；`\\?\` 开头的长路径写法去掉前缀（`\\?\UNC\` 的照 `\\服务器\共享`）；`%`、`#`
/// 转成 `%25`、`%23`（在地址里一个开头转义、一个开头片段，原来照原样放，用户名带它们的读不到图，审计 #099）。
pub fn file_url(path: &std::path::Path) -> String {
    let text = path.to_string_lossy();
    let text = match text.strip_prefix(r"\\?\") {
        Some(rest) => match rest.strip_prefix(r"UNC\") {
            Some(share) => format!(r"\\{share}"),
            None => rest.to_owned(),
        },
        None => text.into_owned(),
    };
    let text = text
        .replace('%', "%25")
        .replace('#', "%23")
        .replace('\\', "/");
    if let Some(share) = text.strip_prefix("//") {
        format!("file://{share}")
    } else if text.starts_with('/') {
        format!("file://{text}")
    } else {
        format!("file:///{text}")
    }
}

#[cfg(test)]
mod tests;
