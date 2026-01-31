use dioxus::prelude::*;
use crate::icons::ArrowLeft;

#[component]
pub fn BackButton() -> Element {
    let nav = use_navigator();
    rsx! {
        button { 
            class: "w-12 flex items-center justify-center hover:bg-accent rounded-xl transition-all no-drag text-foreground active:scale-90 shadow-sm border border-transparent hover:border-border shrink-0",
            style: "height: 48px !important; min-height: 48px !important;",
            onclick: move |_| { nav.go_back(); },
            ArrowLeft { size: 24 }
        }
    }
}

#[component]
pub fn LargeButton(
    label: String,
    description: String,
    icon: Element,
    icon_class: String,
    onclick: EventHandler<MouseEvent>
) -> Element {
    rsx! {
        button { 
            class: "w-full bg-card hover:bg-accent/40 border border-border rounded-2xl p-4 flex items-center gap-4 transition-all group text-left active:scale-95 shadow-sm",
            onclick: move |e| onclick.call(e),
            div { class: "p-3 rounded-xl transition-colors {icon_class}",
                {icon}
            }
            div { class: "flex-1",
                div { class: "font-bold mb-0.5 text-foreground", "{label}" }
                div { class: "text-[11px] text-muted-foreground font-medium", "{description}" }
            }
            crate::icons::ChevronRight { size: 18, class: Some("text-muted-foreground group-hover:text-foreground".to_string()) }
        }
    }
}
