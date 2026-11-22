use crate::model::{self, Config, Platform, Settings};
use crate::protection::{self, Protection};
use serde_json::{json, Value};
use std::rc::Rc;
use std::{collections::HashMap, path::PathBuf};
use tao::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy, EventLoopWindowTarget},
    window::{Fullscreen, Window, WindowBuilder, WindowId},
};
use wry::{
    dpi::{LogicalPosition as WPosition, LogicalSize as WSize},
    PageLoadEvent, Rect, WebContext, WebView, WebViewBuilder,
};

#[derive(Debug, Clone)]
enum Message {
    Command(String),
    Ready,
    Qr(Result<String, String>),
    Location(String, String),
    Popup(String),
}
struct Tab {
    view: WebView,
    id: String,
    url: String,
}
struct Floating {
    _view: WebView,
    _window: Window,
}
struct Desktop {
    shell: WebView,
    tabs: Vec<Tab>,
    floating: HashMap<WindowId, Floating>,
    context: WebContext,
    protection: Rc<Protection>,
    window: Window,
    config: Config,
    file: PathBuf,
    route: String,
    active: Option<String>,
    bounds: Rect,
    proxy: EventLoopProxy<Message>,
    compact: bool,
}
fn rect(x: f64, y: f64, w: f64, h: f64) -> Rect {
    Rect {
        position: WPosition::new(x, y).into(),
        size: WSize::new(w.max(1.0), h.max(1.0)).into(),
    }
}
impl Desktop {
    fn new(
        target: &EventLoopWindowTarget<Message>,
        proxy: EventLoopProxy<Message>,
    ) -> Result<Self, String> {
        let dir = directories::ProjectDirs::from("", "SocialWorkspace", "SocialWorkspaceRust")
            .ok_or("Cannot find the application data folder.")?
            .data_local_dir()
            .to_path_buf();
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let file = dir.join("workspace_apps.json");
        let config = Config::load(&file)?;
        if !file.exists() {
            config.save(&file)?;
        }
        let window = WindowBuilder::new()
            .with_title("Social Workspace")
            .with_decorations(false)
            .with_inner_size(LogicalSize::new(1440., 960.))
            .with_min_inner_size(LogicalSize::new(980., 680.))
            .build(target)
            .map_err(|e| e.to_string())?;
        let p = proxy.clone();
        let ready = proxy.clone();
        let shell = WebViewBuilder::new()
            .with_html(include_str!("../ui/index.html"))
            .with_background_color((16, 19, 27, 255))
            .with_bounds(rect(0., 0., 1440., 960.))
            .with_navigation_handler(|url| url == "about:blank")
            .with_ipc_handler(move |request| {
                let _ = p.send_event(Message::Command(request.body().clone()));
            })
            .with_on_page_load_handler(move |event, _| {
                if matches!(event, PageLoadEvent::Finished) {
                    let _ = ready.send_event(Message::Ready);
                }
            })
            .build_as_child(&window)
            .map_err(|e| e.to_string())?;
        Ok(Self {
            shell,
            tabs: Vec::new(),
            floating: HashMap::new(),
            context: WebContext::new(Some(dir.join("browser"))),
            protection: Rc::new(Protection::bundled()?),
            window,
            config,
            file,
            route: "home".into(),
            active: None,
            bounds: rect(220., 160., 1220., 772.),
            proxy,
            compact: false,
        })
    }
    fn execute(&self, script: &str) {
        let _ = self.shell.evaluate_script(script);
    }
    fn sync(&self) {
        let url = self
            .active
            .as_ref()
            .and_then(|id| self.tabs.iter().find(|t| &t.id == id))
            .map(|t| t.url.as_str());
        let value = json!({"apps":self.config.apps,"settings":self.config.settings,"tabs":self.tabs.iter().map(|t|&t.id).collect::<Vec<_>>(),"active":self.active,"route":self.route,"url":url,"fullscreen":self.window.fullscreen().is_some(),"compact":self.compact});
        self.execute(&format!("window.receive&&window.receive({value});"));
        for tab in &self.tabs {
            let show = self.route == "browser" && self.active.as_ref() == Some(&tab.id);
            let _ = tab.view.set_visible(show);
            if show {
                let _ = tab.view.set_bounds(self.bounds);
            }
        }
    }
    fn feedback(&self, kind: &str, text: &str, error: bool) {
        self.execute(&format!(
            "window.feedback({},{},{});",
            json!(kind),
            json!(text),
            error
        ));
    }
    fn persist(&mut self, next: Config) -> Result<(), String> {
        next.save(&self.file)?;
        self.config = next;
        Ok(())
    }
    fn open(&mut self, id: &str) -> Result<(), String> {
        if !self.tabs.iter().any(|t| t.id == id) {
            let app = self
                .config
                .apps
                .iter()
                .find(|a| a.id == id)
                .ok_or("Platform not found.")?
                .clone();
            let p = self.proxy.clone();
            let tab_id = id.to_owned();
            let pop = self.proxy.clone();
            // Remote content has no workspace IPC handler and cannot read configuration.
            let view = WebViewBuilder::new_with_web_context(&mut self.context)
                .with_url("about:blank")
                .with_background_color((16, 19, 27, 255))
                .with_bounds(self.bounds)
                .with_on_page_load_handler(move |_, url| {
                    if url.starts_with("http://") || url.starts_with("https://") {
                        let _ = p.send_event(Message::Location(tab_id.clone(), url));
                    }
                })
                .with_new_window_req_handler(move |url, _| {
                    let _ = pop.send_event(Message::Popup(url));
                    wry::NewWindowResponse::Deny
                })
                .build_as_child(&self.window)
                .map_err(|e| e.to_string())?;
            protection::attach(&view, self.protection.clone())?;
            view.load_url(&app.url).map_err(|e| e.to_string())?;
            self.tabs.push(Tab {
                view,
                id: id.into(),
                url: app.url,
            });
        }
        self.active = Some(id.into());
        self.route = "browser".into();
        self.sync();
        if let Some(tab) = self.tabs.iter().find(|t| t.id == id) {
            let _ = tab.view.focus();
        }
        Ok(())
    }
    fn floating(
        &mut self,
        url: &str,
        target: &EventLoopWindowTarget<Message>,
    ) -> Result<(), String> {
        let url = model::website(url)?;
        let window = WindowBuilder::new()
            .with_title("Social Workspace · App window")
            .with_inner_size(LogicalSize::new(1100., 800.))
            .build(target)
            .map_err(|e| e.to_string())?;
        let view = WebViewBuilder::new_with_web_context(&mut self.context)
            .with_url("about:blank")
            .build(&window)
            .map_err(|e| e.to_string())?;
        protection::attach(&view, self.protection.clone())?;
        view.load_url(&url).map_err(|e| e.to_string())?;
        self.floating.insert(
            window.id(),
            Floating {
                _view: view,
                _window: window,
            },
        );
        Ok(())
    }
    fn active_view(&self) -> Option<&WebView> {
        self.active
            .as_ref()
            .and_then(|id| self.tabs.iter().find(|t| &t.id == id))
            .map(|t| &t.view)
    }
    fn command(
        &mut self,
        text: &str,
        target: &EventLoopWindowTarget<Message>,
    ) -> Result<bool, String> {
        let value: Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
        let cmd = value["cmd"].as_str().unwrap_or("");
        let id = value["id"].as_str().unwrap_or("");
        match cmd {
            "ready" => self.sync(),
            "route" => {
                let r = value["route"].as_str().unwrap_or("home");
                if ["home", "add", "settings"].contains(&r) {
                    self.route = r.into();
                    self.sync();
                }
            }
            "open" => self.open(id)?,
            "close_tab" => {
                self.tabs.retain(|t| t.id != id);
                if self.active.as_deref() == Some(id) {
                    self.active = None;
                    self.route = "home".into();
                }
                self.sync();
            }
            "bounds" => {
                let x = value["x"].as_f64().unwrap_or(220.);
                let y = value["y"].as_f64().unwrap_or(160.);
                let w = value["width"].as_f64().unwrap_or(1000.);
                let h = value["height"].as_f64().unwrap_or(700.);
                if [x, y, w, h]
                    .iter()
                    .all(|v| v.is_finite() && *v >= 0. && *v < 20000.)
                {
                    self.bounds = rect(x, y, w, h);
                    if let Some(v) = self.active_view() {
                        let _ = v.set_bounds(self.bounds);
                    }
                }
            }
            "minimize" => self.window.set_minimized(true),
            "maximize" => self.window.set_maximized(!self.window.is_maximized()),
            "drag" => {
                if self.window.is_maximized() {
                    self.window.set_maximized(false);
                }
                let _ = self.window.drag_window();
            }
            "close" => return Ok(true),
            "fullscreen" => {
                let full = self.window.fullscreen().is_some();
                self.window.set_fullscreen(if full {
                    None
                } else {
                    Some(Fullscreen::Borderless(None))
                });
                self.sync();
            }
            "back" => {
                if let Some(v) = self.active_view() {
                    let _ = v.evaluate_script("history.back()");
                }
            }
            "forward" => {
                if let Some(v) = self.active_view() {
                    let _ = v.evaluate_script("history.forward()");
                }
            }
            "reload" => {
                if let Some(v) = self.active_view() {
                    let _ = v.evaluate_script("location.reload()");
                }
            }
            "navigate" => {
                let url = model::website(value["url"].as_str().unwrap_or(""))?;
                if let Some(v) = self.active_view() {
                    v.load_url(&url).map_err(|e| e.to_string())?;
                }
            }
            "floating" => {
                let url = self
                    .config
                    .apps
                    .iter()
                    .find(|a| a.id == id)
                    .ok_or("Platform not found.")?
                    .url
                    .clone();
                self.floating(&url, target)?;
            }
            "popout" => {
                if let Some(url) = self
                    .active
                    .as_ref()
                    .and_then(|id| self.tabs.iter().find(|t| &t.id == id))
                    .map(|t| t.url.clone())
                {
                    self.floating(&url, target)?;
                }
            }
            "cycle" => {
                if !self.tabs.is_empty() {
                    let index = self
                        .active
                        .as_ref()
                        .and_then(|id| self.tabs.iter().position(|t| &t.id == id))
                        .unwrap_or(0);
                    let len = self.tabs.len();
                    let next = if value["back"].as_bool().unwrap_or(false) {
                        (index + len - 1) % len
                    } else {
                        (index + 1) % len
                    };
                    let id = self.tabs[next].id.clone();
                    self.open(&id)?;
                }
            }
            "settings" => {
                let settings: Settings =
                    serde_json::from_value(value["settings"].clone()).map_err(|e| e.to_string())?;
                settings.validate()?;
                let mut next = self.config.clone();
                next.settings = settings;
                self.persist(next)?;
                self.sync();
                self.feedback("settings", "Your workspace has been updated.", false);
            }
            "add" => {
                let icon = value["icon"].as_str().map(str::to_owned);
                let app = Platform::new(
                    value["name"].as_str().unwrap_or(""),
                    value["url"].as_str().unwrap_or(""),
                    icon,
                )?;
                if self.config.apps.iter().any(|a| a.url == app.url) {
                    return Err("That website is already in your workspace.".into());
                }
                let mut next = self.config.clone();
                next.apps.push(app);
                self.persist(next)?;
                self.route = "home".into();
                self.sync();
                self.execute("window.platformAdded();");
            }
            "remove" => {
                let mut next = self.config.clone();
                next.apps.retain(|a| a.id != id);
                self.persist(next)?;
                self.tabs.retain(|t| t.id != id);
                if self.active.as_deref() == Some(id) {
                    self.active = None;
                    self.route = "home".into();
                }
                self.sync();
            }
            "pick_picture" => {
                let selected = rfd::FileDialog::new()
                    .set_title("Choose an app picture")
                    .add_filter("Pictures", &["png", "jpg", "jpeg", "webp", "bmp", "ico"])
                    .pick_file();
                if let Some(path) = selected {
                    let picture = model::picture(&path)?;
                    self.execute(&format!(
                        "window.pictureChosen({},{});",
                        json!(picture),
                        if id.is_empty() {
                            Value::Null
                        } else {
                            json!(id)
                        }
                    ));
                }
            }
            "set_icon" => {
                let icon = value["icon"].as_str().unwrap_or("");
                if !icon.starts_with("data:image/png;base64,") || icon.len() > 256000 {
                    return Err("Choose another app picture.".into());
                }
                let mut next = self.config.clone();
                if let Some(a) = next.apps.iter_mut().find(|a| a.id == id) {
                    a.icon = icon.into();
                }
                self.persist(next)?;
                self.sync();
            }
            "qr" => {
                if let Some(path) = rfd::FileDialog::new()
                    .set_title("Choose a website QR image")
                    .add_filter("QR pictures", &["png", "jpg", "jpeg", "webp", "bmp"])
                    .pick_file()
                {
                    let proxy = self.proxy.clone();
                    std::thread::spawn(move || {
                        let _ = proxy.send_event(Message::Qr(model::qr(&path)));
                    });
                }
            }
            _ => {}
        }
        Ok(false)
    }
}
pub fn run() -> Result<(), String> {
    let events = EventLoopBuilder::<Message>::with_user_event().build();
    let proxy = events.create_proxy();
    let mut app = Desktop::new(&events, proxy)?;
    events.run(move |event, target, flow| {
        *flow = ControlFlow::Wait;
        match event {
            Event::UserEvent(Message::Ready) => app.sync(),
            Event::UserEvent(Message::Command(text)) => match app.command(&text, target) {
                Ok(true) => *flow = ControlFlow::Exit,
                Err(error) => {
                    let kind = if app.route == "settings" {
                        "settings"
                    } else {
                        "add"
                    };
                    app.feedback(kind, &error, true);
                }
                _ => {}
            },
            Event::UserEvent(Message::Qr(result)) => match result {
                Ok(url) => app.execute(&format!("window.qrChosen({});", json!(url))),
                Err(e) => app.feedback("add", &e, true),
            },
            Event::UserEvent(Message::Location(id, url)) => {
                if let Some(tab) = app.tabs.iter_mut().find(|t| t.id == id) {
                    tab.url = url;
                }
                app.sync();
            }
            Event::UserEvent(Message::Popup(url)) => {
                let _ = app.floating(&url, target);
            }
            Event::WindowEvent {
                window_id,
                event: WindowEvent::CloseRequested,
                ..
            } => {
                if window_id == app.window.id() {
                    *flow = ControlFlow::Exit;
                } else {
                    app.floating.remove(&window_id);
                }
            }
            Event::WindowEvent {
                window_id,
                event: WindowEvent::Resized(size),
                ..
            } if window_id == app.window.id() => {
                let logical = size.to_logical::<f64>(app.window.scale_factor());
                let _ = app
                    .shell
                    .set_bounds(rect(0., 0., logical.width, logical.height));
            }
            _ => {}
        }
    });
}
