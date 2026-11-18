//! Miyu 的托盘程序 `miyu-applet`（图纸 `docs/blueprint/app.md`）：只有托盘图标（macOS 上是菜单栏图标），没有窗口。界面都在网页里，
//! 托盘调装在旁边的主程序 `miyu` 用浏览器打开它；另照网页服务的样子连核心（不拉起），看它在不在跑。
// 正式版不开控制台窗口（Windows）。
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod about;
mod actions;
mod alerts;
mod app;
#[cfg(test)]
mod argv;
mod autostart;
mod extras;
mod folder;
mod icon;
mod instance;
mod lan;
mod link;
mod log;
mod program;
mod protocol;
mod quit;
mod reason;
mod rpc;
mod status;
mod text;
mod toast;
mod tray;

use std::sync::Arc;

use miyu_client::places::{DataRoot, Env};
use tao::event::{Event as LoopEvent, StartCause};
use tao::event_loop::{ControlFlow, EventLoopBuilder};

use crate::actions::Ctx;
use crate::app::{App, Event, Post};
use crate::instance::Claim;
use crate::text::Text;

fn main() {
    let text = Arc::new(Text::system());
    let autostarted = std::env::args().skip(1).any(|arg| arg == autostart::ARG);
    // 点了通知拉起来的（`--open miyu-applet://open/?session=<编号>`，`protocol`）：要打开的那个会话
    let opened = protocol::from_args(std::env::args().skip(1));
    let mut problem = None;
    // 照网址拉起来的带着登记时的数据根（`--home`，`protocol`）：照它找，不照环境变量
    let mut env = Env::current();
    if let Some(home) = protocol::home_from_args(std::env::args().skip(1)) {
        env.miyu_home = Some(home.into_os_string());
    }
    let root = match DataRoot::locate(&env) {
        Ok(root) => {
            // 拉起的 `miyu` 都照这个数据根（`program::command` 带上 `MIYU_HOME`）
            program::pin_home(root.path());
            Some(root)
        }
        Err(error) => {
            log::line(&format!("no data root: {error}"));
            problem = Some(error.to_string());
            None
        }
    };
    // 数据根的 `var/log/` 核心起过一次才有；没有的等连上核心再定（`app::start_log`）。
    if let Some(logs) = root.as_ref().map(DataRoot::logs)
        && logs.is_dir()
    {
        log::start(logs.join("miyu-applet.log"));
    }
    log::line(&format!(
        "starting {}, autostarted: {autostarted}",
        env!("CARGO_PKG_VERSION")
    ));

    // 单实例：一个数据根一个托盘。第二个起来的叫第一个打开网页界面，自己退。
    let own = folder::own_here();
    let key = instance::key(
        root.as_ref()
            .map_or(std::path::Path::new(""), DataRoot::path),
    );
    let _held = match own.as_deref().map(|dir| (dir, instance::claim(dir, &key))) {
        Some((_, Ok(Claim::First(lock)))) => Some(lock),
        Some((dir, Ok(Claim::Second))) => {
            if autostarted {
                log::line("already running, autostart does nothing");
            } else if let Err(error) = instance::ask(dir, &key, opened.clone().flatten().as_deref())
            {
                log::line(&format!("could not ask the running tray to open: {error}"));
            } else {
                match opened.as_ref().and_then(Option::as_deref) {
                    Some(id) => {
                        log::line(&format!("already running, asked it to open session {id}"))
                    }
                    None => log::line("already running, asked it to open the web interface"),
                }
            }
            return;
        }
        Some((dir, Err(error))) => {
            log::line(&format!("no single instance in {}: {error}", dir.display()));
            None
        }
        None => {
            log::line("no folder of our own, no single instance");
            None
        }
    };

    #[cfg_attr(
        not(target_os = "macos"),
        allow(unused_mut, reason = "只有 macOS 要改")
    )]
    let mut event_loop = EventLoopBuilder::<Event>::with_user_event().build();
    // macOS：不占程序坞、不出应用菜单，只在菜单栏（打包时的 `LSUIElement` 是 AP-5 的事，这里先在运行时设）。
    #[cfg(target_os = "macos")]
    {
        use tao::platform::macos::{ActivationPolicy, EventLoopExtMacOS};
        event_loop.set_activation_policy(ActivationPolicy::Accessory);
    }
    let post = Post::new(event_loop.create_proxy());
    let on_tray = post.clone();
    tray_icon::TrayIconEvent::set_event_handler(Some(move |event| {
        on_tray.send(Event::Tray(event));
    }));
    let on_menu = post.clone();
    tray_icon::menu::MenuEvent::set_event_handler(Some(
        move |event: tray_icon::menu::MenuEvent| {
            on_menu.send(Event::Menu(event.id));
        },
    ));

    let _listening = own.as_deref().and_then(|dir| {
        let asked = post.clone();
        instance::listen(dir, &key, move |session| asked.send(Event::Open(session)))
            .inspect_err(|error| log::line(&format!("not listening for a second start: {error}")))
            .ok()
    });
    let on_link = post.clone();
    let waker = root
        .clone()
        .map(|root| link::spawn(root, move |up| on_link.send(Event::Link(up))));
    let miyu = program::locate_here();
    log::line(&format!("main program found: {}", miyu.is_some()));
    if miyu.is_none() {
        problem.get_or_insert_with(|| text.get(crate::text::key::NO_PROGRAM).to_owned());
    }
    let ctx = Ctx {
        miyu,
        root,
        text: Arc::clone(&text),
        post,
        waker,
    };
    let clicked = ctx.post.clone();
    let toaster = toast::Toaster::new(Arc::clone(&text), own.as_deref(), move |session| {
        clicked.send(Event::Clicked(session));
    });
    // 起来时开什么：点了通知拉起来的开那个会话，开机时起的不开，别的开首页
    let open_at_start = match opened {
        Some(session) => Some(session.map_or(actions::Page::Home, actions::Page::Session)),
        None if autostarted => None,
        None => Some(actions::Page::Home),
    };
    // Windows：点通知照 `miyu-applet:` 网址拉起自己，每次起来照现在的位置登记一遍（`protocol`）
    #[cfg(windows)]
    if let (Some(exe), Some(root)) = (std::env::current_exe().ok(), ctx.root.as_ref())
        && let Err(error) = protocol::register(&exe, root.path())
    {
        log::line(&format!(
            "could not register the miyu-applet: links: {error}"
        ));
    }
    let mut app = App::new(ctx, toaster, open_at_start, problem);
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            LoopEvent::NewEvents(StartCause::Init) => app.started(),
            LoopEvent::UserEvent(event) => app.handle(event, control_flow),
            #[cfg(target_os = "macos")]
            LoopEvent::Reopen { .. } => app.handle(Event::Open(None), control_flow),
            _ => {}
        }
    });
}
