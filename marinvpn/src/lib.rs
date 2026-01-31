#![allow(non_snake_case)]

pub mod components;
pub mod data;
pub mod hooks;
pub mod i18n;
pub mod icons;
pub mod layouts;
pub mod models;
pub mod services;
pub mod state;
pub mod storage;
pub mod views;
pub mod window;

use dioxus::desktop::tao::dpi::PhysicalPosition;
use dioxus::desktop::tao::platform::windows::WindowBuilderExtWindows;
use dioxus::desktop::{Config, LogicalSize, WindowBuilder};
use dioxus::prelude::*;

use components::toast::ToastProvider;
use layouts::MainLayout;
use models::ConnectionStatus;
use state::{AppStateProvider, ConnectionState};
use views::{
    account::Account,
    app_info::AppInfo,
    dashboard::Dashboard,
    devices::Devices,
    locations::Locations,
    login::Login,
    settings::{
        AntiCensorshipSettings, DaitaSettings, MultihopSettings, ServerOverrideSettings, Settings,
        SplitTunnelingSettings, UiSettingsPage, VpnSettingsPage,
    },
    support::Support,
};
use window::{update_tray_tooltip, use_tray_management, WINDOW_HEIGHT, WINDOW_WIDTH};

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
        #[route("/settings/anti-censorship")]
        AntiCensorshipSettings {},
        #[route("/settings/server-override")]
        ServerOverrideSettings {},
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

impl Route {
    pub fn title(&self) -> Option<&'static str> {
        match self {
            Route::Account {} => Some("account"),
            Route::Devices {} => Some("devices"),
            Route::Settings {} => Some("settings"),
            Route::VpnSettingsPage {} => Some("vpn_settings"),
            Route::UiSettingsPage {} => Some("ui_settings"),
            Route::DaitaSettings {} => Some("daita"),
            Route::MultihopSettings {} => Some("multihop"),
            Route::SplitTunnelingSettings {} => Some("split_tunneling"),
            Route::AntiCensorshipSettings {} => Some("anti_censorship"),
            Route::ServerOverrideSettings {} => Some("server_override"),
            Route::Support {} => Some("support"),
            Route::AppInfo {} => Some("app_info"),
            Route::Locations {} => Some("locations"),
            _ => None,
        }
    }
}

pub fn App() -> Element {
    use_tray_management();

    rsx! {
        document::Stylesheet { href: asset!("/assets/tailwind.css") }
        ToastProvider {
            AppStateProvider { AppContent {} }
        }
    }
}

fn AppContent() -> Element {
    let state = use_context::<ConnectionState>();
    let dark_mode = (state.settings)().dark_mode;
    let status = (state.status)();
    let location = (state.current_location)();

    use_effect(move || {
        if status == ConnectionStatus::Connected {
            let location_info = models::LocationInfo::from_string(&location);
            update_tray_tooltip(&format!(
                "Connected. {}, {}",
                location_info.city, location_info.country
            ));
        } else {
            update_tray_tooltip("MarinVPN");
        }
    });

    rsx! {
        div { class: if dark_mode { "dark" },
            div {
                class: "bg-background text-foreground transition-colors duration-300",
                style: "height: {WINDOW_HEIGHT}px; width: {WINDOW_WIDTH}px; position: relative; display: flex; flex-direction: column; overflow: hidden;",
                Router::<Route> {}
            }
        }
    }
}

pub fn run_app() {
    tracing_subscriber::fmt::init();

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
                .with_position(PhysicalPosition::new(100, 100)),
        )
        .with_menu(None)
        .with_resource_directory(".");

    LaunchBuilder::new().with_cfg(config).launch(App);
}
