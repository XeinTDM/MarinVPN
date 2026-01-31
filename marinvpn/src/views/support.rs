use crate::components::toast::{use_toast, ToastType};
use crate::components::ui::button::LargeButton;
use crate::components::ui::modal::Modal;
use crate::icons::*;
use crate::services::auth::AuthService;
use crate::state::ConnectionState;
use dioxus::prelude::*;

#[component]
pub fn Support() -> Element {
    let state = use_context::<ConnectionState>();
    let mut toast = use_toast();
    let i18n = crate::hooks::use_i18n();
    let mut show_report_modal = use_signal(|| false);
    let mut report_text = use_signal(String::new);
    let mut is_submitting = use_signal(|| false);

    rsx! {
        div { class: "h-full p-4 overflow-y-auto bg-background text-foreground custom-scrollbar",
            {
                if show_report_modal() {
                    rsx! {
                        Modal {
                            title: "Report a Problem",
                            onclose: move |_| show_report_modal.set(false),
                            div { class: "flex flex-col gap-4",
                                p { class: "text-xs text-muted-foreground",
                                    "Please describe the issue you are experiencing. This will be sent to our support team along with your support ID."
                                }
                                textarea {
                                    class: "w-full h-32 bg-accent/20 border border-border rounded-xl p-3 text-sm focus:outline-none focus:ring-2 focus:ring-primary/20 transition-all resize-none",
                                    placeholder: "Describe the bug...",
                                    value: "{report_text}",
                                    oninput: move |e| report_text.set(e.value()),
                                }
                                button {
                                    class: "w-full bg-primary hover:brightness-110 text-primary-foreground font-bold py-3 rounded-xl transition-all active:scale-95 flex items-center justify-center gap-2 disabled:opacity-50",
                                    disabled: is_submitting() || report_text().is_empty(),
                                    onclick: move |_| {
                                        is_submitting.set(true);
                                        let acc = (state.account_number)().unwrap_or_default();
                                        let token = (state.auth_token)().unwrap_or_default();
                                        let msg = report_text();
                                        spawn(async move {
                                            match AuthService::report_problem(&acc, &msg, &token).await {
                                                Ok(_) => {
                                                    toast.show("Report sent successfully", ToastType::Success);
                                                    show_report_modal.set(false);
                                                    report_text.set(String::new());
                                                }
                                                Err(e) => toast.show(&format!("Error: {}", e), ToastType::Error),
                                            }
                                            is_submitting.set(false);
                                        });
                                    },
                                    if is_submitting() {
                                        RefreshCw { class: "animate-spin", size: 18 }
                                        "Sending..."
                                    } else {
                                        "Submit Report"
                                    }
                                }
                            }
                        }
                    }
                } else {
                    rsx! { }
                }
            }

            div { class: "space-y-4",
                LargeButton {
                    label: i18n.tr("report_problem").to_string(),
                    description: i18n.tr("report_problem_desc").to_string(),
                    icon_class: "bg-destructive/10 text-destructive group-hover:bg-destructive/20".to_string(),
                    icon: rsx! { TriangleAlert { size: 24 } },
                    onclick: move |_| show_report_modal.set(true)
                }

                LargeButton {
                    label: i18n.tr("faq_guides").to_string(),
                    description: i18n.tr("faq_guides_desc").to_string(),
                    icon_class: "bg-status-info/10 text-status-info group-hover:bg-status-info/20".to_string(),
                    icon: rsx! { BookOpen { size: 24 } },
                    onclick: move |_| { }
                }

                LargeButton {
                    label: i18n.tr("contact_support").to_string(),
                    description: i18n.tr("contact_support_desc").to_string(),
                    icon_class: "bg-status-success/10 text-status-success group-hover:bg-status-success/20".to_string(),
                    icon: rsx! { MessageCircle { size: 24 } },
                    onclick: move |_| { }
                }
            }

            div { class: "mt-10 text-center",
                p { class: "text-[10px] text-muted-foreground font-bold uppercase tracking-widest", {i18n.tr("support_id")} }
                p { class: "text-xs font-mono text-foreground mt-1", "8A29-1B4C-9D0E" }
            }
        }
    }
}
