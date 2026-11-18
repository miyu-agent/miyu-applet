//! 托盘程序的界面线程：收托盘、菜单的事件，收后台线程做完的、连接报来的，改状态、改菜单，派活。

use std::sync::Arc;
use std::time::Instant;

use tao::event_loop::{ControlFlow, EventLoopProxy};
use tray_icon::TrayIconEvent;
use tray_icon::menu::MenuId;

use crate::actions::{self, Ctx, Page};
use crate::instance::Once;
use crate::link::Report;
use crate::status::{self, Busy, Linked, Status};
use crate::text::Text;
use crate::toast::Toaster;
use crate::tray::{self, Extras, Tray, id};
use crate::{autostart, log};

/// 交给界面线程的事。
#[derive(Debug)]
pub enum Event {
    /// 点了菜单的一项。
    Menu(MenuId),
    /// 点了托盘图标。
    Tray(TrayIconEvent),
    /// 托盘那条连接报来的。
    Link(Report),
    /// 后台开始做一件起停的事（「重新启动」停完了、开始拉起）。
    Begin(Busy),
    /// 后台做完一件起停的事，带做成了没有。
    Done(Busy, bool),
    /// 出错了：拉不起核心、打不开网页界面，带 `miyu` 说的那一句。状态行断着的时候写「出错了」和它。
    Failed(String),
    /// 有人要打开网页界面：第二个起来的托盘、macOS 上点了程序坞或者再开一次；带着会话的（Windows 上点了通知，`protocol`）打开那个会话。
    Open(Option<String>),
    /// 点了系统通知：打开那个会话；账号级的提醒（没有会话的）打开网页界面。
    Clicked(Option<String>),
    /// 「退出 Miyu」停完了，现在退。
    Exit,
}

/// 从别的线程把事交给界面线程。
#[derive(Clone)]
pub struct Post(EventLoopProxy<Event>);

impl Post {
    pub fn new(proxy: EventLoopProxy<Event>) -> Self {
        Self(proxy)
    }

    pub fn send(&self, event: Event) {
        if let Err(error) = self.0.send_event(event) {
            log::line(&format!("the event loop is gone: {error}"));
        }
    }
}

/// 界面线程手上的。
pub struct App {
    ctx: Ctx,
    text: Arc<Text>,
    status: Status,
    /// 照核心来的几行（软件包的入口、局域网）：变了把菜单整个换一份。
    extras: Extras,
    tray: Option<Tray>,
    /// 弹系统通知的（施工 AP-4）。
    toaster: Toaster,
    /// 起来时开什么：开机拉起来的不开（`None`），点了通知拉起来的开那个会话（`protocol`），别的开首页。
    open_at_start: Option<Page>,
    /// 在退了：别的事不再接。
    quitting: bool,
    /// 核心用的网页包的版本（「关于」照它写）：连上过核心才有，断了留着上一次的。
    web: Option<String>,
    asked: Once,
}

impl App {
    /// `problem` 是起来时就知道的错（找不到数据根、找不到主程序），状态行照它写「出错了」。
    pub fn new(
        ctx: Ctx,
        toaster: Toaster,
        open_at_start: Option<Page>,
        problem: Option<String>,
    ) -> Self {
        let mut status = Status::default();
        if let Some(problem) = problem {
            status.fail(problem);
        }
        Self {
            text: Arc::clone(&ctx.text),
            ctx,
            status,
            extras: Extras::default(),
            tray: None,
            toaster,
            open_at_start,
            quitting: false,
            web: None,
            asked: Once::default(),
        }
    }

    /// 事件循环转起来了：建托盘（macOS 要在这以后建），要开浏览器的开一次。
    pub fn started(&mut self) {
        match tray::create(&self.text, &self.look(), autostart::enabled(), &self.extras) {
            Ok(tray) => self.tray = Some(tray),
            Err(error) => log::line(&format!("no tray icon: {error}")),
        }
        if let Some(page) = self.open_at_start.take() {
            actions::open_web(&self.ctx, page);
        }
    }

    pub fn handle(&mut self, event: Event, control_flow: &mut ControlFlow) {
        match event {
            Event::Exit => *control_flow = ControlFlow::Exit,
            _ if self.quitting => {}
            Event::Menu(item) => self.menu(item.as_ref()),
            Event::Tray(event) => self.clicked(&event),
            Event::Link(report) => self.linked(report),
            Event::Begin(busy) => {
                self.status.begin(busy);
                self.redraw();
            }
            Event::Done(busy, ok) => {
                self.status.done(busy, ok);
                self.redraw();
            }
            Event::Failed(reason) => {
                self.status.fail(reason);
                self.redraw();
            }
            Event::Open(session) => {
                if self.asked.pass(Instant::now(), session.as_deref()) {
                    actions::open_web(&self.ctx, session.map_or(Page::Home, Page::Session));
                }
            }
            Event::Clicked(session) => {
                let page = session.map_or(Page::Home, Page::Session);
                actions::open_web(&self.ctx, page);
            }
        }
    }

    fn linked(&mut self, report: Report) {
        match report {
            Report::Up => {
                self.status.link(Linked::Up);
                start_log(&self.ctx);
            }
            Report::Slow => self.status.link(Linked::Slow),
            Report::Down => self.status.link(Linked::Down),
            Report::Refused(why) => {
                self.status.link(Linked::Down);
                self.status.fail(why);
            }
            Report::Packages(packages) => {
                if packages != self.extras.packages {
                    self.extras.packages = packages;
                    self.rebuild();
                }
                return;
            }
            Report::Lan(lan) => {
                if lan != self.extras.lan {
                    self.extras.lan = lan;
                    self.rebuild();
                }
                return;
            }
            Report::Note(note) => {
                self.toaster.show(note);
                return;
            }
            Report::Web(web) => {
                self.web = web;
                return;
            }
        }
        self.redraw();
    }

    fn menu(&mut self, item: &str) {
        match item {
            id::TOGGLE if !self.status.is_busy() => {
                if self.status.is_up() {
                    self.status.begin(Busy::Stopping);
                    actions::stop(&self.ctx);
                } else {
                    self.status.begin(Busy::Starting);
                    actions::start(&self.ctx);
                }
                self.redraw();
            }
            id::RESTART if !self.status.is_busy() => {
                self.status.begin(Busy::Stopping);
                actions::restart(&self.ctx);
                self.redraw();
            }
            id::TOGGLE | id::RESTART => log::line("busy, start/stop ignored"),
            id::OPEN => actions::open_web(&self.ctx, Page::Home),
            id::SETTINGS => actions::open_web(&self.ctx, Page::Settings),
            id::LAN => {
                let on = self.extras.lan == Some(true);
                if let Some(tray) = &self.tray {
                    tray.set_lan(on);
                }
                if self.status.state() == status::State::Running {
                    actions::toggle_lan(&self.ctx, on);
                } else {
                    log::line("not running, LAN access left alone");
                }
            }
            id::PHONE => actions::open_web(&self.ctx, Page::Access),
            id::AUTOSTART => {
                let on = actions::toggle_autostart(&self.ctx);
                if let Some(tray) = &self.tray {
                    tray.set_autostart(on);
                }
            }
            id::LOGS => actions::open_logs(&self.ctx),
            id::ABOUT => actions::about(&self.ctx, self.web.clone()),
            id::QUIT => {
                log::line("quitting with the core");
                self.quitting = true;
                // 先拿掉图标：停核心最多要等一分钟，图标留着像没点上。
                self.tray = None;
                actions::quit(&self.ctx);
            }
            other => match other.strip_prefix(id::PACKAGE) {
                Some(package) => actions::open_web(&self.ctx, Page::Package(package.to_owned())),
                None => log::line(&format!("unknown menu item {other}")),
            },
        }
    }

    /// 点了托盘图标：Windows、Linux 左键「打开网页界面」（右键托盘库、Linux 的托盘宿主自己出菜单）。macOS 点了出菜单。
    fn clicked(&mut self, event: &TrayIconEvent) {
        if cfg!(target_os = "macos") {
            return;
        }
        if let TrayIconEvent::Click {
            button: tray_icon::MouseButton::Left,
            button_state: tray_icon::MouseButtonState::Up,
            ..
        } = event
        {
            actions::open_web(&self.ctx, Page::Home);
        }
    }

    fn look(&self) -> status::Look {
        status::look(&self.status.state(), self.extras.lan == Some(true))
    }

    fn redraw(&self) {
        if let Some(tray) = &self.tray {
            tray.show(&self.text, &self.look());
        }
    }

    /// 照核心来的几行变了：菜单整个换一份。
    fn rebuild(&mut self) {
        let look = self.look();
        if let Some(tray) = &mut self.tray {
            tray.rebuild(&self.text, &look, &self.extras);
        }
    }
}

/// 连上核心了：数据根的 `var/log/` 这时一定有了（核心起来时建），日志还没定下的定在那里。
fn start_log(ctx: &Ctx) {
    if log::is_started() {
        return;
    }
    if let Some(logs) = ctx.root.as_ref().map(miyu_client::places::DataRoot::logs)
        && logs.is_dir()
    {
        log::start(logs.join("miyu-applet.log"));
        log::line(&format!(
            "logging here from now on, {}",
            env!("CARGO_PKG_VERSION")
        ));
    }
}
