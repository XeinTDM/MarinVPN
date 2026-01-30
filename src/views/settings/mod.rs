pub mod vpn;
pub mod ui;
pub mod view;

pub use vpn::VpnSettings;
pub use ui::UiSettings;
pub use view::Settings;

use dioxus::prelude::*;
use crate::state::ConnectionState;
use crate::components::SettingRow;

use crate::hooks::use_scroll_handler;

#[component]
pub fn VpnSettingsPage() -> Element {
    let dns_expanded = use_signal(|| false);
    use_scroll_handler(Some(dns_expanded));

    rsx! {
        div { class: "h-full w-full overflow-y-auto custom-scrollbar",
            VpnSettings { dns_expanded }
        }
    }
}

#[component]
pub fn UiSettingsPage() -> Element {
    rsx! {
        div { class: "h-full w-full overflow-y-auto custom-scrollbar", UiSettings {} }
    }
}

#[component]
pub fn DaitaSettings() -> Element {
    let mut state = use_context::<ConnectionState>();
    let s = state.settings.read();
    use_scroll_handler(None);

    rsx! {
        div { class: "h-full w-full overflow-y-auto custom-scrollbar",
            div { class: "pb-24 -mx-4",
                p { class: "text-sm text-muted-foreground mb-6 px-4",
                    "DAITA adds padding to your traffic so that all packets have the same size. This makes it significantly harder for AI and traffic analysis tools to track or identify your online activities."
                }
                div { class: "divide-y divide-border/30",
                    SettingRow {
                        id: "daita",
                        label: "Enable DAITA",
                        checked: s.daita_enabled,
                        onclick: move |_| {
                            state.settings.with_mut(|s| s.daita_enabled = !s.daita_enabled);
                        },
                    }
                }
            }
        }
    }
}

#[component]
pub fn MultihopSettings() -> Element {
    let mut state = use_context::<ConnectionState>();
    let s = state.settings.read();
    use_scroll_handler(None);

    rsx! {
        div { class: "h-full w-full overflow-y-auto custom-scrollbar",
            div { class: "pb-24 -mx-4",
                p { class: "text-sm text-muted-foreground mb-6 px-4",
                    "Route your traffic through two or more VPN servers for an extra layer of privacy and anonymity. This hides your entry point from the exit point and vice-versa."
                }
                div { class: "divide-y divide-border/30",
                    SettingRow {
                        id: "multi-hop",
                        label: "Enable Multihop",
                        checked: s.multi_hop,
                        onclick: move |_| {
                            state.settings.with_mut(|s| s.multi_hop = !s.multi_hop);
                        },
                    }
                }
            }
        }
    }
}

#[component]
pub fn SplitTunnelingSettings() -> Element {
    rsx! {
        div { class: "h-full w-full overflow-y-auto custom-scrollbar",
            div { class: "pb-24 -mx-4 divide-y divide-border/30",
                div { class: "p-6 text-center text-muted-foreground text-xs", "No apps excluded" }
                for app in ["Chrome", "Discord", "Spotify", "Steam"] {
                    div {
                        class: "px-4 flex items-center justify-between hover:bg-accent/20 transition-colors border-b border-border/50 last:border-0",
                        style: "height: 48px;",
                        span { class: "text-sm font-medium", "{app}" }
                        div { class: "w-4 h-4 rounded border border-border group-hover:border-primary transition-colors" }
                    }
                }
            }
        }
    }
}