use dioxus::prelude::*;
use crate::icons::*;
use crate::state::ConnectionState;
use crate::components::toast::{use_toast, ToastType};

#[component]
pub fn Devices() -> Element {
    let state = use_context::<ConnectionState>();
    let mut toast = use_toast();
    
    let mut other_devices = use_signal(|| vec![
        ("silent forest", "Oct 12, 2025"),
        ("brave eagle", "Jan 05, 2026"),
        ("wild breeze", "Nov 20, 2025"),
    ]);

    rsx! {
        div { class: "h-full w-full flex flex-col bg-background",
            div { class: "flex-1 overflow-y-auto custom-scrollbar p-4",
                div { class: "space-y-3 pb-24",
                    // Current Device
                    div { class: "px-1",
                        div { class: "bg-card/50 rounded-2xl p-4 border border-primary/20 shadow-sm flex items-center justify-between",
                            div { class: "flex items-center gap-4",
                                div {
                                    div { class: "text-sm font-bold text-foreground capitalize", "{state.device_name}" }
                                    div { class: "text-[10px] text-muted-foreground font-medium", "Added on Jan 30, 2026 (Now)" }
                                }
                            }
                            div { class: "px-2 py-1 bg-primary/10 text-primary rounded text-[9px] font-bold uppercase tracking-wider", "This Device" }
                        }
                    }

                    // Other Devices
                    div { class: "px-1 space-y-3",
                        for (name, date) in other_devices() {
                            div { 
                                key: "{name}",
                                class: "bg-card rounded-2xl p-4 border border-border shadow-sm flex items-center justify-between group hover:border-muted-foreground/20 transition-colors",
                                div { class: "flex items-center gap-4",
                                    div {
                                        div { class: "text-sm font-bold text-foreground capitalize", "{name}" }
                                        div { class: "text-[10px] text-muted-foreground font-medium", "Added on {date}" }
                                    }
                                }
                                button { 
                                    class: "w-12 flex items-center justify-center hover:bg-destructive/10 text-muted-foreground hover:text-destructive rounded-xl transition-all active:scale-90",
                                    style: "height: 48px;",
                                    onclick: move |_| {
                                        let mut current = other_devices.peek().clone();
                                        current.retain(|(n, _)| *n != name);
                                        other_devices.set(current);
                                        toast.show(&format!("Removed {}", name), ToastType::Success);
                                    },
                                    X { size: 20 }
                                }
                            }
                        }
                        
                        if other_devices().is_empty() {
                            div { class: "py-8 text-center border-2 border-dashed border-border rounded-3xl",
                                p { class: "text-xs text-muted-foreground font-medium", "No other devices connected" }
                            }
                        }
                    }
                }
            }
        }
    }
}
