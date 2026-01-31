use dioxus::prelude::*;

use crate::icons::{ChevronRight, Info, Check};

#[component]
pub fn SettingRow(
    label: String, 
    checked: bool, 
    onclick: EventHandler<MouseEvent>, 
    id: Option<String>,
    class: Option<String>,
    show_info: Option<bool>,
    oninfo: Option<EventHandler<MouseEvent>>,
) -> Element {
    let class_str = class.as_deref().unwrap_or_default();
    
    rsx! {
        div { 
            id,
            class: "flex items-center justify-between px-4 hover:bg-accent/30 cursor-pointer transition-colors shrink-0 {class_str}",
            style: "height: 48px !important; min-height: 48px !important;",
            onclick: move |e| onclick.call(e),
            div { class: "flex items-center gap-2",
                span { class: "font-bold text-sm text-foreground", "{label}" }
                if show_info.unwrap_or(false) {
                    button { 
                        class: "text-muted-foreground hover:text-primary transition-colors p-1",
                        onclick: move |e| {
                            e.stop_propagation();
                            if let Some(handler) = oninfo {
                                handler.call(e);
                            }
                        },
                        Info { size: 14 }
                    }
                }
            }
            div { 
                class: "w-11 h-6 rounded-full relative transition-all duration-300 flex-shrink-0",
                class: if checked { "bg-primary shadow-lg shadow-primary/30" } else { "bg-muted" },
                div { 
                    class: "absolute top-1 left-1 w-4 h-4 bg-white rounded-full transition-all duration-300 shadow-sm",
                    class: if checked { "translate-x-5" } else { "" }
                }
            }
        }
    }
}

#[component]
pub fn SettingDescription(text: String, class: Option<String>) -> Element {
    let class_str = class.unwrap_or_default();
    rsx! {
        div { class: "px-4 py-2 shrink-0 {class_str}",
            p { class: "text-xs text-muted-foreground leading-relaxed", "{text}" }
        }
    }
}

#[component]
pub fn SettingGap(height: u32, class: Option<String>) -> Element {
    let class_str = class.unwrap_or_default();
    rsx! {
        div { class: "shrink-0 {class_str}", style: "height: {height}px;" }
    }
}

#[component]
pub fn SettingAction(
    label: String, 
    value: Option<String>, 
    onclick: EventHandler<MouseEvent>,
    class: Option<String>,
) -> Element {
    let class_str = class.as_deref().unwrap_or_default();
    rsx! {
        div { 
            class: "flex items-center justify-between px-4 hover:bg-accent/30 cursor-pointer transition-colors shrink-0 {class_str}",
            style: "height: 48px !important; min-height: 48px !important;",
            onclick: move |e| onclick.call(e),
            span { class: "font-bold text-sm text-foreground", "{label}" }
            div { class: "flex items-center gap-2",
                if let Some(v) = value {
                    span { class: "text-xs text-muted-foreground", "{v}" }
                }
                ChevronRight { size: 16, class: Some("text-muted-foreground".to_string()) }
            }
        }
    }
}

#[component]
pub fn SettingSelectRow(
    label: String,
    selected: bool,
    onclick: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        div { 
            class: "flex items-center justify-between px-4 hover:bg-accent/30 cursor-pointer transition-colors shrink-0",
            style: "height: 48px !important; min-height: 48px !important;",
            onclick: move |e| onclick.call(e),
            span { class: "font-bold text-sm text-foreground", "{label}" }
            if selected {
                Check { size: 16, class: Some("text-primary".to_string()) }
            }
        }
    }
}

#[component]
pub fn SettingTitle(
    label: String,
    show_info: Option<bool>,
    oninfo: Option<EventHandler<MouseEvent>>,
) -> Element {
    rsx! {
        div { 
            class: "flex items-center gap-2 px-4 shrink-0",
            style: "height: 48px !important; min-height: 48px !important;",
            span { class: "font-bold text-sm text-foreground", "{label}" }
            if show_info.unwrap_or(false) {
                button { 
                    class: "text-muted-foreground hover:text-primary transition-colors p-1",
                    onclick: move |e| {
                        e.stop_propagation();
                        if let Some(handler) = oninfo {
                            handler.call(e);
                        }
                    },
                    Info { size: 14 }
                }
            }
        }
    }
}

#[component]
pub fn SettingInput(
    label: String,
    value: String,
    oninput: EventHandler<FormEvent>,
) -> Element {
    rsx! {
        div { 
            class: "flex items-center justify-between px-4 shrink-0",
            style: "height: 48px !important; min-height: 48px !important;",
            span { class: "font-bold text-sm text-foreground", "{label}" }
            input {
                class: "bg-transparent text-right text-sm text-foreground focus:outline-none w-24",
                value: "{value}",
                oninput: move |e| oninput.call(e),
            }
        }
    }
}

#[component]
pub fn SettingCollapsible(
    label: String,
    expanded: bool,
    onclick: EventHandler<MouseEvent>,
    show_info: Option<bool>,
    oninfo: Option<EventHandler<MouseEvent>>,
    id: Option<String>,
) -> Element {
    rsx! {
        div { 
            id,
            class: "flex items-center justify-between px-4 hover:bg-accent/30 cursor-pointer transition-colors shrink-0",
            style: "height: 48px !important; min-height: 48px !important;",
            onclick: move |e| onclick.call(e),
            div { class: "flex items-center gap-2",
                span { class: "font-bold text-sm text-foreground", "{label}" }
                if show_info.unwrap_or(false) {
                    button { 
                        class: "text-muted-foreground hover:text-primary transition-colors p-1",
                        onclick: move |e| {
                            e.stop_propagation();
                            if let Some(handler) = oninfo {
                                handler.call(e);
                            }
                        },
                        Info { size: 14 }
                    }
                }
            }
            div { 
                class: "mr-2 text-muted-foreground transition-transform duration-300",
                class: if expanded { "rotate-180" } else { "" },
                crate::icons::ChevronDown { size: 14 }
            }
        }
    }
}

#[component]
pub fn MenuRow(label: String, icon: Element, onclick: EventHandler<MouseEvent>) -> Element {
    rsx! {
        div { 
            class: "flex items-center justify-between px-4 hover:bg-accent/30 cursor-pointer transition-colors group shrink-0",
            style: "height: 48px !important; min-height: 48px !important;",
            onclick: move |e| onclick.call(e),
            div { class: "flex items-center gap-3",
                div { class: "text-muted-foreground group-hover:text-primary transition-colors flex items-center", {icon} }
                span { class: "font-bold text-sm text-foreground", "{label}" }
            }
            ChevronRight { size: 16, class: Some("text-muted-foreground".to_string()) }
        }
    }
}
