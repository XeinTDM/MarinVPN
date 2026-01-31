use dioxus::prelude::*;
use crate::icons::*;
use crate::components::*;
use crate::state::ConnectionState;
use crate::models::Language;

#[component]
pub fn UiSettings() -> Element {
    let mut state = use_context::<ConnectionState>();
    let settings = state.settings.read();
    let i18n = crate::hooks::use_i18n();
    let mut lang_expanded = use_signal(|| false);

    rsx! {
        div { class: "divide-y divide-border/30 -mx-4",
            SettingRow { 
                label: i18n.tr("connection_status_notif").to_string(), 
                checked: true,
                onclick: move |_| { /* Toggle */ }
            }
            SettingRow { 
                label: i18n.tr("security_alerts").to_string(), 
                checked: true,
                onclick: move |_| { /* Toggle */ }
            }
            SettingRow { 
                label: i18n.tr("new_locations").to_string(), 
                checked: false,
                onclick: move |_| { /* Toggle */ }
            }

            // Language Selection
            div { class: "flex flex-col",
                SettingCollapsible {
                    label: i18n.tr("select_language").to_string(),
                    expanded: lang_expanded(),
                    onclick: move |_| lang_expanded.set(!lang_expanded()),
                }

                if lang_expanded() {
                    div { class: "bg-accent/5 divide-y divide-border/20",
                        for lang in Language::all() {
                            div { 
                                class: "px-4 flex items-center justify-between hover:bg-accent/20 transition-colors cursor-pointer last:border-0 shrink-0",
                                style: "height: 48px !important; min-height: 48px !important;",
                                onclick: move |_| {
                                    state.settings.with_mut(|s| s.language = *lang);
                                },
                                span { class: "text-sm font-bold text-foreground", "{lang.name()}" }
                                if settings.language == *lang {
                                    CircleCheck { size: 16, class: Some("text-primary".to_string()) }
                                }
                            }
                        }
                    }
                }
            }

            SettingRow { 
                label: i18n.tr("dark_mode").to_string(), 
                checked: settings.dark_mode,
                onclick: move |_| {
                    state.settings.with_mut(|s| s.dark_mode = !s.dark_mode);
                }
            }
        }
    }
}
