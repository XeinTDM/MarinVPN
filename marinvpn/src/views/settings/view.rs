use crate::components::MenuRow;
use crate::icons::*;
use crate::state::ConnectionState;
use crate::Route;
use dioxus::desktop::use_window;
use dioxus::prelude::*;

#[component]
pub fn Settings() -> Element {
    let state = use_context::<ConnectionState>();
    let nav = use_navigator();
    let window = use_window();

    use_effect(move || {
        if let Some(target) = (state.scroll_to)() {
            match target.as_str() {
                "protocol" | "quantum-resistant" | "kill-switch" | "dns-blocking" | "ipv6"
                | "auto-connect" | "local-sharing" | "launch-startup" => {
                    nav.push(Route::VpnSettingsPage {});
                }
                "multi-hop" => {
                    nav.push(Route::MultihopSettings {});
                }
                "split-tunneling" => {
                    nav.push(Route::SplitTunnelingSettings {});
                }
                "daita" => {
                    nav.push(Route::DaitaSettings {});
                }
                "obfuscation" => {
                    nav.push(Route::VpnSettingsPage {});
                }
                _ => {}
            }
        }
    });

    rsx! {
        div { class: "h-full w-full flex flex-col bg-background",
            div { class: "flex-1 overflow-y-auto custom-scrollbar",
                div { class: "pb-24 divide-y divide-border/30 -mx-4",
                    MenuRow {
                        label: "DAITA".to_string(),
                        icon: rsx! {
                            ShieldCheck { size: 18 }
                        },
                        onclick: move |_| {
                            nav.push(Route::DaitaSettings {});
                        },
                    }
                    MenuRow {
                        label: "Multihop".to_string(),
                        icon: rsx! {
                            RefreshCw { size: 18 }
                        },
                        onclick: move |_| {
                            nav.push(Route::MultihopSettings {});
                        },
                    }
                    MenuRow {
                        label: "VPN settings".to_string(),
                        icon: rsx! {
                            Shield { size: 18 }
                        },
                        onclick: move |_| {
                            nav.push(Route::VpnSettingsPage {});
                        },
                    }
                    MenuRow {
                        label: "User interface settings".to_string(),
                        icon: rsx! {
                            crate::icons::Settings { size: 18 }
                        },
                        onclick: move |_| {
                            nav.push(Route::UiSettingsPage {});
                        },
                    }
                    MenuRow {
                        label: "Split tunneling".to_string(),
                        icon: rsx! {
                            FlaskConical { size: 18 }
                        },
                        onclick: move |_| {
                            nav.push(Route::SplitTunnelingSettings {});
                        },
                    }
                    MenuRow {
                        label: "Support".to_string(),
                        icon: rsx! {
                            LifeBuoy { size: 18 }
                        },
                        onclick: move |_| {
                            nav.push(Route::Support {});
                        },
                    }
                    MenuRow {
                        label: "App info".to_string(),
                        icon: rsx! {
                            Info { size: 18 }
                        },
                        onclick: move |_| {
                            nav.push(Route::AppInfo {});
                        },
                    }

                    div { class: "p-4",
                        button {
                            class: "w-full flex items-center justify-center bg-destructive/10 hover:bg-destructive/20 text-destructive rounded-xl border border-destructive/20 transition-all font-bold shadow-sm active:scale-95 text-xs shrink-0",
                            style: "height: 48px !important; min-height: 48px !important;",
                            onclick: move |_| {
                                window.close();
                            },
                            "Disconnect & Exit"
                        }
                    }
                }
            }
        }
    }
}
