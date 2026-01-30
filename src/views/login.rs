use dioxus::prelude::*;
use dioxus::desktop::use_window;
use rand::Rng;
use crate::icons::*;
use crate::state::ConnectionState;
use crate::Route;

#[component]
pub fn Login() -> Element {
    let mut state = use_context::<ConnectionState>();
    let nav = use_navigator();
    let window = use_window();
    let mut input_value = use_signal(|| String::new());
    let mut error_msg = use_signal(|| Option::<String>::None);

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

    let mut handle_login = move || {
        let val = input_value();
        let digits: String = val.chars().filter(|c| c.is_digit(10)).collect();
        if digits.len() < 16 {
             error_msg.set(Some("Invalid account number (16 digits required)".to_string()));
        } else {
             state.account_number.set(Some(val));
             
             // Generate random device name
             let adjectives = ["cold", "warm", "fast", "brave", "silent", "gentle", "wild", "smart"];
             let nouns = ["chicken", "eagle", "tiger", "river", "mountain", "forest", "breeze", "storm"];
             let mut rng = rand::thread_rng();
             let adj = adjectives[rng.gen_range(0..adjectives.len())];
             let noun = nouns[rng.gen_range(0..nouns.len())];
             state.device_name.set(format!("{} {}", adj, noun));

             nav.replace(Route::Dashboard {});
        }
    };

    let generate_account = move |_| {
        let mut rng = rand::thread_rng();
        let p1: u16 = rng.gen_range(1000..9999);
        let p2: u16 = rng.gen_range(1000..9999);
        let p3: u16 = rng.gen_range(1000..9999);
        let p4: u16 = rng.gen_range(1000..9999);
        input_value.set(format!("{} {} {} {}", p1, p2, p3, p4));
        error_msg.set(None);
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
                        p { class: "text-muted-foreground mt-2 font-medium", "Secure & Private" }
                    }

                    div { class: "space-y-4",
                        div {
                            input {
                                class: "w-full bg-card border border-border focus:border-primary focus:ring-4 focus:ring-primary/10 rounded-2xl px-4 py-4 text-center font-mono text-xl tracking-widest outline-none transition-all placeholder-muted-foreground shadow-sm",
                                placeholder: "0000 0000 0000 0000",
                                value: "{input_value}",
                                oninput: move |e| {
                                    let formatted = format_account_number(e.value());
                                    input_value.set(formatted);
                                    error_msg.set(None);
                                },
                                onkeydown: move |e| {
                                    if e.key() == Key::Enter {
                                        handle_login();
                                    }
                                }
                            }
                            if let Some(msg) = error_msg() {
                                div { class: "text-destructive text-[10px] font-bold text-center mt-2 uppercase tracking-wider", "{msg}" }
                            }
                        }

                        button {
                            class: "w-full bg-primary hover:brightness-110 text-primary-foreground font-bold py-4 rounded-2xl shadow-xl shadow-primary/20 transition-all active:scale-95 flex items-center justify-center gap-2",
                            onclick: move |_| handle_login(),
                            "Log In"
                            ArrowRight { size: 18 }
                        }
                    }

                    div { class: "mt-8 text-center",
                        button {
                             class: "text-xs text-muted-foreground hover:text-primary transition-colors flex items-center justify-center gap-2 w-full font-medium",
                             onclick: generate_account,
                             RefreshCw { size: 12 }
                             "Generate account number"
                        }
                    }
                 }
            }
        }
    }
}
