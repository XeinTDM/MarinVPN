use crate::components::toast::ToastManager;
use crate::components::toast::ToastType;
use crate::components::ui::Modal;
use crate::hooks::use_i18n;
use crate::services::auth::AuthService;
use crate::state::ConnectionState;
use dioxus::prelude::*;

#[component]
pub fn Login() -> Element {
    let mut state = use_context::<ConnectionState>();
    let mut toasts = use_context::<ToastManager>();
    let i18n = use_i18n();
    let branding = state.settings.read();
    let branding_name = branding.branding_name.clone();
    let branding_logo = branding.branding_logo_path.clone();

    let mut account_input = use_signal(String::new);
    let mut is_loading = use_signal(|| false);
    let mut device_limit = use_signal(|| None as Option<Vec<crate::models::Device>>);
    let mut limit_error = use_signal(|| None as Option<String>);

    let on_login = move |_| {
        let acc_num = account_input().replace(" ", "").to_uppercase();
        if acc_num.len() < 16 {
            toasts.show("Invalid account number format", ToastType::Error);
            return;
        }

        spawn(async move {
            is_loading.set(true);
            match AuthService::login(&acc_num, None).await {
                Ok(resp) => {
                    if resp.success {
                        if let (Some(info), Some(device), Some(token), Some(refresh)) = (
                            resp.account_info,
                            resp.current_device,
                            resp.auth_token,
                            resp.refresh_token,
                        ) {
                            state.account_number.set(Some(info.account_number.clone()));
                            state.auth_token.set(Some(token));
                            state.refresh_token.set(Some(refresh));
                            state.account_expiry.set(Some(info.expiry_date));
                            state.device_name.set(device);
                            toasts.show("Logged in successfully", ToastType::Success);
                            navigator().push(crate::Route::Dashboard {});
                        } else {
                            toasts.show("Invalid login response", ToastType::Error);
                        }
                    } else if let Some(devs) = resp.devices {
                        device_limit.set(Some(devs));
                        limit_error.set(resp.error);
                    } else {
                        toasts.show(
                            &resp.error.unwrap_or_else(|| "Login failed".to_string()),
                            ToastType::Error,
                        );
                    }
                }
                Err(e) => toasts.show(&e.user_friendly_message(), ToastType::Error),
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
                Err(e) => toasts.show(&e.user_friendly_message(), ToastType::Error),
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
                    div { class: "inline-flex items-center justify-center w-16 h-16 rounded-2xl bg-primary/10 mb-4 overflow-hidden",
                        if branding_logo.is_empty() {
                            crate::icons::Shield { class: "w-8 h-8 text-primary", size: 32 }
                        } else {
                            img { src: "{branding_logo}", class: "h-10 w-10 object-contain" }
                        }
                    }
                    h1 { class: "text-3xl font-bold tracking-tight", "{branding_name}" }
                    p { class: "text-muted-foreground", {i18n.tr("login_subtitle")} }
                }

                div { class: "space-y-4",
                    div { class: "space-y-2",
                        label { class: "text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70",
                            {i18n.tr("account_number")}
                        }
                        input {
                            class: "flex h-12 w-full rounded-xl border border-input bg-background px-4 py-2 text-lg ring-offset-background file:border-0 file:bg-transparent file:text-sm file:font-medium placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50 transition-all",
                            placeholder: "ABCD EFGH JKLM NOPQ",
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

        if let Some(devices) = device_limit() {
            Modal {
                title: "Device limit reached".to_string(),
                onclose: move |_| {
                    device_limit.set(None);
                    limit_error.set(None);
                },
                children: rsx! {
                    div { class: "text-xs text-muted-foreground mb-3",
                        {limit_error().unwrap_or_else(|| "Remove a device to continue.".to_string())}
                    }
                    div { class: "space-y-2 max-h-56 overflow-y-auto pr-1",
                        for device in devices {
                            {
                                let dev_name = device.name.clone();
                                let acc = account_input();
                                rsx! {
                                    div { class: "flex items-center justify-between gap-2 border border-border rounded-xl px-3 py-2",
                                        div { class: "text-xs",
                                            div { class: "font-semibold capitalize text-foreground", "{dev_name}" }
                                            {
                                                let date_str = device.created_date.clone();
                                                rsx! { div { class: "text-[10px] text-muted-foreground", "Created {date_str}" } }
                                            }
                                        }
                                        button {
                                            class: "h-9 px-3 text-[10px] font-bold rounded-lg bg-destructive text-destructive-foreground hover:opacity-90 transition-all",
                                            onclick: move |_| {
                                                let acc_num = acc.replace(" ", "").to_uppercase();
                                                let dev = dev_name.clone();
                                                spawn(async move {
                                                    match AuthService::login(&acc_num, Some(dev)).await {
                                                        Ok(resp) => {
                                                            if resp.success {
                                                                if let (Some(info), Some(device), Some(token), Some(refresh)) = (
                                                                    resp.account_info,
                                                                    resp.current_device,
                                                                    resp.auth_token,
                                                                    resp.refresh_token,
                                                                ) {
                                                                    state.account_number.set(Some(info.account_number.clone()));
                                                                    state.auth_token.set(Some(token));
                                                                    state.refresh_token.set(Some(refresh));
                                                                    state.account_expiry.set(Some(info.expiry_date));
                                                                    state.device_name.set(device);
                                                                    toasts.show("Logged in successfully", ToastType::Success);
                                                                    device_limit.set(None);
                                                                    limit_error.set(None);
                                                                    navigator().push(crate::Route::Dashboard {});
                                                                } else {
                                                                    toasts.show("Invalid login response", ToastType::Error);
                                                                }
                                                            } else {
                                                                toasts.show(
                                                                    &resp.error.unwrap_or_else(|| "Login failed".to_string()),
                                                                    ToastType::Error,
                                                                );
                                                            }
                                                        }
                                                        Err(e) => toasts.show(&e.user_friendly_message(), ToastType::Error),
                                                    }
                                                });
                                            },
                                            "Kick & Continue"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
