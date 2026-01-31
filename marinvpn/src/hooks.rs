use dioxus::prelude::*;
use crate::state::ConnectionState;
use crate::state::VpnAction;
use crate::models::ConnectionStatus;

/// A client hook for interacting with the VPN service.
/// 
/// This struct wraps the global state actions into a convenient API.
#[derive(Clone, Copy)]
pub struct VpnClient {
    state: ConnectionState,
}

impl VpnClient {
    /// Initiates a connection to the specified location.
    pub fn connect(&self, location: String) {
        self.state.vpn_action.send(VpnAction::Connect(location));
    }

    /// Disconnects the current VPN session.
    pub fn disconnect(&self) {
        self.state.vpn_action.send(VpnAction::Disconnect);
    }

    /// Toggles the VPN connection state.
    /// 
    /// If disconnected, it connects to the current location.
    /// If connected or connecting, it disconnects.
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

    /// Toggles a location as a favorite.
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

/// Handles scrolling to specific elements when triggered by global state.
/// 
/// Takes an optional signal to expand DNS blocking settings if that's the target.
/// Note: This uses `document::eval` and assumes a browser-like environment (Desktop/Web).
pub fn use_scroll_handler(dns_expanded: Option<Signal<bool>>) {
    let mut state = use_context::<ConnectionState>();
    
    use_effect(move || {
        if let Some(target) = (state.scroll_to)() {
            // Clear the scroll target immediately to prevent double-firing
            // We do this via a small task to allow the effect to complete first if needed, 
            // but setting it to None inside the effect that listens to it is generally safe in Dioxus 0.7 
            // as long as we don't create an infinite loop (which the Some check prevents).
            
            spawn(async move {
                // Wait for layout/navigation transitions to complete.
                // This 150ms delay is a heuristic to ensure the DOM element exists after page navigation.
                tokio::time::sleep(std::time::Duration::from_millis(150)).await;
                
                // If target is dns-blocking and we have the expansion signal, expand it first
                if target == "dns-blocking" {
                    if let Some(mut expanded) = dns_expanded {
                        expanded.set(true);
                        // Small delay to allow the accordion animation/rendering to start
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
                
                // Reset state
                state.scroll_to.set(None);
            });
        }
    });
}