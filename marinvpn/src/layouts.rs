use dioxus::prelude::*;
use crate::icons::*;
use crate::Route;
use crate::state::ConnectionState;
use crate::components::{BackButton, ConnectionOverlay};

use crate::window::{WINDOW_WIDTH, WINDOW_HEIGHT};

#[component]
pub fn MainLayout() -> Element {
    let state = use_context::<ConnectionState>();
    let nav = use_navigator();
    let i18n = crate::hooks::use_i18n();
    
    use_effect(move || {
        if (state.account_number)().is_none() {
            nav.replace(Route::Login {});
        }
    });

    if (state.account_number)().is_none() {
        return rsx! {
            div { class: "h-full w-full bg-background flex items-center justify-center",
                Loader {
                    size: 40,
                    class: Some("animate-spin text-primary".to_string()),
                }
            }
        };
    }

    let route = use_route::<Route>();
    
    let is_dashboard = matches!(route, Route::Dashboard {});
    let is_sub_page = !is_dashboard;

    let show_overlay = is_dashboard;

    let title = route.title().map(|key| i18n.tr(key));

    rsx! {
        div {
            class: "flex flex-col bg-background text-foreground font-sans select-none overflow-hidden pointer-events-auto relative",
            style: "height: {WINDOW_HEIGHT}px; width: {WINDOW_WIDTH}px;",
            // Top Header & Navigation
            div { class: "flex flex-col bg-background/95 backdrop-blur-md z-50 border-b border-border/50",
                if is_sub_page {
                    div { class: "flex items-center gap-3 px-4 py-2",
                        BackButton {}
                        if let Some(t) = title {
                            h2 { class: "text-lg font-bold tracking-tight", "{t}" }
                        }
                        div { class: "flex-1 h-10 drag-region" }
                    }
                } else {
                    div { class: "flex items-center justify-between px-4 py-2",
                        Link {
                            to: Route::Dashboard {},
                            class: "flex items-center gap-2 hover:opacity-80 transition-opacity no-drag",
                            img {
                                src: asset!("/assets/logo.png"),
                                class: "h-6 w-auto",
                            }
                            span { class: "font-bold text-lg tracking-tight", "MarinVPN" }
                        }

                        div { class: "flex-1 h-8 drag-region" }

                        div { class: "flex items-center gap-1 no-drag",
                            NavItem {
                                to: Route::Account {},
                                icon: rsx! {
                                    User { size: 20 }
                                },
                            }
                            NavItem {
                                to: Route::Settings {},
                                icon: rsx! {
                                    Settings { size: 20 }
                                },
                            }
                        }
                    }
                }
            }

            // Content Area
            div { class: "flex-1 relative overflow-hidden flex flex-col", Outlet::<Route> {} }

            // Connection controls (Floating Overlay)
            if show_overlay {
                ConnectionOverlay {}
            }
        }
    }
}

#[component]
fn NavItem(to: Route, icon: Element, #[props(default)] label: Option<&'static str>) -> Element {
    rsx! {
        Link {
            to,
            class: "flex items-center justify-center p-2 gap-1.5 text-muted-foreground hover:text-primary transition-colors active:text-primary rounded-xl no-drag",
            active_class: "text-primary bg-primary/10",
            {icon}
            if let Some(l) = label {
                span { class: "text-xs font-medium", "{l}" }
            }
        }
    }
}
