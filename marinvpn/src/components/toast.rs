use crate::icons::*;
use dioxus::prelude::*;

#[derive(Clone, Copy, PartialEq)]
pub enum ToastType {
    Info,
    Success,
    Error,
}

#[derive(Clone, PartialEq)]
pub struct Toast {
    pub id: usize,
    pub message: String,
    pub type_: ToastType,
    pub is_closing: bool,
}

#[derive(Clone, Copy)]
pub struct ToastManager {
    toasts: Signal<Vec<Toast>>,
    next_id: Signal<usize>,
}

impl ToastManager {
    pub fn show(&mut self, message: &str, type_: ToastType) {
        let mut id_write = self.next_id.write();
        let id = *id_write;
        *id_write += 1;
        drop(id_write);

        let toast = Toast {
            id,
            message: message.to_string(),
            type_,
            is_closing: false,
        };

        self.toasts.write().push(toast);

        let mut toasts = self.toasts;
        spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(3000)).await;

            toasts.with_mut(|t| {
                if let Some(toast) = t.iter_mut().find(|t| t.id == id) {
                    toast.is_closing = true;
                }
            });

            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
            toasts.write().retain(|t| t.id != id);
        });
    }
}

pub fn use_toast() -> ToastManager {
    use_context::<ToastManager>()
}

#[component]
pub fn ToastProvider(children: Element) -> Element {
    let toasts = use_signal(Vec::new);
    let next_id = use_signal(|| 0);

    use_context_provider(|| ToastManager { toasts, next_id });

    rsx! {
        div { class: "contents",
            {children}

            div { class: "absolute bottom-24 left-0 right-0 flex flex-col items-center gap-2 pointer-events-none z-[100]",
                for toast in toasts() {
                    div {
                        key: "{toast.id}",
                        class: "pointer-events-auto bg-card border border-border text-foreground px-4 py-3 rounded-2xl shadow-xl flex items-center gap-3 transition-all duration-300",
                        class: if toast.is_closing { "opacity-0 translate-y-2 scale-95" } else { "animate-in slide-in-from-bottom-2 fade-in" },
                        match toast.type_ {
                            ToastType::Info => rsx! {
                                Info { size: 18, class: Some("text-status-info".to_string()) }
                            },
                            ToastType::Success => rsx! {
                                CircleCheck { size: 18, class: Some("text-status-success".to_string()) }
                            },
                            ToastType::Error => rsx! {
                                CircleAlert { size: 18, class: Some("text-status-error".to_string()) }
                            },
                        }
                        span { class: "text-sm font-medium", "{toast.message}" }
                    }
                }
            }
        }
    }
}
