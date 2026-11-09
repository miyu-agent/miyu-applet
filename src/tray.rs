//! 托盘图标和菜单（图纸 `app.md` 第 3、4 条，2026-11-03 项目主人看过的样板）：从上往下
//!
//! 1. 状态行（灰、不能点，前面一个小圆点）；
//! 2. 「停止」或「启动」、「重新启动」（带图标；不能用时灰着，不藏）；
//! 3. 「打开网页界面」（默认项）、「设置…」（macOS 带 ⌘,）；
//! 4. 软件包挂的入口（核心 TR-2，施工 AP-3）：一个包一行，一个都没有的不出这一组；
//! 5. 局域网（施工 AP-3）：「局域网访问」（打勾；只在运行中能点），开着时多一行「在手机上打开…」；核心说不认 `web.listen` 的、
//!    还没连上过的不出这一组；
//! 6. 「开机时启动」（打勾）、「打开日志所在的文件夹」；
//! 7. 「关于 Miyu」、「退出 Miyu」（带图标；macOS 带 ⌘Q）。
//!
//! 点图标：Windows、Linux 左键「打开网页界面」、右键出菜单；macOS 点了出菜单。Linux 上托盘库走 ksni 后端（施工 AP-3）：直接说
//! StatusNotifierItem，左键是宿主调的 `Activate`，右键宿主照 dbusmenu 自己画菜单。菜单的事件在 `app` 里统一接。

use tray_icon::menu::{
    CheckMenuItem, Icon as MenuIcon, IconMenuItem, Menu, MenuItem, PredefinedMenuItem,
};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

use crate::extras::Entry;
use crate::icon::{self, MENU_SIZE};
use crate::status::{Look, Toggle};
use crate::text::{Text, key};

/// 菜单项的编号。
pub mod id {
    pub const STATUS: &str = "status";
    pub const TOGGLE: &str = "toggle";
    pub const RESTART: &str = "restart";
    pub const OPEN: &str = "open";
    pub const SETTINGS: &str = "settings";
    pub const LAN: &str = "lan";
    pub const PHONE: &str = "phone";
    pub const AUTOSTART: &str = "autostart";
    pub const LOGS: &str = "logs";
    pub const ABOUT: &str = "about";
    pub const QUIT: &str = "quit";
    /// 软件包挂的入口：后面接包的编号。
    pub const PACKAGE: &str = "package:";
}

/// 建好的托盘：跟着状态变的几行拿着，好改。
pub struct Tray {
    icon: TrayIcon,
    items: Items,
    stop_icon: Option<MenuIcon>,
    start_icon: Option<MenuIcon>,
}

/// 菜单里跟着状态变的几行。菜单整个换掉（软件包的入口变了）时跟着换。
struct Items {
    status: IconMenuItem,
    toggle: IconMenuItem,
    restart: IconMenuItem,
    autostart: CheckMenuItem,
    /// 「局域网访问」：这一组不出的时候没有。
    lan: Option<CheckMenuItem>,
}

/// 照核心来的、变了要把菜单整个换掉的几行。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Extras {
    /// 软件包挂的入口。
    pub packages: Vec<Entry>,
    /// 局域网开没开（`web.listen`）；`None` 是还不知道、核心不认（网页软件没装）：不出这一组。
    pub lan: Option<bool>,
}

/// 解一张编进程序的菜单图标；解不出的（不会：测试查过）不带图标。
fn menu_icon(bytes: &[u8]) -> Option<MenuIcon> {
    let image = icon::decode(bytes).ok()?;
    MenuIcon::from_rgba(image.pixels, image.width, image.height).ok()
}

/// 给一行放图标：macOS 上当模板图（系统照菜单着色），别的照原样画。
fn put_icon(item: &IconMenuItem, icon: Option<MenuIcon>) {
    #[cfg(target_os = "macos")]
    item.set_icon_templated(icon);
    #[cfg(not(target_os = "macos"))]
    item.set_icon(icon);
}

/// macOS 上的快捷键（只在菜单里显示、菜单开着时按得到）；别的平台托盘菜单不带。
#[cfg(target_os = "macos")]
fn shortcut(keys: &str) -> Option<tray_icon::menu::accelerator::Accelerator> {
    keys.parse().ok()
}
#[cfg(not(target_os = "macos"))]
fn shortcut(_: &str) -> Option<tray_icon::menu::accelerator::Accelerator> {
    None
}

/// 照 `extras` 建一份菜单（还没画状态）。
fn menu(text: &Text, autostart: bool, extras: &Extras) -> Result<(Menu, Items), String> {
    let status = IconMenuItem::with_id(id::STATUS, "", false, None, None);
    let toggle = IconMenuItem::with_id(id::TOGGLE, "", false, None, None);
    let restart = IconMenuItem::with_id(id::RESTART, text.get(key::RESTART), false, None, None);
    put_icon(&restart, menu_icon(icon::RESTART));
    let open = MenuItem::with_id(id::OPEN, text.get(key::OPEN), true, None);
    let settings = MenuItem::with_id(
        id::SETTINGS,
        text.get(key::SETTINGS),
        true,
        shortcut("CmdOrCtrl+Comma"),
    );
    let autostart_item = CheckMenuItem::with_id(
        id::AUTOSTART,
        text.get(key::AUTOSTART),
        true,
        autostart,
        None,
    );
    let logs = MenuItem::with_id(id::LOGS, text.get(key::LOGS), true, None);
    let about = MenuItem::with_id(id::ABOUT, text.get(key::ABOUT), true, None);
    let quit = IconMenuItem::with_id(
        id::QUIT,
        text.get(key::QUIT),
        true,
        None,
        shortcut("CmdOrCtrl+Q"),
    );
    put_icon(&quit, menu_icon(icon::QUIT));
    // 软件包挂的入口：清单写了 `[tray]`、开着的包一个一行，字是核心照连接的语言挑好的，点了 `miyu pkg page <编号>`。
    let packages: Vec<MenuItem> = extras
        .packages
        .iter()
        .map(|entry| {
            let item_id = format!("{}{}", id::PACKAGE, entry.package);
            MenuItem::with_id(item_id, &entry.name, true, None)
        })
        .collect();

    let menu = Menu::new();
    let separator = PredefinedMenuItem::separator;
    let mut added = vec![
        menu.append(&status),
        menu.append(&toggle),
        menu.append(&restart),
        menu.append(&separator()),
        menu.append(&open),
        menu.append(&settings),
    ];
    if !packages.is_empty() {
        added.push(menu.append(&separator()));
        added.extend(packages.iter().map(|item| menu.append(item)));
    }
    // 局域网：「局域网访问」打勾的开关，开着时多一行「在手机上打开…」（用浏览器打开网页设置「访问」那一组，地址和二维码在那里）。
    let lan = extras
        .lan
        .map(|on| CheckMenuItem::with_id(id::LAN, text.get(key::LAN), false, on, None));
    let phone = MenuItem::with_id(id::PHONE, text.get(key::PHONE), true, None);
    if let Some(lan) = &lan {
        added.push(menu.append(&separator()));
        added.push(menu.append(lan));
        if extras.lan == Some(true) {
            added.push(menu.append(&phone));
        }
    }
    added.extend([
        menu.append(&separator()),
        menu.append(&autostart_item),
        menu.append(&logs),
        menu.append(&separator()),
        menu.append(&about),
        menu.append(&quit),
    ]);
    if let Some(error) = added.into_iter().find_map(Result::err) {
        return Err(error.to_string());
    }
    let items = Items {
        status,
        toggle,
        restart,
        autostart: autostart_item,
        lan,
    };
    Ok((menu, items))
}

/// 建托盘图标和菜单，先照 `look` 画一遍。
///
/// # Errors
/// 建不出（Linux 上连不上会话的 D-Bus）。
pub fn create(text: &Text, look: &Look, autostart: bool, extras: &Extras) -> Result<Tray, String> {
    let (menu, items) = menu(text, autostart, extras)?;
    let image = icon::decode(icon::TRAY)?;
    let tray_icon =
        Icon::from_rgba(image.pixels, image.width, image.height).map_err(|e| e.to_string())?;
    // macOS 上是模板图，系统照菜单栏着色。
    #[cfg(target_os = "macos")]
    let builder = TrayIconBuilder::new().with_icon_templated(tray_icon);
    #[cfg(not(target_os = "macos"))]
    let builder = TrayIconBuilder::new().with_icon(tray_icon);
    let built = builder
        .with_id("miyu")
        .with_tooltip("Miyu")
        .with_menu(Box::new(menu))
        // macOS 点了出菜单；Windows 左键交给程序（打开网页界面），右键出菜单。Linux 的 ksni 后端不看这一项：左键总是报给程序。
        .with_menu_on_left_click(cfg!(target_os = "macos"))
        .build()
        .map_err(|error| error.to_string())?;
    let tray = Tray {
        icon: built,
        items,
        stop_icon: menu_icon(icon::STOP),
        start_icon: menu_icon(icon::START),
    };
    tray.show(text, look);
    Ok(tray)
}

impl Tray {
    /// 照 `look` 改状态行和起停那两行。
    pub fn show(&self, text: &Text, look: &Look) {
        let items = &self.items;
        let status = match look.reason.as_deref().map(str::trim) {
            Some(reason) if !reason.is_empty() => format!("{}\n{reason}", text.get(look.status)),
            _ => text.get(look.status).to_owned(),
        };
        items.status.set_text(status);
        let dot = icon::dot(look.dot.rgb(), MENU_SIZE);
        // 圆点带颜色，不当模板图。
        items
            .status
            .set_icon(MenuIcon::from_rgba(dot.pixels, dot.width, dot.height).ok());
        let (label, image) = match look.toggle {
            Toggle::Stop => (key::STOP, &self.stop_icon),
            Toggle::Start => (key::START, &self.start_icon),
        };
        items.toggle.set_text(text.get(label));
        put_icon(&items.toggle, image.clone());
        items.toggle.set_enabled(look.toggle_enabled);
        items.restart.set_enabled(look.restart_enabled);
        if let Some(lan) = &items.lan {
            lan.set_enabled(look.lan_enabled);
        }
    }

    /// 照核心来的几行变了：整个菜单换一份，再照 `look` 画一遍。「开机时启动」照现在打的勾。
    pub fn rebuild(&mut self, text: &Text, look: &Look, extras: &Extras) {
        let autostart = self.items.autostart.is_checked();
        match menu(text, autostart, extras) {
            Ok((menu, items)) => {
                self.icon.set_menu(Some(Box::new(menu)));
                self.items = items;
                self.show(text, look);
            }
            Err(error) => crate::log::line(&format!("menu not rebuilt: {error}")),
        }
    }

    /// 「开机时启动」打不打勾。
    pub fn set_autostart(&self, on: bool) {
        self.items.autostart.set_checked(on);
    }

    /// 「局域网访问」打不打勾：点了托盘库自己先翻过去，照知道的翻回来，等核心推来的变动再改。
    pub fn set_lan(&self, on: bool) {
        if let Some(lan) = &self.items.lan {
            lan.set_checked(on);
        }
    }
}
