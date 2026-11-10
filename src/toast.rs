//! 弹系统通知（图纸 `app.md` 第 5 条，施工 AP-4），照 `notify-rust`：Linux 是会话 D-Bus 上的 `org.freedesktop.Notifications`，
//! macOS 是 NSUserNotificationCenter，Windows 是 WinRT 的 toast。一条一个后台线程：发出去，等人点了或者关了；点了（通知本身、
//! 「打开」按钮）照 [`Toaster::new`] 给的回调打开那个会话。
//!
//! 标题、正文、头像照 [`Note`]（AP-20，2026-11-10 项目主人：系统通知照会话的人格来）：标题是人格的名字，无人格的写「Miyu」；人格有头像的
//! 把图写进托盘自己目录的 `avatars/<版本>.<扩展名>`，交给通知当图；没头像的、无人格的不带图（原来带托盘的 logo，不再带）。
//!
//! 三个平台做得到的不一样（关在这里，别处不分平台）：
//! - Linux：头像走 `image-path` 提示，带 `desktop-entry` 提示（`miyu-applet`，同安装包的 `.desktop`）；同一个
//!   `tag` 的换掉上一条：记着每个 `tag` 上一条的编号，下一条带着它发（`replaces_id`）。点了是通知服务发回的 `ActionInvoked`
//!   （默认动作 `default`）。
//! - Windows：toast 照应用身份（AUMID）[`APP_ID`] 发，系统照开始菜单里带这个身份的快捷方式（Tauri 打包的 MSI 自带）写名字、画图标；
//!   没装（直接跑编出来的程序）的不一定弹得出。不走 notify-rust，自己拼（`winrt`，AP-4 补：库拼的没有 `launch`，点正文、通知中心里点
//!   都回不来）：正文和「打开」按钮点了都交回 `Activated`，同一个 `tag` 的换掉上一条。只在托盘还在跑时回得来。
//! - Windows 的头像放在 toast 的 `appLogoOverride`（圆形裁剪），照文件地址读。
//! - macOS：照 [`APP_ID`] 认装好的 `Miyu.app`，图标照它；找不到的照库的默认（访达）。头像走库的 `image_path`（`content_image`，横幅
//!   右边一张图；左边的应用图标换不了）。点了要等：带一个「打开」按钮，库才等人点。不换掉上一条。

#[cfg(not(windows))]
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
#[cfg(not(windows))]
use std::sync::{Mutex, PoisonError};

#[cfg(not(windows))]
use notify_rust::{Notification, NotificationResponse};

use crate::alerts::{Avatar, Note};
use crate::log;
use crate::text::{Text, key};

/// 应用身份：Tauri 打包配置的 `identifier`（`tauri.conf.json`）。Windows 安装包开始菜单的快捷方式带的 `System.AppUserModel.ID`
/// 就是它（`tauri-bundler` 2.10.4 的 `main.wxs`），macOS 是 `Miyu.app` 的 `CFBundleIdentifier`。Linux 不用：通知服务照 `desktop-entry`。
#[cfg(any(windows, target_os = "macos"))]
const APP_ID: &str = "io.github.miyu-agent.miyu-applet";

/// 点了通知：要打开的会话（账号级的提醒是 `None`，打开网页界面）。
type OnClick = Box<dyn Fn(Option<String>) + Send + Sync>;

/// 弹通知的。复制出去的是同一个。
#[derive(Clone)]
pub struct Toaster(Arc<Shared>);

struct Shared {
    text: Arc<Text>,
    /// 托盘自己的目录：头像写在它的 `avatars/` 里。
    own: Option<PathBuf>,
    #[cfg_attr(
        windows,
        expect(
            dead_code,
            reason = "Windows 上点了通知照 miyu-applet: 网址拉起托盘，不回调"
        )
    )]
    on_click: OnClick,
    #[cfg(not(windows))]
    shown: Mutex<Shown>,
}

impl Toaster {
    /// `own` 是托盘自己的目录（放通知的图标）。
    pub fn new(
        text: Arc<Text>,
        own: Option<&Path>,
        on_click: impl Fn(Option<String>) + Send + Sync + 'static,
    ) -> Self {
        platform::prepare();
        Self(Arc::new(Shared {
            text,
            own: own.map(Path::to_path_buf),
            on_click: Box::new(on_click),
            #[cfg(not(windows))]
            shown: Mutex::new(Shown::default()),
        }))
    }

    /// 在后台线程里弹一条，等人点。
    pub fn show(&self, note: Note) {
        let shared = Arc::clone(&self.0);
        let started = std::thread::Builder::new()
            .name("notification".into())
            .spawn(move || show_now(&shared, &note));
        if let Err(error) = started {
            log::line(&format!("could not start a notification: {error}"));
        }
    }
}

/// Windows：自己拼 toast（`winrt`），点了照 `miyu-applet:` 网址拉起托盘、打开那个会话（`protocol`），不用回调。
#[cfg(windows)]
fn show_now(shared: &Shared, note: &Note) {
    let body = note.body(shared.text.get(note.why.key()));
    let link = crate::protocol::link(note.session.as_deref());
    let image = note
        .avatar()
        .and_then(|avatar| avatar_file(shared.own.as_deref(), avatar));
    if let Err(error) = winrt::show(
        APP_ID,
        note.heading(),
        &body,
        shared.text.get(key::NOTIFY_OPEN),
        &link,
        &note.tag,
        image.as_deref(),
    ) {
        log::now_and_then("notification", &format!("notification not shown: {error}"));
    }
}

#[cfg(not(windows))]
fn show_now(shared: &Shared, note: &Note) {
    let mut notification = Notification::new();
    let body = note.body(shared.text.get(note.why.key()));
    notification
        .appname("Miyu")
        .summary(note.heading())
        .body(&body)
        .action("default", shared.text.get(key::NOTIFY_OPEN));
    let replaces = shared.shown().replaces(&note.tag);
    let image = note
        .avatar()
        .and_then(|avatar| avatar_file(shared.own.as_deref(), avatar));
    platform::dress(&mut notification, image.as_deref(), replaces);
    let handle = match notification.show() {
        Ok(handle) => handle,
        Err(error) => {
            log::now_and_then("notification", &format!("notification not shown: {error}"));
            return;
        }
    };
    let id = platform::id_of(&handle);
    let mine = id.map(|id| shared.shown().sent(&note.tag, id));
    let waited = handle.wait_for_response(|response: &NotificationResponse| {
        let clicked = match response {
            NotificationResponse::Default => true,
            NotificationResponse::Action(action) => action == "default",
            NotificationResponse::Reply(_) | NotificationResponse::Closed(_) => false,
        };
        let closed = matches!(response, NotificationResponse::Closed(_));
        // 换掉过的：等旧那一条的线程也收得到这一下，只认最新的那个，免得打开两次。
        let latest = match (id, mine) {
            (Some(id), Some(mine)) => shared.shown().answered(id, mine, closed),
            _ => true,
        };
        if clicked && latest {
            log::line(&format!("notification {} clicked", note.tag));
            (shared.on_click)(note.session.clone());
        }
    });
    if let Err(error) = waited {
        log::line(&format!("notification {} not followed: {error}", note.tag));
    }
}

/// 头像写成文件（通知服务、系统照路径读图）：托盘自己目录的 `avatars/<版本>.<扩展名>`，已经有一样的不再写；没有托盘目录、写不成的
/// 不带图（记一行）。
fn avatar_file(own: Option<&Path>, avatar: &Avatar) -> Option<PathBuf> {
    let dir = own?.join("avatars");
    let name: String = avatar
        .version
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .collect();
    if name.is_empty() {
        return None;
    }
    let path = dir.join(format!("{name}.{}", avatar.ext));
    if std::fs::read(&path).is_ok_and(|bytes| bytes == avatar.bytes) {
        return Some(path);
    }
    let written = crate::folder::private(&dir).and_then(|()| std::fs::write(&path, &avatar.bytes));
    match written {
        Ok(()) => Some(path),
        Err(error) => {
            log::line(&format!("no avatar for the notification: {error}"));
            None
        }
    }
}

#[cfg(not(windows))]
impl Shared {
    fn shown(&self) -> std::sync::MutexGuard<'_, Shown> {
        self.shown.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(not(windows))]
/// 发出去还没关的那几条（只有给编号的平台有）：每个 `tag` 上一条的编号；每个编号最新一次是第几次发的。换掉一条以后等旧那一次的
/// 线程还在等同一个编号，点了两个线程都收得到，只有最新的那一次算数。
#[derive(Debug, Default)]
struct Shown {
    by_tag: HashMap<String, u32>,
    latest: HashMap<u32, u64>,
    sends: u64,
}

#[cfg(not(windows))]
impl Shown {
    /// 这个 `tag` 上一条还开着的编号。
    fn replaces(&self, tag: &str) -> Option<u32> {
        self.by_tag.get(tag).copied()
    }

    /// 发出去了，编号是 `id`：交回这是第几次。
    fn sent(&mut self, tag: &str, id: u32) -> u64 {
        self.sends += 1;
        self.by_tag.insert(tag.to_owned(), id);
        self.latest.insert(id, self.sends);
        self.sends
    }

    /// 第 `send` 次发的、编号 `id` 的那一条有了回音（`closed` 是关掉了）：交回它是不是最新的那一次；最新的关掉了就忘了它，同一个
    /// `tag` 的下一条另起一条。
    fn answered(&mut self, id: u32, send: u64, closed: bool) -> bool {
        let latest = self.latest.get(&id) == Some(&send);
        if latest && closed {
            self.latest.remove(&id);
            self.by_tag.retain(|_, shown| *shown != id);
        }
        latest
    }
}

/// Linux 这一类（freedesktop 的通知服务）。
#[cfg(all(unix, not(target_os = "macos")))]
mod platform {
    use std::path::Path;

    use notify_rust::{Hint, Notification, NotificationHandle};

    /// 起来时不用做什么（原来把 logo 写进托盘目录当通知的图标；AP-20 起没头像的不带图）。
    pub fn prepare() {}

    /// 头像走 `image-path` 提示；带 `desktop-entry` 提示；同一个 `tag` 的换掉上一条。
    pub fn dress(notification: &mut Notification, image: Option<&Path>, replaces: Option<u32>) {
        if let Some(image) = image.and_then(Path::to_str) {
            notification.image_path(image);
        }
        notification.hint(Hint::DesktopEntry("miyu-applet".to_owned()));
        if let Some(id) = replaces {
            notification.id(id);
        }
    }

    pub fn id_of(handle: &NotificationHandle) -> Option<u32> {
        Some(handle.id())
    }
}

/// Windows：应用图标照开始菜单快捷方式的，不用另写；通知自己拼（[`winrt`]）。
#[cfg(windows)]
mod platform {
    pub fn prepare() {}
}

#[cfg(windows)]
mod winrt;

/// macOS：起来时认一次 `Miyu.app`。
#[cfg(target_os = "macos")]
mod platform {
    use std::path::Path;

    use notify_rust::{Notification, NotificationHandle};

    use crate::log;

    pub fn prepare() {
        if let Err(error) = notify_rust::set_application(super::APP_ID) {
            log::line(&format!("notifications go out as the default app: {error}"));
        }
    }

    /// 头像走库的 `image_path`（`content_image`）；不换掉上一条。
    pub fn dress(notification: &mut Notification, image: Option<&Path>, _: Option<u32>) {
        if let Some(image) = image.and_then(Path::to_str) {
            notification.image_path(image);
        }
    }

    pub fn id_of(_: &NotificationHandle) -> Option<u32> {
        None
    }
}

#[cfg(all(test, not(windows)))]
mod tests;
