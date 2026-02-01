#![allow(non_snake_case)]

pub mod components;
pub mod data;
pub mod error;
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
use dioxus::desktop::{use_window, Config, LogicalSize, WindowBuilder};
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
use window::{
    update_tray_icon_path, update_tray_tooltip, use_tray_management, WINDOW_HEIGHT, WINDOW_WIDTH,
};

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
    let branding_name = (state.settings)().branding_name.clone();
    let branding_name_title = branding_name.clone();
    let branding_color = (state.settings)().branding_accent_color.clone();
    let branding_logo = (state.settings)().branding_logo_path.clone();
    let window = use_window();

    use_effect(move || {
        if status == ConnectionStatus::Connected {
            let location_info = models::LocationInfo::from_string(&location);
            update_tray_tooltip(&format!(
                "Connected. {}, {}",
                location_info.city, location_info.country
            ));
        } else {
            update_tray_tooltip(&branding_name);
        }
    });

    use_effect(move || {
        window.window.set_title(&branding_name_title);
    });

    use_effect(move || {
        if branding_logo.trim().is_empty() {
            update_tray_icon_path(None);
        } else {
            update_tray_icon_path(Some(&branding_logo));
        }
    });

    let (primary, primary_fg) = branding_colors(&branding_color);
    let theme_style = format!(
        "--color-primary: {}; --color-primary-foreground: {};",
        primary, primary_fg
    );

    rsx! {
        div { class: if dark_mode { "dark" },
            div {
                class: "bg-background text-foreground transition-colors duration-300",
                style: "height: {WINDOW_HEIGHT}px; width: {WINDOW_WIDTH}px; position: relative; display: flex; flex-direction: column; overflow: hidden; {theme_style}",
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

fn branding_colors(hex: &str) -> (String, String) {
    let color = normalize_hex_color(hex).unwrap_or_else(|| "#6D28D9".to_string());
    let fg = if is_dark_color(&color) {
        "#FFFFFF".to_string()
    } else {
        "#111111".to_string()
    };
    (color, fg)
}

fn normalize_hex_color(input: &str) -> Option<String> {
    let trimmed = input.trim();
    let val = if trimmed.starts_with('#') {
        trimmed.to_string()
    } else {
        format!("#{}", trimmed)
    };
    if is_valid_hex_color(&val) {
        Some(val)
    } else {
        None
    }
}

fn is_valid_hex_color(color: &str) -> bool {
    let bytes = color.as_bytes();
    if bytes.len() != 7 {
        return false;
    }
    if bytes[0] != b'#' {
        return false;
    }
    bytes[1..].iter().all(|b| b.is_ascii_hexdigit())
}

fn is_dark_color(color: &str) -> bool {
    let r = u8::from_str_radix(&color[1..3], 16).unwrap_or(0) as f32 / 255.0;
    let g = u8::from_str_radix(&color[3..5], 16).unwrap_or(0) as f32 / 255.0;
    let b = u8::from_str_radix(&color[5..7], 16).unwrap_or(0) as f32 / 255.0;
    let luminance = 0.2126 * r + 0.7152 * g + 0.0722 * b;
    luminance < 0.5
}
