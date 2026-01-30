use dioxus::prelude::*;
use crate::icons::*;
use crate::components::SettingRow;
use crate::state::ConnectionState;

#[component]
pub fn UiSettings() -> Element {
    let mut state = use_context::<ConnectionState>();
    let settings = (state.settings)();

    rsx! {
        div { class: "divide-y divide-border/30 -mx-4",
            SettingRow { 
                label: "Connection Status", 
                checked: true,
                onclick: move |_| { /* Toggle */ }
            }
            SettingRow { 
                label: "Security Alerts", 
                checked: true,
                onclick: move |_| { /* Toggle */ }
            }
            SettingRow { 
                label: "New Locations", 
                checked: false,
                onclick: move |_| { /* Toggle */ }
            }

            for lang in ["English (US)", "Svenska", "Deutsch", "Français"] {
                div { 
                    class: "px-4 flex items-center justify-between hover:bg-accent/20 transition-colors cursor-pointer border-b border-border/50 last:border-0",
                    style: "height: 48px;",
                    onclick: move |_| { /* Select */ },
                    span { class: "text-sm font-bold text-foreground", "{lang}" }
                    if lang == "English (US)" {
                        CircleCheck { size: 16, class: Some("text-primary".to_string()) }
                    }
                }
            }

            SettingRow { 
                label: "Dark Mode", 
                checked: settings.dark_mode,
                onclick: move |_| {
                    state.settings.with_mut(|s| s.dark_mode = !s.dark_mode);
                }
            }
        }
    }
}
