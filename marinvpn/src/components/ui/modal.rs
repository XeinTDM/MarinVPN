use dioxus::prelude::*;
use crate::icons::Info;

#[component]
pub fn Modal(
    title: String,
    children: Element,
    onclose: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        div { 
            class: "fixed inset-0 z-50 flex items-center justify-center p-4 animate-in fade-in duration-200",
            // Backdrop
            div { 
                class: "absolute inset-0 bg-background/80 backdrop-blur-sm",
                onclick: move |e| onclose.call(e),
            }
            // Dialog
            div { 
                class: "relative w-full max-w-[320px] bg-card border border-border rounded-2xl shadow-2xl p-6 flex flex-col animate-in zoom-in-95 duration-200",
                h3 { class: "text-lg font-bold mb-4", "{title}" }
                {children}
            }
        }
    }
}

#[component]
pub fn InfoDialog(
    title: String,
    content: Element,
    onclose: EventHandler<MouseEvent>,
    button_text: Option<String>,
) -> Element {
    let btn_text = button_text.unwrap_or_else(|| "Got it!".to_string());
    rsx! {
        div { 
            class: "fixed inset-0 z-50 flex items-center justify-center p-4 animate-in fade-in duration-200",
            // Backdrop
            div { 
                class: "absolute inset-0 bg-background/80 backdrop-blur-sm",
                onclick: move |e| onclose.call(e),
            }
            // Dialog
            div { 
                class: "relative w-full max-w-[280px] bg-card border border-border rounded-2xl shadow-2xl p-6 flex flex-col items-center text-center animate-in zoom-in-95 duration-200",
                
                div { class: "w-12 rounded-full bg-primary/10 flex items-center justify-center mb-4 shrink-0",
                style: "height: 48px !important; min-height: 48px !important;",
                    Info { size: 24, class: Some("text-primary".to_string()) }
                }

                h3 { class: "text-lg font-bold mb-4", "{title}" }
                
                div { class: "text-xs text-muted-foreground leading-relaxed mb-6 w-full text-left",
                    {content}
                }

                button { 
                    class: "w-full h-11 bg-primary text-primary-foreground font-bold rounded-xl hover:opacity-90 transition-all active:scale-95 shadow-lg shadow-primary/20",
                    onclick: move |e| onclose.call(e),
                    "{btn_text}"
                }
            }
        }
    }
}
