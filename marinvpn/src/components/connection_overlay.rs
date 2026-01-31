use crate::hooks::use_vpn_client;
use crate::icons::{Loader, RefreshCw};
use crate::models::{ConnectionStatus, SettingsState};
use crate::state::ConnectionState;
use crate::window::WINDOW_WIDTH;
use crate::Route;
use dioxus::prelude::*;

struct ConnectionDetails {
    country: String,
    city: String,
    server_name: String,
}

fn get_connection_details(location_text: &str, settings: &SettingsState) -> ConnectionDetails {
    let location = crate::models::LocationInfo::from_string(location_text);

    let server_code = match location.country.as_str() {
        "Sweden" => "se",
        "United States" => "us",
        "Germany" => "de",
        "United Kingdom" => "gb",
        "Netherlands" => "nl",
        _ => "un",
    };
    let city_lower = location.city.to_lowercase();
    let city_code = city_lower.get(0..3).unwrap_or("unk");

    let server_name = if settings.multi_hop {
        format!("{}-{}-101 via se-sto-001", server_code, city_code)
    } else {
        format!("{}-{}-101", server_code, city_code)
    };

    ConnectionDetails {
        country: location.country,
        city: location.city,
        server_name,
    }
}

fn get_active_features(settings: &SettingsState) -> Vec<String> {
    let mut features = Vec::new();
    if settings.daita_enabled {
        features.push("DAITA".to_string());
    }
    if settings.quantum_resistant {
        features.push("Quantum resistance".to_string());
    }
    if settings.multi_hop {
        features.push("Multihop".to_string());
    }
    if settings.split_tunneling {
        features.push("Split tunneling".to_string());
    }
    if settings.lockdown_mode {
        features.push("Lockdown mode".to_string());
    }
    if settings.obfuscation {
        features.push("Obfuscation".to_string());
    }
    if settings.local_sharing {
        features.push("Local network sharing".to_string());
    }

    let dns_active = [
        (settings.dns_blocking.ads, "Ads"),
        (settings.dns_blocking.trackers, "Trackers"),
        (settings.dns_blocking.malware, "Malware"),
        (settings.dns_blocking.gambling, "Gambling"),
        (settings.dns_blocking.adult_content, "Adult Content"),
        (settings.dns_blocking.social_media, "Social Media"),
    ];
    let active_dns_count = dns_active.iter().filter(|(active, _)| *active).count();

    match active_dns_count {
        1 => {
            if let Some((_, name)) = dns_active.iter().find(|(active, _)| *active) {
                features.push(name.to_string());
            }
        }
        n if n > 1 => features.push("DNS content blockers".to_string()),
        _ => {}
    }
    features
}

#[component]
pub fn ConnectionOverlay() -> Element {
    let mut state = use_context::<ConnectionState>();
    let nav = use_navigator();
    let vpn = use_vpn_client();
    let i18n = crate::hooks::use_i18n();
    let status = (state.status)();
    let settings = (state.settings)();
    let location_text = (state.current_location)();

    let details = get_connection_details(&location_text, &settings);
    let features = get_active_features(&settings);

    let button_color_class = match status {
        ConnectionStatus::Connected => "bg-status-error text-white shadow-status-error/40",
        ConnectionStatus::Connecting => "bg-status-warning shadow-status-warning/40",
        ConnectionStatus::Disconnecting => "bg-orange-500 shadow-orange-500/40",
        ConnectionStatus::Disconnected => "bg-status-success text-white shadow-status-success/40",
    };

    let status_color = match status {
        ConnectionStatus::Connected => "text-status-success",
        ConnectionStatus::Connecting | ConnectionStatus::Disconnecting => "text-status-warning",
        ConnectionStatus::Disconnected => "text-status-error",
    };

    rsx! {
        div {
            class: "fixed left-0 right-0 px-4 z-[100] pointer-events-none",
            style: "bottom: 16px; width: {WINDOW_WIDTH}px;",
            div { class: "pointer-events-auto flex flex-col items-start gap-1 mx-auto bg-card/60 backdrop-blur-xl border border-white/10 p-5 rounded-2xl shadow-2xl",

                // Status text
                div { class: "{status_color} text-xs font-bold uppercase tracking-wider mb-1",
                    {
                        match status {
                            ConnectionStatus::Connected => i18n.tr("connected"),
                            ConnectionStatus::Connecting => i18n.tr("connecting"),
                            ConnectionStatus::Disconnecting => i18n.tr("disconnecting"),
                            ConnectionStatus::Disconnected => i18n.tr("disconnected"),
                        }
                    }
                }

                // Location info
                div { class: "flex flex-col items-start gap-0.5 mb-2",
                    if settings.multi_hop {
                        div { class: "flex flex-col gap-0.5",
                            div { class: "flex items-center gap-2",
                                span { class: "text-[10px] font-bold text-primary uppercase",
                                    "Entry"
                                }
                                span { class: "text-sm font-bold", "{settings.entry_location}" }
                            }
                            div { class: "flex items-center gap-2",
                                span { class: "text-[10px] font-bold text-status-success uppercase",
                                    "Exit"
                                }
                                span { class: "text-sm font-bold", "{settings.exit_location}" }
                            }
                        }
                    } else {
                        div { class: "font-bold text-lg leading-tight",
                            "{details.country}, {details.city}"
                        }
                        div { class: "text-[10px] text-muted-foreground font-medium uppercase tracking-widest",
                            "{details.server_name}"
                        }
                    }
                }

                // Features
                div { class: "flex flex-wrap items-center gap-1.5 mb-4",
                    {
                        let limit = 4;
                        let show_more = features.len() > limit;
                        let display_features: Vec<String> = if show_more {
                            features.iter().take(limit).cloned().collect()
                        } else {
                            features.clone()
                        };

                        rsx! {
                            for feature in display_features {
                                button {
                                    class: "inline-block px-3 py-1.5 rounded-md bg-white/10 text-[10px] font-bold text-muted-foreground border border-white/5 leading-none hover:bg-white/20 transition-colors no-drag",
                                    onclick: move |_| {
                                        let target = match feature.as_str() {
                                            "DAITA" => "daita",
                                            "Quantum resistance" => "quantum-resistant",
                                            "Multihop" => "multi-hop",
                                            "Split tunneling" => "split-tunneling",
                                            "Lockdown mode" => "lockdown-mode",
                                            "Obfuscation" => "obfuscation",
                                            "Local network sharing" => "local-sharing",
                                            "Ads" | "Trackers" | "Malware" | "Gambling" | "Adult Content"
                                            | "Social Media" | "DNS content blockers" => "dns-blocking",
                                            _ => "general",
                                        };
                                        state.scroll_to.set(Some(target.to_string()));
                                        nav.push(Route::Settings {});
                                    },
                                    "{feature}"
                                }
                            }
                            if show_more {
                                {
                                    let more_count = features.len() - limit;
                                    rsx! {
                                        button {
                                            class: "inline-block px-3 py-1.5 rounded-md bg-white/10 text-[10px] font-bold text-muted-foreground border border-white/5 leading-none hover:bg-white/20 transition-colors no-drag",
                                            onclick: move |_| {
                                                state.scroll_to.set(Some("general".to_string()));
                                                nav.push(Route::Settings {});
                                            },
                                            "+{more_count} more"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Switch location and reload
                div { class: "flex items-center w-full mb-3 border border-white/10 rounded overflow-hidden bg-white/5",
                    button {
                        class: "flex-1 h-8 flex items-center justify-center hover:bg-white/5 text-foreground transition-all active:scale-[0.98] no-drag text-xs font-semibold",
                        onclick: move |_| {
                            nav.push(Route::Locations {});
                        },
                        {i18n.tr("switch_location")}
                    }
                    // Vertical divider
                    div { class: "w-[1px] h-4 bg-white/10" }
                    button {
                        class: "w-10 h-8 flex items-center justify-center hover:bg-white/5 text-muted-foreground transition-all active:scale-[0.98] no-drag",
                        onclick: move |_| {
                            if status == ConnectionStatus::Connected {
                                let loc = (state.current_location)();
                                vpn.disconnect();
                                vpn.connect(loc);
                            }
                        },
                        RefreshCw { size: 14 }
                    }
                }

                // Main connect button
                button {
                    onclick: move |_| vpn.toggle(),
                    disabled: matches!(status, ConnectionStatus::Connecting | ConnectionStatus::Disconnecting),
                    class: "group relative h-8 flex items-center justify-center w-full rounded shadow-xl hover:brightness-110 transition-all duration-300 cursor-pointer disabled:opacity-80 disabled:cursor-not-allowed text-sm font-bold {button_color_class} no-drag",
                    if status == ConnectionStatus::Connecting {
                        Loader {
                            size: 16,
                            class: Some("animate-spin mr-2".to_string()),
                        }
                        {i18n.tr("connecting")}
                    } else if status == ConnectionStatus::Disconnecting {
                        Loader {
                            size: 16,
                            class: Some("animate-spin mr-2".to_string()),
                        }
                        {i18n.tr("disconnecting")}
                    } else {
                        match status {
                            ConnectionStatus::Connected => i18n.tr("disconnect"),
                            ConnectionStatus::Disconnected => i18n.tr("connect"),
                            _ => "Action",
                        }
                    }
                }
            }
        }
    }
}
