use crate::icons::*;
use dioxus::prelude::*;

#[component]
pub fn AppInfo() -> Element {
    let mut beta_joined = use_signal(|| false);
    let i18n = crate::hooks::use_i18n();
    let state = use_context::<crate::state::ConnectionState>();
    let branding = state.settings.read();
    let branding_name = branding.branding_name.clone();
    let branding_logo = branding.branding_logo_path.clone();

    rsx! {
        div { class: "h-full p-4 overflow-y-auto bg-background text-foreground custom-scrollbar",
            div { class: "flex flex-col items-center mb-8",
                div { class: "w-16 h-16 bg-primary rounded-[2rem] flex items-center justify-center text-primary-foreground shadow-2xl shadow-primary/20 mb-4 rotate-3 overflow-hidden",
                    if branding_logo.is_empty() {
                        ShieldCheck { size: 32, stroke_width: 2 }
                    } else {
                        img { src: "{branding_logo}", class: "h-10 w-10 object-contain" }
                    }
                }
                h3 { class: "text-2xl font-bold text-foreground", "{branding_name}" }
                p { class: "text-muted-foreground text-xs font-bold uppercase tracking-widest mt-1", "Version 0.1.5" }
            }

            div { class: "space-y-6",
                div { class: "bg-card rounded-2xl p-5 border border-border shadow-sm",
                    div { class: "flex items-start gap-4",
                        div { class: "p-2 bg-primary/10 rounded-xl text-primary",
                            FlaskConical { size: 20 }
                        }
                        div { class: "flex-1",
                            h4 { class: "font-bold text-lg mb-1 text-foreground", {i18n.tr("beta_program")} }
                            p { class: "text-[11px] text-muted-foreground font-medium mb-4 leading-relaxed", {i18n.tr("beta_desc")} }

                            if beta_joined() {
                                button {
                                    class: "w-full h-[48px] min-h-[48px] bg-accent/50 hover:bg-accent border border-border rounded-xl text-xs font-bold transition-all active:scale-95 text-foreground flex items-center justify-center",
                                    onclick: move |_| beta_joined.set(false),
                                    {i18n.tr("leave_beta")}
                                }
                            } else {
                                button {
                                    class: "w-full bg-primary hover:brightness-110 text-primary-foreground rounded-xl text-xs font-bold transition-all active:scale-95 shadow-lg shadow-primary/20 flex items-center justify-center shrink-0",
                                    style: "height: 48px !important; min-height: 48px !important;",
                                    onclick: move |_| beta_joined.set(true),
                                    {i18n.tr("join_beta")}
                                }
                            }
                        }
                    }
                }

                div {
                    h4 { class: "text-[10px] font-bold text-muted-foreground uppercase tracking-widest mb-3 ml-1", {i18n.tr("whats_new")} }
                    div { class: "space-y-4",
                        ChangeLogItem {
                            version: "v0.1.5",
                            date: "Jan 29, 2026",
                            changes: vec![
                                "Added Quantum-resistant tunnel option.",
                                "Implemented granular DNS content blocking.",
                                "Added support for multi-hop connections.",
                                "Improved UI for tray mode."
                            ]
                        }
                        ChangeLogItem {
                            version: "v0.1.4",
                            date: "Jan 15, 2026",
                            changes: vec![
                                "Added Favorites to location list.",
                                "Fixed connection timeout issues.",
                                "Visual improvements to the map."
                            ]
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn ChangeLogItem(version: &'static str, date: &'static str, changes: Vec<&'static str>) -> Element {
    rsx! {
        div { class: "bg-card rounded-2xl p-5 border border-border shadow-sm",
            div { class: "flex justify-between items-baseline mb-4",
                span { class: "font-bold text-primary", "{version}" }
                span { class: "text-[10px] font-bold text-muted-foreground uppercase", "{date}" }
            }
            ul { class: "space-y-3",
                for change in changes {
                    li { class: "flex items-start gap-3 text-xs text-foreground font-medium",
                        span { class: "text-primary mt-1", "•" }
                        span { "{change}" }
                    }
                }
            }
        }
    }
}
