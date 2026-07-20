//! KDE / Freedesktop system tray via StatusNotifierItem (Wayland and X11).

use std::sync::Arc;

use ksni::blocking::TrayMethods;
use ksni::{Category, MenuItem, Status, Tray};

const APP_ID: &str = "com.quickconvert.QuickConvert";

pub struct TrayCallbacks {
    pub show: Arc<dyn Fn() + Send + Sync>,
    pub quit: Arc<dyn Fn() + Send + Sync>,
}

pub struct QuickConvertTray {
    callbacks: TrayCallbacks,
}

impl QuickConvertTray {
    pub fn spawn(callbacks: TrayCallbacks) -> ksni::blocking::Handle<Self> {
        let tray = Self { callbacks };
        tray.spawn().expect("failed to start system tray")
    }
}

impl Tray for QuickConvertTray {
    fn id(&self) -> String {
        APP_ID.into()
    }

    fn icon_name(&self) -> String {
        APP_ID.into()
    }

    fn title(&self) -> String {
        "QuickConvert".into()
    }

    fn category(&self) -> Category {
        Category::ApplicationStatus
    }

    fn status(&self) -> Status {
        Status::Active
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        (self.callbacks.show)();
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        use ksni::menu::*;

        let show = self.callbacks.show.clone();
        let quit = self.callbacks.quit.clone();

        vec![
            StandardItem {
                label: "Show QuickConvert".into(),
                icon_name: APP_ID.into(),
                activate: Box::new(move |_| (show)()),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "Quit".into(),
                icon_name: "application-exit".into(),
                activate: Box::new(move |_| (quit)()),
                ..Default::default()
            }
            .into(),
        ]
    }
}
