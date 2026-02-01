use crate::components::toast::{ToastManager, ToastType};
use crate::components::*;
use crate::icons::*;
use crate::models::Language;
use crate::state::ConnectionState;
use dioxus::prelude::*;
use rfd::FileDialog;
use std::path::Path;

#[component]
pub fn UiSettings() -> Element {
    let mut state = use_context::<ConnectionState>();
    let mut toasts = use_context::<ToastManager>();
    let settings = state.settings.read();
    let i18n = crate::hooks::use_i18n();
    let mut lang_expanded = use_signal(|| false);
    let mut branding_expanded = use_signal(|| false);
    let mut logo_preview = use_signal(|| settings.branding_logo_path.clone());

    rsx! {
        div { class: "divide-y divide-border/30 -mx-4",
            // Branding
            div { class: "flex flex-col",
                SettingCollapsible {
                    label: i18n.tr("branding").to_string(),
                    expanded: branding_expanded(),
                    onclick: move |_| branding_expanded.set(!branding_expanded()),
                }

                if branding_expanded() {
                    div { class: "bg-accent/5 divide-y divide-border/20",
                        div { class: "px-4 py-3 flex flex-col gap-2",
                            label { class: "text-[11px] font-bold uppercase tracking-widest text-muted-foreground", {i18n.tr("branding_preset")} }
                            div { class: "grid grid-cols-3 gap-2",
                                button {
                                    class: "rounded-xl border border-border py-2 text-[10px] font-bold uppercase tracking-widest transition-all {preset_class(&settings.branding_preset, \"stealth\")}",
                                    onclick: move |_| {
                                        apply_preset(&mut state, "stealth", &mut logo_preview);
                                    },
                                    {i18n.tr("branding_preset_stealth")}
                                }
                                button {
                                    class: "rounded-xl border border-border py-2 text-[10px] font-bold uppercase tracking-widest transition-all {preset_class(&settings.branding_preset, \"neutral\")}",
                                    onclick: move |_| {
                                        apply_preset(&mut state, "neutral", &mut logo_preview);
                                    },
                                    {i18n.tr("branding_preset_neutral")}
                                }
                                button {
                                    class: "rounded-xl border border-border py-2 text-[10px] font-bold uppercase tracking-widest transition-all {preset_class(&settings.branding_preset, \"custom\")}",
                                    onclick: move |_| {
                                        state.settings.with_mut(|s| s.branding_preset = "custom".to_string());
                                    },
                                    {i18n.tr("branding_preset_custom")}
                                }
                            }
                        }
                        div { class: "px-4 py-3 flex flex-col gap-2",
                            label { class: "text-[11px] font-bold uppercase tracking-widest text-muted-foreground", {i18n.tr("branding_name")} }
                            input {
                                class: "w-full bg-background border border-border rounded-xl px-3 py-2 text-xs font-medium focus:outline-none focus:ring-2 focus:ring-primary/20 transition-all",
                                value: "{settings.branding_name}",
                                oninput: move |e| {
                                    let val = e.value();
                                    state.settings.with_mut(|s| s.branding_name = val);
                                    state.settings.with_mut(|s| s.branding_preset = "custom".to_string());
                                },
                            }
                        }
                        div { class: "px-4 py-3 flex items-center justify-between gap-3",
                            div { class: "flex flex-col gap-1",
                                label { class: "text-[11px] font-bold uppercase tracking-widest text-muted-foreground", {i18n.tr("branding_color")} }
                                span { class: "text-[10px] text-muted-foreground", {i18n.tr("branding_color_hint")} }
                            }
                            input {
                                class: "h-10 w-16 rounded-lg border border-border bg-background",
                                r#type: "color",
                                value: "{settings.branding_accent_color}",
                                oninput: move |e| {
                                    let val = e.value();
                                    state.settings.with_mut(|s| s.branding_accent_color = val);
                                    state.settings.with_mut(|s| s.branding_preset = "custom".to_string());
                                },
                            }
                        }
                        div { class: "px-4 py-3 flex flex-col gap-2",
                            label { class: "text-[11px] font-bold uppercase tracking-widest text-muted-foreground", {i18n.tr("branding_logo")} }
                            input {
                                class: "w-full bg-background border border-border rounded-xl px-3 py-2 text-xs font-medium focus:outline-none focus:ring-2 focus:ring-primary/20 transition-all",
                                placeholder: i18n.tr("branding_logo_hint"),
                                value: "{logo_preview}",
                                oninput: move |e| {
                                    let val = e.value();
                                    logo_preview.set(val);
                                    state.settings.with_mut(|s| s.branding_preset = "custom".to_string());
                                },
                            }
                        }
                        div { class: "px-4 pb-3 flex items-center gap-2",
                            button {
                                class: "flex-1 bg-accent/30 hover:bg-accent border border-border rounded-xl text-xs font-bold py-2 transition-all active:scale-95",
                                onclick: move |_| {
                                    if let Some(path) = FileDialog::new()
                                        .add_filter("Image", &["png", "jpg", "jpeg", "ico"])
                                        .pick_file()
                                    {
                                        let path_str = path.to_string_lossy().to_string();
                                        match validate_logo_path(&path_str) {
                                            Ok(_) => {
                                                logo_preview.set(path_str.clone());
                                                state.settings.with_mut(|s| s.branding_logo_path = path_str);
                                                state.settings.with_mut(|s| s.branding_preset = "custom".to_string());
                                            }
                                            Err(msg) => toasts.show(&msg, ToastType::Error),
                                        }
                                    }
                                },
                                {i18n.tr("branding_pick_logo")}
                            }
                            button {
                                class: "flex-1 bg-card hover:bg-accent/40 border border-border rounded-xl text-xs font-bold py-2 transition-all active:scale-95",
                                onclick: move |_| {
                                    logo_preview.set(String::new());
                                    state.settings.with_mut(|s| s.branding_logo_path = String::new());
                                    state.settings.with_mut(|s| s.branding_preset = "custom".to_string());
                                },
                                {i18n.tr("branding_clear_logo")}
                            }
                            button {
                                class: "flex-1 bg-primary text-primary-foreground hover:brightness-110 rounded-xl text-xs font-bold py-2 transition-all active:scale-95",
                                onclick: move |_| {
                                    let val = logo_preview();
                                    if val.trim().is_empty() {
                                        state.settings.with_mut(|s| s.branding_logo_path = String::new());
                                        state.settings.with_mut(|s| s.branding_preset = "custom".to_string());
                                        return;
                                    }
                                    match validate_logo_path(&val) {
                                        Ok(_) => {
                                            state.settings.with_mut(|s| s.branding_logo_path = val);
                                            state.settings.with_mut(|s| s.branding_preset = "custom".to_string());
                                        }
                                        Err(msg) => toasts.show(&msg, ToastType::Error),
                                    }
                                },
                                {i18n.tr("branding_apply_logo")}
                            }
                        }
                        div { class: "px-4 py-3",
                            button {
                                class: "w-full bg-accent/30 hover:bg-accent border border-border rounded-xl text-xs font-bold py-2 transition-all active:scale-95",
                                onclick: move |_| {
                                    state.settings.with_mut(|s| {
                                        s.branding_name = "MarinVPN".to_string();
                                        s.branding_accent_color = "#6D28D9".to_string();
                                        s.branding_logo_path = "".to_string();
                                        s.branding_preset = "custom".to_string();
                                    });
                                    logo_preview.set(String::new());
                                },
                                {i18n.tr("branding_reset")}
                            }
                        }
                    }
                }
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

fn validate_logo_path(path: &str) -> Result<(), String> {
    let p = Path::new(path);
    if !p.exists() {
        return Err("Logo path does not exist.".to_string());
    }
    let ext = p
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if !matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "ico") {
        return Err("Logo must be a PNG, JPG, or ICO file.".to_string());
    }
    image::open(p).map_err(|_| "Logo file could not be read.".to_string())?;
    Ok(())
}

fn preset_class(current: &str, value: &str) -> &'static str {
    if current == value {
        "bg-primary/15 text-primary border-primary/30"
    } else {
        "bg-card hover:bg-accent/40 text-foreground"
    }
}

fn apply_preset(state: &mut ConnectionState, preset: &str, logo_preview: &mut Signal<String>) {
    let (name, color, logo) = match preset {
        "stealth" => ("System Monitor", "#111827", ""),
        "neutral" => ("Secure Client", "#0F172A", ""),
        _ => ("MarinVPN", "#6D28D9", ""),
    };

    state.settings.with_mut(|s| {
        s.branding_preset = preset.to_string();
        s.branding_name = name.to_string();
        s.branding_accent_color = color.to_string();
        s.branding_logo_path = logo.to_string();
    });
    logo_preview.set(logo.to_string());
}
