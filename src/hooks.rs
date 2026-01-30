use dioxus::prelude::*;
use std::time::Duration;
use tokio::time::sleep;
use crate::state::ConnectionState;
use crate::models::ConnectionStatus;
use crate::components::toast::{use_toast, ToastType};
use chrono::Utc;

#[derive(Clone, Copy)]
pub struct VpnClient {
    state: ConnectionState,
    toast: crate::components::toast::ToastManager,
}

impl VpnClient {
    pub fn connect(&self, location: String) {
        let mut state = self.state;
        // let mut toast = self.toast;
        
        spawn(async move {
            state.current_location.set(location.clone());
            state.status.set(ConnectionStatus::Connecting);
            
            sleep(Duration::from_millis(1500)).await;
            
            state.status.set(ConnectionStatus::Connected);
            state.connected_since.set(Some(Utc::now().timestamp() as f64));
        });
    }

    pub fn disconnect(&self) {
        let mut state = self.state;
        let mut toast = self.toast;
        spawn(async move {
            state.status.set(ConnectionStatus::Disconnecting);
            sleep(Duration::from_millis(800)).await;
            state.status.set(ConnectionStatus::Disconnected);
            state.connected_since.set(None);
            
            toast.show("Disconnected", ToastType::Info);
        });
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
    let toast = use_toast();
    VpnClient { state, toast }
}

pub fn use_scroll_handler(dns_expanded: Option<Signal<bool>>) {
    let mut state = use_context::<ConnectionState>();
    
    use_effect(move || {
        if let Some(target) = (state.scroll_to)() {
            spawn(async move {
                // Wait a bit for the layout to settle
                tokio::time::sleep(std::time::Duration::from_millis(150)).await;
                
                // If target is dns-blocking and we have the expansion signal, expand it first
                if target == "dns-blocking" {
                    if let Some(mut expanded) = dns_expanded {
                        expanded.set(true);
                        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                    }
                }

                let eval = document::eval(&format!(
                    r#"
                    const el = document.getElementById("{}");
                    if (el) {{
                        el.scrollIntoView({{ behavior: "smooth", block: "center" }});
                    }}
                    "#,
                    target
                ));
                let _ = eval;
                // Clear the scroll target
                state.scroll_to.set(None);
            });
        }
    });
}