use dioxus::prelude::*;
use crate::icons::*;
use crate::Route;
use crate::state::ConnectionState;
use crate::models::ConnectionStatus;
use crate::hooks::use_vpn_client;
use crate::window::WINDOW_WIDTH;

#[component]
pub fn ConnectionOverlay() -> Element {
    let mut state = use_context::<ConnectionState>();
    let nav = use_navigator();
    let vpn = use_vpn_client();
    let status = (state.status)();
    let settings = (state.settings)();
    
    let location_text = (state.current_location)();
    let country = location_text.split(',').next().unwrap_or("Unknown").trim();
    let city = location_text.split(',').nth(1).unwrap_or("Unknown").trim();

    let server_code = match country {
        "Sweden" => "se",
        "United States" => "us",
        "Germany" => "de",
        "United Kingdom" => "gb",
        "Netherlands" => "nl",
        _ => "un",
    };
    let city_lower = city.to_lowercase();
    let city_code = city_lower.get(0..3).unwrap_or("unk");
    
    let server_name = if settings.multi_hop {
        format!("{}-{}-101 via se-sto-001", server_code, city_code)
    } else {
        format!("{}-{}-101", server_code, city_code)
    };

    let mut features = Vec::new();
    if settings.daita_enabled { features.push("DAITA".to_string()); }
    if settings.quantum_resistant { features.push("Quantum resistance".to_string()); }
    if settings.multi_hop { features.push("Multihop".to_string()); }
    if settings.split_tunneling { features.push("Split tunneling".to_string()); }
    if settings.lockdown_mode { features.push("Lockdown mode".to_string()); }
    if settings.obfuscation { features.push("Obfuscation".to_string()); }
    if settings.local_sharing { features.push("Local network sharing".to_string()); }

    let dns_active = [
        (settings.dns_blocking.ads, "Ads"),
        (settings.dns_blocking.trackers, "Trackers"),
        (settings.dns_blocking.malware, "Malware"),
        (settings.dns_blocking.gambling, "Gambling"),
        (settings.dns_blocking.adult_content, "Adult Content"),
        (settings.dns_blocking.social_media, "Social Media"),
    ];
    let active_dns_names: Vec<&str> = dns_active.iter()
        .filter(|(active, _)| *active)
        .map(|(_, name)| *name)
        .collect();
    
    if active_dns_names.len() == 1 {
        features.push(active_dns_names[0].to_string());
    } else if active_dns_names.len() > 1 {
        features.push("DNS content blockers".to_string());
    }

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
            div { 
                class: "pointer-events-auto flex flex-col items-start gap-1 mx-auto bg-card/60 backdrop-blur-xl border border-white/10 p-5 rounded-2xl shadow-2xl",
                
                // Status text
                div { 
                    class: "{status_color} text-xs font-bold uppercase tracking-wider mb-1",
                    match status {
                        ConnectionStatus::Connected => "Connected",
                        ConnectionStatus::Connecting => "Connecting",
                        ConnectionStatus::Disconnecting => "Disconnecting",
                        ConnectionStatus::Disconnected => "Disconnected",
                    }
                }

                // Location info
                div { class: "flex flex-col items-start gap-0.5 mb-2",
                    div { class: "font-bold text-lg leading-tight", "{country}, {city}" }
                    div { class: "text-[10px] text-muted-foreground font-medium uppercase tracking-widest", "{server_name}" }
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
                                            _ if feature == "DNS content blockers" || dns_active.iter().any(|(_, name)| feature == *name) => "dns-blocking",
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
                        "Switch Location"
                    }
                    // Vertical divider
                    div { class: "w-[1px] h-4 bg-white/10" }
                    button {
                        class: "w-10 h-8 flex items-center justify-center hover:bg-white/5 text-muted-foreground transition-all active:scale-[0.98] no-drag",
                        onclick: move |_| {
                            // Mock reload
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
                        "Connecting..."
                    } else if status == ConnectionStatus::Disconnecting {
                        Loader {
                            size: 16,
                            class: Some("animate-spin mr-2".to_string()),
                        }
                        "Disconnecting..."
                    } else {
                        match status {
                            ConnectionStatus::Connected => "Disconnect",
                            ConnectionStatus::Disconnected => "Connect",
                            _ => "Action",
                        }
                    }
                }
            }
        }
    }
}
