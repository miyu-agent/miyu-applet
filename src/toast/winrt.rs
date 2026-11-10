//! Windows 的通知自己拼（AP-4 补，2026-11-04 Windows 虚拟机上测出来的）：notify-rust 底下的 tauri-winrt-notification 拼的 toast
//! 点正文、收进通知中心以后再点，Windows 都不交回托盘（只有带 `arguments` 的按钮在横幅上时交回 `Activated`）。没打包的程序要通知中心里
//! 点得回来得注册 COM 激活器（仓库不许 `unsafe`），这里改成照网址激活：
//! - 正文、「打开」按钮都是 `activationType="protocol"`，网址是 [`crate::protocol::link`]（`miyu-applet://open/?session=<编号>`）：点了系统照
//!   当前用户登记的 `miyu-applet` 协议拉起 `miyu-applet.exe --open <网址>`，有托盘在跑的交给它（单实例），没在跑的起来就打开那个会话。横幅上、
//!   通知中心里、托盘退了以后点都一样。不用等事件，弹出去就走。
//! - 同一个 `tag` 的放在同一组（`Tag` 截到 64 个字、`Group`）：下一条换掉上一条，不再摞起来。
//!
//! 都照 WinRT 的接口（`windows` 这个 crate，安全的 Rust）。

use windows::Data::Xml::Dom::XmlDocument;
use windows::UI::Notifications::{ToastNotification, ToastNotificationManager};
use windows::core::HSTRING;

/// 同一个应用的通知都放在这一组里，组里照 `tag` 换。
const GROUP: &str = "miyu";

/// 弹一条：点了去 `link`；`image` 是人格的头像文件（AP-20），没有的不带图。`tag` 最长 64 个字（WinRT 的规矩），长了截掉。
///
/// # Errors
///
/// 拼不成、弹不出来：WinRT 的原话。
pub fn show(
    app_id: &str,
    title: &str,
    body: &str,
    open: &str,
    link: &str,
    tag: &str,
    image: Option<&std::path::Path>,
) -> Result<(), String> {
    let doc = XmlDocument::new().map_err(|error| error.to_string())?;
    let image = image.map(file_url);
    doc.LoadXml(&HSTRING::from(template(
        title,
        body,
        open,
        link,
        image.as_deref(),
    )))
    .map_err(|error| error.to_string())?;
    let toast =
        ToastNotification::CreateToastNotification(&doc).map_err(|error| error.to_string())?;
    let short: String = tag.chars().take(64).collect();
    toast
        .SetTag(&HSTRING::from(short))
        .map_err(|error| error.to_string())?;
    toast
        .SetGroup(&HSTRING::from(GROUP))
        .map_err(|error| error.to_string())?;
    ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(app_id))
        .and_then(|notifier| notifier.Show(&toast))
        .map_err(|error| error.to_string())
}

/// toast 的 XML：标题、正文两行；有头像的放在 `appLogoOverride`（圆形裁剪，`image` 是文件地址）；正文、「打开」按钮点了都照网址激活，
/// 去 `link`。
pub fn template(title: &str, body: &str, open: &str, link: &str, image: Option<&str>) -> String {
    let link = escape(link);
    let image = image.map_or_else(String::new, |src| {
        format!(
            "<image placement=\"appLogoOverride\" hint-crop=\"circle\" src=\"{}\"/>",
            escape(src)
        )
    });
    format!(
        "<toast launch=\"{link}\" activationType=\"protocol\"><visual><binding template=\"ToastGeneric\">\
         <text>{}</text><text>{}</text>{image}</binding></visual><actions>\
         <action content=\"{}\" arguments=\"{link}\" activationType=\"protocol\"/></actions></toast>",
        escape(title),
        escape(body),
        escape(open)
    )
}

/// 本机文件的地址（`file:///C:/…`）：反斜杠换成斜杠。
pub fn file_url(path: &std::path::Path) -> String {
    let text = path.to_string_lossy().replace('\\', "/");
    if text.starts_with('/') {
        format!("file://{text}")
    } else {
        format!("file:///{text}")
    }
}

/// XML 里要转的五个字。
fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests;
