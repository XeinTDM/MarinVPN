use dioxus::prelude::*;
use crate::hooks::use_i18n;
use crate::services::auth::AuthService;
use crate::state::ConnectionState;
use crate::components::toast::ToastType;
use crate::components::toast::ToastManager;

#[component]
pub fn Login() -> Element {
    let mut state = use_context::<ConnectionState>();
    let mut toasts = use_context::<ToastManager>();
    let i18n = use_i18n();
    
    let mut account_input = use_signal(|| String::new());
    let mut is_loading = use_signal(|| false);

    let on_login = move |_| {
        let acc_num = account_input().replace(" ", "");
        if acc_num.len() < 16 {
            toasts.show("Invalid account number format", ToastType::Error);
            return;
        }

        let dev_name = state.device_name.peek().clone();

        spawn(async move {
            is_loading.set(true);
            match AuthService::login(&acc_num, Some(dev_name)).await {
                Ok((info, device, token)) => {
                    state.account_number.set(Some(info.account_number.clone()));
                    state.auth_token.set(Some(token));
                    state.account_expiry.set(Some(info.expiry_date));
                    state.device_name.set(device);
                    toasts.show("Logged in successfully", ToastType::Success);
                    navigator().push(crate::Route::Dashboard {});
                }
                Err(e) => toasts.show(&e, ToastType::Error),
            }
            is_loading.set(false);
        });
    };

    let on_generate = move |_| {
        spawn(async move {
            is_loading.set(true);
            match AuthService::generate_account_number().await {
                Ok(num) => {
                    account_input.set(num);
                    toasts.show("New account generated", ToastType::Info);
                }
                Err(e) => toasts.show(&e, ToastType::Error),
            }
            is_loading.set(false);
        });
    };

    rsx! {
        div { class: "flex-1 flex flex-col items-center justify-center p-8 bg-background relative overflow-hidden",
            div { class: "absolute -top-24 -right-24 w-64 h-64 bg-primary/10 rounded-full blur-3xl" }
            div { class: "absolute -bottom-24 -left-24 w-64 h-64 bg-primary/5 rounded-full blur-3xl" }

            div { class: "w-full max-w-sm space-y-8 z-10",
                div { class: "text-center space-y-2",
                    div { class: "inline-flex items-center justify-center w-16 h-16 rounded-2xl bg-primary/10 mb-4",
                        crate::icons::Shield { class: "w-8 h-8 text-primary", size: 32 }
                    }
                    h1 { class: "text-3xl font-bold tracking-tight", "MarinVPN" }
                    p { class: "text-muted-foreground", {i18n.tr("login_subtitle")} }
                }

                div { class: "space-y-4",
                    div { class: "space-y-2",
                        label { class: "text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70",
                            {i18n.tr("account_number")}
                        }
                        input {
                            class: "flex h-12 w-full rounded-xl border border-input bg-background px-4 py-2 text-lg ring-offset-background file:border-0 file:bg-transparent file:text-sm file:font-medium placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50 transition-all",
                            placeholder: "0000 0000 0000 0000",
                            value: "{account_input}",
                            oninput: move |e| account_input.set(e.value()),
                            disabled: is_loading(),
                        }
                    }

                    button {
                        class: "inline-flex items-center justify-center rounded-xl text-sm font-medium ring-offset-background transition-all focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50 bg-primary text-primary-foreground hover:bg-primary/90 h-12 px-4 py-2 w-full text-base",
                        onclick: on_login,
                        disabled: is_loading() || account_input().is_empty(),
                        if is_loading() {
                            div { class: "mr-2 h-4 w-4 animate-spin rounded-full border-2 border-current border-t-transparent" }
                        }
                        {i18n.tr("login_button")}
                    }

                    div { class: "relative py-4",
                        div { class: "absolute inset-0 flex items-center",
                            span { class: "w-full border-t border-muted" }
                        }
                        div { class: "relative flex justify-center text-xs uppercase",
                            span { class: "bg-background px-2 text-muted-foreground font-medium", {i18n.tr("or_text")} }
                        }
                    }

                    button {
                        class: "inline-flex items-center justify-center rounded-xl text-sm font-medium ring-offset-background transition-all focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50 border border-input bg-background hover:bg-accent hover:text-accent-foreground h-12 px-4 py-2 w-full text-base",
                        onclick: on_generate,
                        disabled: is_loading(),
                        {i18n.tr("generate_account")}
                    }
                }

                p { class: "px-8 text-center text-sm text-muted-foreground leading-relaxed",
                    {i18n.tr("login_footer")}
                }
            }
        }
    }
}
