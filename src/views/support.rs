use dioxus::prelude::*;
use crate::icons::*;
use crate::components::ui::button::LargeButton;

#[component]
pub fn Support() -> Element {
    rsx! {
        div { class: "h-full p-4 overflow-y-auto bg-background text-foreground custom-scrollbar",
            div { class: "space-y-4",
                LargeButton {
                    label: "Report a Problem",
                    description: "Found a bug? Let us know.",
                    icon_class: "bg-destructive/10 text-destructive group-hover:bg-destructive/20",
                    icon: rsx! { TriangleAlert { size: 24 } },
                    onclick: move |_| { }
                }

                LargeButton {
                    label: "FAQ & Guides",
                    description: "Learn how to use features.",
                    icon_class: "bg-status-info/10 text-status-info group-hover:bg-status-info/20",
                    icon: rsx! { BookOpen { size: 24 } },
                    onclick: move |_| { }
                }

                LargeButton {
                    label: "Contact Support",
                    description: "Get help from our team.",
                    icon_class: "bg-status-success/10 text-status-success group-hover:bg-status-success/20",
                    icon: rsx! { MessageCircle { size: 24 } },
                    onclick: move |_| { }
                }
            }

            div { class: "mt-10 text-center",
                p { class: "text-[10px] text-muted-foreground font-bold uppercase tracking-widest", "Support ID" }
                p { class: "text-xs font-mono text-foreground mt-1", "8A29-1B4C-9D0E" }
            }
        }
    }
}
