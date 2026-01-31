pub mod vpn;
pub mod ui;
pub mod view;

pub use vpn::VpnSettings;
pub use ui::UiSettings;
pub use view::Settings;

use dioxus::prelude::*;
use crate::icons::{CircleCheck};
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
                        label: "Enable DAITA".to_string(),
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
    let regions = state.regions.read();
    use_scroll_handler(None);

    rsx! {
        div { class: "h-full w-full overflow-y-auto custom-scrollbar",
            div { class: "pb-24 -mx-4 divide-y divide-border/30",
                div { class: "p-4",
                    p { class: "text-sm text-muted-foreground mb-6",
                        "Route your traffic through two or more VPN servers for an extra layer of privacy and anonymity. This hides your entry point from the exit point and vice-versa."
                    }
                    SettingRow {
                        id: "multi-hop",
                        label: "Enable Multihop".to_string(),
                        checked: s.multi_hop,
                        onclick: move |_| {
                            state.settings.with_mut(|s| s.multi_hop = !s.multi_hop);
                        },
                    }
                }

                if s.multi_hop {
                    div { class: "flex flex-col bg-accent/5",
                        div { class: "p-4 pb-2",
                            h4 { class: "text-[10px] font-bold text-muted-foreground uppercase tracking-widest", "Entry Location" }
                        }
                        div { class: "px-1 space-y-1 pb-4",
                            for region in regions.iter().cloned() {
                                for city in region.cities.iter().cloned() {
                                    {
                                        let loc = format!("{}, {}", region.name, city.name);
                                        let loc2 = loc.clone();
                                        let is_active = s.entry_location == loc;
                                        rsx! {
                                            div {
                                                key: "{region.name}-{city.name}-entry",
                                                class: "px-3 py-2 rounded-xl flex items-center justify-between cursor-pointer transition-colors",
                                                class: if is_active { "bg-primary/10 text-primary" } else { "hover:bg-accent/40 text-foreground" },
                                                onclick: move |_| {
                                                    state.settings.with_mut(|s| s.entry_location = loc2.clone());
                                                },
                                                div { class: "flex items-center gap-3",
                                                    span { class: "text-lg", "{region.flag}" }
                                                    span { class: "text-xs font-bold", "{city.name}" }
                                                }
                                                if is_active {
                                                    CircleCheck { size: 14 }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        div { class: "p-4 pb-2 border-t border-border/30",
                            h4 { class: "text-[10px] font-bold text-muted-foreground uppercase tracking-widest", "Exit Location" }
                        }
                        div { class: "px-1 space-y-1 pb-4",
                            for region in regions.iter().cloned() {
                                for city in region.cities.iter().cloned() {
                                    {
                                        let loc = format!("{}, {}", region.name, city.name);
                                        let loc2 = loc.clone();
                                        let is_active = s.exit_location == loc;
                                        rsx! {
                                            div {
                                                key: "{region.name}-{city.name}-exit",
                                                class: "px-3 py-2 rounded-xl flex items-center justify-between cursor-pointer transition-colors",
                                                class: if is_active { "bg-status-success/10 text-status-success" } else { "hover:bg-accent/40 text-foreground" },
                                                onclick: move |_| {
                                                    state.settings.with_mut(|s| s.exit_location = loc2.clone());
                                                },
                                                div { class: "flex items-center gap-3",
                                                    span { class: "text-lg", "{region.flag}" }
                                                    span { class: "text-xs font-bold", "{city.name}" }
                                                }
                                                if is_active {
                                                    CircleCheck { size: 14 }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
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
                        class: "px-4 flex items-center justify-between hover:bg-accent/20 transition-colors border-b border-border/50 last:border-0 shrink-0",
                        style: "height: 48px !important; min-height: 48px !important;",
                        span { class: "text-sm font-medium", "{app}" }
                        div { class: "w-4 h-4 rounded border border-border group-hover:border-primary transition-colors" }
                    }
                }
            }
        }
    }
}

#[component]
pub fn AntiCensorshipSettings() -> Element {
    let mut state = use_context::<ConnectionState>();
    let s = state.settings.read();
    
    rsx! {
        div { class: "h-full w-full overflow-y-auto custom-scrollbar",
            div { class: "pb-24 -mx-4 divide-y divide-border/30",
                div { class: "p-4",
                    p { class: "text-sm text-muted-foreground mb-4",
                        "Configure settings to bypass network censorship and improve connectivity in restricted environments."
                    }
                }
                SettingRow {
                    label: "Obfuscation".to_string(),
                    checked: s.obfuscation,
                    onclick: move |_| {
                        state.settings.with_mut(|s| s.obfuscation = !s.obfuscation);
                    },
                }
            }
        }
    }
}

#[component]
pub fn ServerOverrideSettings() -> Element {
    rsx! {
        div { class: "h-full w-full overflow-y-auto custom-scrollbar p-4",
            div { class: "bg-accent/10 rounded-xl p-4 border border-border/50",
                p { class: "text-xs text-muted-foreground",
                    "No server IP overrides configured. This advanced feature allows you to manually specify the IP address for a VPN server."
                }
            }
        }
    }
}