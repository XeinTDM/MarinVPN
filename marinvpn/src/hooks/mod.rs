pub mod use_account;
pub mod use_connection;
pub mod use_servers;
pub mod tests;

use crate::models::{ConnectionStatus, VpnAction};
use crate::state::ConnectionState;
use dioxus::prelude::*;

#[derive(Clone, Copy)]
pub struct VpnClient {
    state: ConnectionState,
}

impl VpnClient {
    pub fn connect(&self, location: String) {
        self.state.vpn_action.send(VpnAction::Connect(location));
    }

    pub fn disconnect(&self) {
        self.state.vpn_action.send(VpnAction::Disconnect);
    }

    pub fn toggle(&self) {
        let state = self.state;
        match (state.status)() {
            ConnectionStatus::Disconnected => {
                let loc = (state.current_location)();
                self.connect(loc);
            }
            ConnectionStatus::Connected => {
                self.disconnect();
            }
            _ => {}
        }
    }

    pub fn toggle_favorite(&self, location: String) {
        let mut favorites = self.state.favorites;
        let mut current = favorites.peek().clone();
        if current.contains(&location) {
            current.remove(&location);
        } else {
            current.insert(location);
        }
        favorites.set(current);
    }
}

pub fn use_vpn_client() -> VpnClient {
    let state = use_context::<ConnectionState>();
    VpnClient { state }
}

#[derive(Clone, Copy)]
pub struct I18n {
    lang: crate::models::Language,
}

impl I18n {
    pub fn tr(&self, key: &str) -> &'static str {
        crate::i18n::translate(key, self.lang)
    }
}

pub fn use_i18n() -> I18n {
    let state = use_context::<ConnectionState>();
    let lang = (state.settings)().language;
    I18n { lang }
}

pub fn use_scroll_handler(dns_expanded: Option<Signal<bool>>) {
    let mut state = use_context::<ConnectionState>();

    use_effect(move || {
        if let Some(target) = (state.scroll_to)() {
            spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(150)).await;

                if target == "dns-blocking" {
                    if let Some(mut expanded) = dns_expanded {
                        expanded.set(true);
                        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                    }
                }

                let _ = document::eval(&format!(
                    r#"
                    const el = document.getElementById("{}");
                    if (el) {{
                        el.scrollIntoView({{ behavior: "smooth", block: "center" }});
                    }}
                    "#,
                    target
                ));

                state.scroll_to.set(None);
            });
        }
    });
}
