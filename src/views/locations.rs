use dioxus::prelude::*;
use crate::icons::*;
use crate::state::ConnectionState;
use crate::data::get_regions;
use crate::Route;
use crate::hooks::use_vpn_client;
use crate::components::toast::{use_toast, ToastType};
use crate::models::City;

#[component]
pub fn Locations() -> Element {
    let state = use_context::<ConnectionState>();
    let mut current_tab = use_signal(|| "All"); 

    let mut expanded_country = use_signal(|| Option::<String>::None);
    let mut search_query = use_signal(|| String::new());

    let filtered_regions = use_memo(move || {
        let regions = get_regions();
        let query = search_query().to_lowercase();
        let favs = state.favorites.read();
        let show_favs = current_tab() == "Favorites";
        
        regions.into_iter().filter(|region| {
            let matches_query = query.is_empty() || region.name.to_lowercase().contains(&query);
            if !matches_query {
                return false;
            }
            
            if show_favs {
                region.cities.iter().any(|c| favs.contains(&format!("{}, {}", region.name, c.name)))
            } else {
                true
            }
        }).collect::<Vec<_>>()
    });

    rsx! {
        div { class: "flex-1 flex flex-col bg-background overflow-hidden",
            div { class: "p-4 pb-2",
                div { class: "flex items-center gap-2 mb-2",
                    div { class: "relative flex-1",
                        div { class: "absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none",
                            Search { size: 16, class: "text-muted-foreground".to_string() }
                        }
                        input {
                            class: "w-full bg-card border border-border rounded-xl pl-10 pr-4 py-2 text-sm text-foreground placeholder-muted-foreground focus:outline-none focus:ring-2 focus:ring-primary/20 transition-all shadow-sm",
                            placeholder: "Search locations...",
                            value: "{search_query}",
                            oninput: move |e| search_query.set(e.value()),
                        }
                    }
                    button {
                        class: "p-2.5 rounded-xl border transition-all focus:outline-none",
                        class: if current_tab() == "Favorites" { "bg-status-warning/10 border-status-warning/30 text-status-warning" } else { "bg-card border-border text-muted-foreground hover:text-foreground" },
                        onclick: move |_| {
                            if current_tab() == "Favorites" {
                                current_tab.set("All");
                            } else {
                                current_tab.set("Favorites");
                            }
                        },
                        if current_tab() == "Favorites" {
                            Star { size: 18, fill: Some("currentColor".to_string()) }
                        } else {
                            Star { size: 18 }
                        }
                    }
                }
            }

            div { class: "flex-1 overflow-y-auto custom-scrollbar p-4 pt-2 space-y-3",
                for region in filtered_regions() {
                    div { class: "bg-card border border-border rounded-2xl overflow-hidden shadow-sm hover:border-muted transition-colors",
                        button { 
                            class: "w-full p-4 flex items-center justify-between cursor-pointer hover:bg-accent/30 transition-colors focus:outline-none focus:bg-accent/30",
                            onclick: move |_| {
                                if expanded_country() == Some(region.name.to_string()) {
                                    expanded_country.set(None);
                                } else {
                                    expanded_country.set(Some(region.name.to_string()));
                                }
                            },
                            div { class: "flex items-center gap-4",
                                span { class: "text-2xl", "{region.flag}" }
                                span { class: "font-bold text-foreground", "{region.name}" }
                            }
                            div { 
                                class: "text-muted-foreground transform transition-transform duration-300",
                                class: if expanded_country() == Some(region.name.to_string()) { "rotate-90 text-primary" } else { "" },
                                ChevronRight { size: 20 }
                            }
                        }

                        if expanded_country() == Some(region.name.to_string()) || current_tab() == "Favorites" {
                            div { class: "bg-background/40 border-t border-border/50 divide-y divide-border/30",
                                for city in region.cities {
                                    LocationItem { 
                                        city: city.clone(), 
                                        region_name: region.name,
                                        current_tab: current_tab()
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
fn LocationItem(city: City, region_name: &'static str, current_tab: &'static str) -> Element {
    let state = use_context::<ConnectionState>();
    let vpn = use_vpn_client();
    let nav = use_navigator();
    let mut toast = use_toast();
    let settings = state.settings.read();
    
    let location_name = format!("{}, {}", region_name, city.name);
    let is_fav = state.favorites.read().contains(&location_name);
    
    if current_tab == "Favorites" && !is_fav {
        return rsx! { };
    }

    let location_name_connect = location_name.clone();
    let location_name_fav = location_name.clone();

    rsx! {
        div { class: "px-4 py-3 pl-14 hover:bg-accent/20 flex items-center justify-between group transition-colors",
            button { 
                class: "flex items-center gap-3 flex-1 text-left focus:outline-none",
                onclick: move |_| {
                    vpn.connect(location_name_connect.clone());
                    nav.push(Route::Dashboard {});
                },
                div { 
                    class: "w-2 h-2 rounded-full shadow-[0_0_8px_currentColor]",
                    class: if city.load < 50 { "text-status-success bg-current" } else if city.load < 80 { "text-status-warning bg-current" } else { "text-status-error bg-current" }
                }
                div {
                    div { class: "text-foreground font-medium", "{city.name}" }
                    div { class: "text-[11px] text-muted-foreground font-mono", "{city.ping}ms • {city.load}% load" }
                }
            }
            
            div { class: "flex items-center gap-2",
                if settings.multi_hop {
                    div { class: "flex items-center gap-1.5",
                        button {
                            class: "px-2.5 py-1 rounded-md bg-white/5 hover:bg-white/10 border border-white/10 text-[10px] font-bold text-muted-foreground transition-colors",
                            onclick: move |e| {
                                e.stop_propagation();
                                toast.show(&format!("Entry point set to {}", city.name), ToastType::Info);
                            },
                            "ENTRY"
                        }
                        button {
                            class: "px-2.5 py-1 rounded-md bg-primary/10 hover:bg-primary/20 border border-primary/20 text-[10px] font-bold text-primary transition-colors",
                            onclick: move |e| {
                                e.stop_propagation();
                                toast.show(&format!("Exit point set to {}", city.name), ToastType::Success);
                            },
                            "EXIT"
                        }
                    }
                }

                button {
                    class: "p-2 hover:bg-accent rounded-lg transition-all",
                    class: if is_fav { "text-status-warning" } else { "text-muted-foreground" },
                    onclick: move |e| {
                        e.stop_propagation();
                        vpn.toggle_favorite(location_name_fav.clone());
                    },
                    if is_fav {
                        Star { size: 16, fill: Some("currentColor".to_string()) }
                    } else {
                        Star { size: 16 }
                    }
                }
            }
        }
    }
}