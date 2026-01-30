#![allow(non_snake_case)]

use dioxus::prelude::*;
use dioxus::desktop::{Config, WindowBuilder, LogicalSize};
use dioxus::desktop::tao::dpi::PhysicalPosition;
use dioxus::desktop::tao::platform::windows::WindowBuilderExtWindows;

mod components;
mod layouts;
mod views;
mod models;
mod state;
mod data;
mod hooks;
mod storage;
mod icons;
mod window;

use layouts::MainLayout;
use views::{
    dashboard::Dashboard,
    locations::Locations,
    settings::{
        Settings, VpnSettingsPage, UiSettingsPage, DaitaSettings, 
        MultihopSettings, SplitTunnelingSettings
    },
    account::Account,
    devices::Devices,
    login::Login,
    support::Support,
    app_info::AppInfo,
};
use state::AppStateProvider;
use state::ConnectionState;
use components::toast::ToastProvider;
use window::{use_tray_management, create_tray_icon, WINDOW_WIDTH, WINDOW_HEIGHT};

#[derive(Clone, Routable, Debug, PartialEq)]
#[rustfmt::skip]
pub enum Route {
    #[layout(MainLayout)]
        #[route("/")]
        Dashboard {},
        #[route("/locations")]
        Locations {},
        #[route("/settings")]
        Settings {},
        #[route("/settings/vpn")]
        VpnSettingsPage {},
        #[route("/settings/ui")]
        UiSettingsPage {},
        #[route("/settings/daita")]
        DaitaSettings {},
        #[route("/settings/multihop")]
        MultihopSettings {},
        #[route("/settings/split-tunneling")]
        SplitTunnelingSettings {},
        #[route("/account")]
        Account {},
        #[route("/devices")]
        Devices {},
        #[route("/support")]
        Support {},
        #[route("/app-info")]
        AppInfo {},
    #[end_layout]
    #[route("/login")]
    Login {},
}

fn App() -> Element {
    use_tray_management();

    rsx! {
        document::Stylesheet { href: asset!("/assets/tailwind.css") }
        AppStateProvider {
            ToastProvider { 
                AppContent {}
            }
        }
    }
}

fn AppContent() -> Element {
    let state = use_context::<ConnectionState>();
    let dark_mode = (state.settings)().dark_mode;

    rsx! {
        div {
            class: if dark_mode { "dark" },
            div {
                class: "bg-background text-foreground transition-colors duration-300",
                style: "height: {WINDOW_HEIGHT}px; width: {WINDOW_WIDTH}px; position: relative; display: flex; flex-direction: column; overflow: hidden;",
                Router::<Route> {}
            }
        }
    }
}

fn main() {
    let _tray = create_tray_icon();

    let config = Config::new()
        .with_window(
            WindowBuilder::new()
                .with_title("MarinVPN")
                .with_inner_size(LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT))
                .with_decorations(false)
                .with_transparent(true)
                .with_resizable(false)
                .with_skip_taskbar(true)
                .with_visible(false)
                .with_position(PhysicalPosition::new(100, 100))
        )
        .with_menu(None)
        .with_resource_directory(".");

    LaunchBuilder::new()
        .with_cfg(config)
        .launch(App);
}