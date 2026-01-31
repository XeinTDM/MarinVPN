use dioxus::prelude::*;
use dioxus::desktop::use_window;
use crate::icons::*;
use crate::state::ConnectionState;
use crate::services::auth::AuthService;
use crate::Route;

#[component]
pub fn Login() -> Element {
    let mut state = use_context::<ConnectionState>();
    let mut toasts = crate::components::toast::use_toast();
    let nav = use_navigator();
    let window = use_window();
    let i18n = crate::hooks::use_i18n();
    let mut input_value = use_signal(|| String::new());
    let mut error_msg = use_signal(|| Option::<String>::None);
    let mut is_loading = use_signal(|| false);

    let format_account_number = |val: String| -> String {
        let digits: String = val.chars().filter(|c| c.is_digit(10)).take(16).collect();
        let mut formatted = String::new();
        for (i, c) in digits.chars().enumerate() {
            if i > 0 && i % 4 == 0 {
                formatted.push(' ');
            }
            formatted.push(c);
        }
        formatted
    };

    let mut handle_login = move |_| {
        let val = input_value();
        is_loading.set(true);
        error_msg.set(None);
        
        spawn(async move {
            match AuthService::login(&val, None).await {
                Ok((info, current_device, token)) => {
                    state.account_number.set(Some(info.account_number));
                    state.auth_token.set(Some(token));
                    state.account_expiry.set(Some(info.expiry_date));
                    state.device_name.set(current_device);
                    toasts.show("Welcome back!", crate::components::toast::ToastType::Success);
                    nav.replace(Route::Dashboard {});
                }
                Err(e) => {
                    toasts.show(&e, crate::components::toast::ToastType::Error);
                    error_msg.set(Some(e));
                    is_loading.set(false);
                }
            }
        });
    };

    let generate_account = move |_| {
        spawn(async move {
            match AuthService::generate_account_number().await {
                Ok(new_account) => {
                    input_value.set(new_account);
                    error_msg.set(None);
                    toasts.show("New account generated!", crate::components::toast::ToastType::Success);
                }
                Err(e) => {
                    toasts.show(&e, crate::components::toast::ToastType::Error);
                    error_msg.set(Some(e));
                }
            }
        });
    };

    rsx! {
        div { class: "flex-1 bg-background text-foreground flex flex-col overflow-hidden",
            div { class: "drag-region flex justify-end p-2",
                button {
                    class: "no-drag p-2 hover:bg-destructive/20 hover:text-destructive rounded-xl text-muted-foreground transition-all",
                    onclick: move |_| window.close(),
                    X { size: 18 }
                }
            }

            div { class: "flex-1 flex flex-col items-center justify-center p-8 -mt-4",
                 div { class: "w-full max-w-xs",
                    div { class: "flex flex-col items-center mb-10",
                        div { 
                            class: "w-20 h-20 bg-primary rounded-[2rem] flex items-center justify-center text-primary-foreground shadow-2xl shadow-primary/20 mb-6 rotate-3 cursor-pointer hover:scale-105 transition-transform active:rotate-12", 
                            onclick: generate_account,
                            ShieldCheck { size: 40, stroke_width: 2 }
                        }
                        h1 { class: "text-3xl font-bold tracking-tight", "MarinVPN" }
                        p { class: "text-muted-foreground mt-2 font-medium", {i18n.tr("secure_private")} }
                    }

                    div { class: "space-y-4",
                        div {
                            input {
                                class: "w-full bg-card border border-border focus:border-primary focus:ring-4 focus:ring-primary/10 rounded-2xl px-4 py-4 text-center font-mono text-xl tracking-widest outline-none transition-all placeholder-muted-foreground shadow-sm",
                                placeholder: "0000 0000 0000 0000",
                                value: "{input_value}",
                                disabled: is_loading(),
                                oninput: move |e| {
                                    let formatted = format_account_number(e.value());
                                    input_value.set(formatted);
                                    error_msg.set(None);
                                },
                                onkeydown: move |e| {
                                    if e.key() == Key::Enter && !is_loading() {
                                        handle_login(());
                                    }
                                }
                            }
                            if let Some(msg) = error_msg() {
                                div { class: "text-destructive text-[10px] font-bold text-center mt-2 uppercase tracking-wider", "{msg}" }
                            }
                        }

                        button {
                            class: "w-full bg-primary hover:brightness-110 text-primary-foreground font-bold py-4 rounded-2xl shadow-xl shadow-primary/20 transition-all active:scale-95 flex items-center justify-center gap-2 disabled:opacity-50 disabled:cursor-not-allowed",
                            onclick: move |_| handle_login(()),
                            disabled: is_loading(),
                            if is_loading() {
                                RefreshCw { class: "animate-spin", size: 18 }
                                "Authenticating..."
                            } else {
                                {i18n.tr("login")}
                                ArrowRight { size: 18 }
                            }
                        }
                    }

                    div { class: "mt-8 text-center",
                        button {
                             class: "text-xs text-muted-foreground hover:text-primary transition-colors flex items-center justify-center gap-2 w-full font-medium disabled:opacity-50",
                             onclick: generate_account,
                             disabled: is_loading(),
                             RefreshCw { size: 12 }
                             {i18n.tr("generate_account")}
                        }
                    }
                 }
            }
        }
    }
}

