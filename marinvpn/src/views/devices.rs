use crate::components::toast::{use_toast, ToastType};
use crate::icons::{RefreshCw, X};
use crate::services::auth::AuthService;
use crate::state::ConnectionState;
use dioxus::prelude::*;

#[component]
pub fn Devices() -> Element {
    let state = use_context::<ConnectionState>();
    let mut toast = use_toast();
    let account_number = (state.account_number)().unwrap_or_default();
    let auth_token = (state.auth_token)().unwrap_or_default();

    let mut devices_resource = {
        let acc = account_number.clone();
        let token = auth_token.clone();
        use_resource(move || {
            let acc = acc.clone();
            let token = token.clone();
            async move { AuthService::get_devices(&acc, &token).await }
        })
    };

    rsx! {
        div { class: "h-full w-full flex flex-col bg-background",
            div { class: "flex-1 overflow-y-auto custom-scrollbar",
                div { class: "space-y-3 pb-24",
                    match &*devices_resource.value().read() {
                        Some(Ok(devices)) => rsx! {
                            for device in devices {
                                {
                                    let name = device.name.clone();
                                    let name_for_action = name.clone();
                                    let is_current = name == (state.device_name)();
                                    let date_str = device.created_date.clone();
                                    let display_msg = if is_current { format!("Added on {} (Now)", date_str) } else { format!("Added on {}", date_str) };
                                    let acc_for_remove = account_number.clone();
                                    let token_for_remove = auth_token.clone();

                                    rsx! {
                                        div {
                                            key: "{name}",
                                            class: "px-1",
                                            div {
                                                class: "bg-card rounded-2xl p-4 border shadow-sm flex items-center justify-between transition-all",
                                                class: if is_current { "border-primary/20 bg-card/50" } else { "border-border hover:border-muted-foreground/20" },
                                                div { class: "flex items-center gap-4",
                                                    div {
                                                        div { class: "text-sm font-bold text-foreground capitalize",
                                                            "{name}"
                                                        }
                                                        div { class: "text-[10px] text-muted-foreground font-medium",
                                                            "{display_msg}"
                                                        }
                                                    }
                                                }
                                                if is_current {
                                                    div { class: "px-2 py-1 bg-primary/10 text-primary rounded text-[9px] font-bold uppercase tracking-wider",
                                                        "This Device"
                                                    }
                                                } else {
                                                    button {
                                                        class: "w-12 flex items-center justify-center hover:bg-destructive/10 text-muted-foreground hover:text-destructive rounded-xl transition-all active:scale-90",
                                                        style: "height: 48px !important; min-height: 48px !important; flex-shrink: 0 !important;",
                                                        onclick: move |_| {
                                                            let acc = acc_for_remove.clone();
                                                            let token = token_for_remove.clone();
                                                            let dev_name = name_for_action.clone();
                                                            spawn(async move {
                                                                match AuthService::remove_device(&acc, &dev_name, &token).await {
                                                                    Ok(_) => {
                                                                        devices_resource.restart();
                                                                        toast.show(&format!("Removed {}", dev_name), ToastType::Success);
                                                                    }
                                                                    Err(e) => toast.show(&e.user_friendly_message(), ToastType::Error),
                                                                }
                                                            });
                                                        },
                                                        X { size: 20 }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        },
                        Some(Err(e)) => rsx! {
                            div { class: "p-8 text-center text-destructive text-xs", "Failed to load devices: {e.user_friendly_message()}" }
                        },
                        None => rsx! {
                            div { class: "p-8 flex justify-center", RefreshCw { class: "animate-spin text-muted-foreground", size: 24 } }
                        }
                    }
                }
            }
        }
    }
}
