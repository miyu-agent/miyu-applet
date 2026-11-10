//! toast 的 XML（AP-4 补）：正文和按钮都照网址激活、去同一个网址；字照 XML 转义。

use super::*;

const LINK: &str = "miyu-applet://open/?session=01a1a520-30b6-702a-ada5-603e4d8a21e8";

#[test]
fn the_body_and_the_button_both_open_the_link() {
    let xml = template("修 bug", "回答完了", "打开", LINK, None);
    assert!(
        xml.starts_with(&format!(
            "<toast launch=\"{LINK}\" activationType=\"protocol\">"
        )),
        "{xml}"
    );
    assert!(
        xml.contains("<text>修 bug</text><text>回答完了</text>"),
        "{xml}"
    );
    assert!(
        xml.contains(&format!(
            "<action content=\"打开\" arguments=\"{LINK}\" activationType=\"protocol\"/>"
        )),
        "{xml}"
    );
}

#[test]
fn text_is_escaped_for_xml() {
    let xml = template("a<b> & \"c\" 'd'", "x", "y", "miyu-applet://open/", None);
    assert!(
        xml.contains("<text>a&lt;b&gt; &amp; &quot;c&quot; &apos;d&apos;</text>"),
        "{xml}"
    );
}

#[test]
fn the_document_loads() {
    let doc = XmlDocument::new().expect("XmlDocument");
    doc.LoadXml(&HSTRING::from(template(
        "t",
        "b",
        "o",
        "miyu-applet://open/?session=x&y=1",
        Some("file:///C:/Users/me/AppData/a&b.png"),
    )))
    .expect("拼的 XML 读得进（网址、图的地址里的 & 转过）");
}

/// 人格有头像的（AP-20）：放在 appLogoOverride，圆形裁剪，照文件地址；没头像的不带 image。
#[test]
fn an_avatar_goes_into_the_app_logo_override() {
    let path = std::path::Path::new(r"C:\Users\me\AppData\Local\miyu-applet\avatars\0123abcd.png");
    let url = file_url(path);
    assert_eq!(
        url,
        "file:///C:/Users/me/AppData/Local/miyu-applet/avatars/0123abcd.png"
    );
    let xml = template("美雪", "修 bug · 回答完了", "打开", LINK, Some(&url));
    assert!(
        xml.contains(&format!(
            "<image placement=\"appLogoOverride\" hint-crop=\"circle\" src=\"{url}\"/>"
        )),
        "{xml}"
    );
    assert!(!template("Miyu", "回答完了", "打开", LINK, None).contains("<image"));
}
