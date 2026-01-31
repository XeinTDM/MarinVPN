use crate::components::toast::{use_toast, ToastType};
use crate::icons::*;
use crate::state::ConnectionState;
use crate::Route;
use dioxus::prelude::*;

#[component]
pub fn Account() -> Element {
    let mut state = use_context::<ConnectionState>();
    let nav = use_navigator();
    let mut toast = use_toast();
    let i18n = crate::hooks::use_i18n();
    let account = (state.account_number)().unwrap_or_default();
    let mut show_account = use_signal(|| false);

    let expiry_text = use_memo(move || {
        if let Some(expiry) = (state.account_expiry)() {
            let dt = chrono::DateTime::from_timestamp(expiry, 0).unwrap_or_default();
            dt.format("%d %b %y, %H:%M").to_string()
        } else {
            "No active subscription".to_string()
        }
    });

    rsx! {
        div { class: "h-full w-full flex flex-col bg-background p-4",
            // Content Area
            div { class: "flex-1 overflow-y-auto custom-scrollbar",
                div { class: "space-y-6 pb-6",
                    // Device Name
                    div { class: "px-1",
                        h4 { class: "text-[10px] font-bold text-muted-foreground uppercase tracking-widest mb-2",
                            {i18n.tr("device_name")}
                        }
                        div { class: "flex items-center justify-between py-1",
                            span { class: "text-sm font-bold text-foreground capitalize",
                                "{state.device_name}"
                            }
                            button {
                                class: "text-[10px] font-bold text-primary hover:underline uppercase tracking-widest focus:outline-none",
                                onclick: move |_| {
                                    nav.push(Route::Devices {});
                                },
                                {i18n.tr("manage_devices")}
                            }
                        }
                    }

                    // Account Number
                    div { class: "px-1",
                        h4 { class: "text-[10px] font-bold text-muted-foreground uppercase tracking-widest mb-2",
                            {i18n.tr("account_number")}
                        }
                        div { class: "flex items-center justify-between py-1",
                            span { class: "text-sm font-bold text-foreground font-mono",
                                if show_account() {
                                    "{account}"
                                } else {
                                    "**** **** **** ****"
                                }
                            }
                            div { class: "flex items-center gap-1",
                                button {
                                    class: "w-12 flex items-center justify-center hover:bg-accent rounded-lg text-muted-foreground hover:text-foreground transition-all focus:outline-none",
                                    style: "height: 48px !important; min-height: 48px !important; flex-shrink: 0 !important;",
                                    onclick: move |_| show_account.set(!show_account()),
                                    Eye { size: 20 }
                                }
                                button {
                                    class: "w-12 flex items-center justify-center hover:bg-accent rounded-lg text-muted-foreground hover:text-foreground transition-all focus:outline-none",
                                    style: "height: 48px !important; min-height: 48px !important; flex-shrink: 0 !important;",
                                    onclick: move |_| {
                                        toast.show(i18n.tr("copy_account"), ToastType::Success);
                                    },
                                    Copy { size: 20 }
                                }
                            }
                        }
                    }

                    // Paid Until
                    div { class: "px-1",
                        h4 { class: "text-[10px] font-bold text-muted-foreground uppercase tracking-widest mb-2",
                            {i18n.tr("paid_until")}
                        }
                        div { class: "flex items-center justify-between py-1",
                            span { class: "text-sm font-bold text-foreground",
                                "{expiry_text}"
                            }
                        }
                    }
                }
            }

            // Bottom Buttons
            div { class: "px-1 py-4 space-y-3 border-t border-border/30",
                button {
                    class: "w-full flex items-center justify-center bg-primary hover:brightness-110 text-primary-foreground rounded-lg text-xs font-bold transition-all active:scale-[0.98] shadow-lg shadow-primary/20",
                    style: "height: 48px !important; min-height: 48px !important; flex-shrink: 0 !important;",
                    onclick: move |_| {
                        toast.show("Redirecting to shop...", ToastType::Info);
                    },
                    {i18n.tr("buy_credit")}
                }
                button {
                    class: "w-full flex items-center justify-center bg-card hover:bg-accent/40 border border-border text-foreground rounded-[8px] text-xs font-bold transition-all active:scale-[0.98] shadow-sm",
                    style: "height: 48px !important; min-height: 48px !important; flex-shrink: 0 !important;",
                    onclick: move |_| {
                        toast.show("Please enter voucher code", ToastType::Info);
                    },
                    {i18n.tr("redeem_voucher")}
                }
                button {
                    class: "w-full flex items-center justify-center bg-destructive/10 hover:bg-destructive/20 text-destructive rounded-lg border border-destructive/20 text-xs font-bold transition-all active:scale-[0.98]",
                    style: "height: 48px !important; min-height: 48px !important; flex-shrink: 0 !important;",
                    onclick: move |_| {
                        state.account_number.set(None);
                        state.auth_token.set(None);
                        state.account_expiry.set(None);
                        nav.replace(Route::Dashboard {});
                    },
                    {i18n.tr("log_out")}
                }
            }
        }
    }
}
