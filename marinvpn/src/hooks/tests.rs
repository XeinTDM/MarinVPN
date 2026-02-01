#[cfg(test)]
mod tests {
    use crate::error::AppError;
    use crate::hooks::use_account::AccountState;
    use crate::hooks::use_connection::use_connection_with_service;
    use crate::models::{
        CommonVpnServer, ConnectionStatus, SettingsState, VpnAction, WireGuardConfig,
    };
    use crate::services::vpn::{VpnEvent, VpnService};
    use crate::services::AppService;
    use async_trait::async_trait;
    use dioxus::prelude::*;
    use marinvpn_common::DnsBlockingState;
    use std::sync::{Arc, Mutex};
    use tokio::sync::broadcast;

    #[derive(Clone)]
    struct MockVpnService {
        tx: broadcast::Sender<VpnEvent>,
        connected: Arc<Mutex<bool>>,
        connect_calls: Arc<Mutex<usize>>,
    }

    impl PartialEq for MockVpnService {
        fn eq(&self, other: &Self) -> bool {
            Arc::ptr_eq(&self.connected, &other.connected)
        }
    }

    impl MockVpnService {
        fn new() -> Self {
            let (tx, _) = broadcast::channel(10);
            Self {
                tx,
                connected: Arc::new(Mutex::new(false)),
                connect_calls: Arc::new(Mutex::new(0)),
            }
        }
    }

    #[async_trait]
    impl VpnService for MockVpnService {
        fn subscribe(&self) -> broadcast::Receiver<VpnEvent> {
            self.tx.subscribe()
        }
        async fn connect(
            &self,
            _entry: String,
            _entry_config: WireGuardConfig,
            _exit: Option<(String, WireGuardConfig)>,
            _settings: SettingsState,
            _auth: Option<(String, String)>,
        ) {
            {
                let mut calls = self.connect_calls.lock().unwrap();
                *calls += 1;
                let mut lock = self.connected.lock().unwrap();
                *lock = true;
            }
            let _ = self.tx.send(VpnEvent::StatusChanged(ConnectionStatus::Connected));
        }
        async fn disconnect(&self) {
            {
                let mut lock = self.connected.lock().unwrap();
                *lock = false;
            }
            let _ = self.tx.send(VpnEvent::StatusChanged(ConnectionStatus::Disconnected));
        }
        async fn get_status(&self) -> ConnectionStatus {
            let lock = self.connected.lock().unwrap();
            if *lock {
                ConnectionStatus::Connected
            } else {
                ConnectionStatus::Disconnected
            }
        }
        async fn enable_captive_portal(&self, _duration_secs: u64) {}
        async fn apply_lockdown(&self, _settings: &SettingsState) -> Result<(), crate::services::vpn::VpnError> {
            Ok(())
        }
        async fn disable_kill_switch(&self) {}
    }

    #[derive(Clone, PartialEq)]
    struct MockAppService;

    #[async_trait]
    impl AppService for MockAppService {
        async fn find_best_server(&self, _country: Option<&str>) -> Result<CommonVpnServer, AppError> {
            Ok(CommonVpnServer {
                country: "Sweden".to_string(),
                city: "Stockholm".to_string(),
                endpoint: "1.2.3.4:51820".to_string(),
                public_key: "abc".to_string(),
                current_load: 10,
                avg_latency: 20,
            })
        }
        async fn find_best_server_excluding(
            &self,
            _country: Option<&str>,
            _exclude: &[String],
        ) -> Result<CommonVpnServer, AppError> {
            Err(AppError::Vpn("Not implemented".to_string()))
        }
        async fn get_anonymous_config(
            &self,
            _location: &str,
            _token: &str,
            _dns: Option<DnsBlockingState>,
            _qr: bool,
        ) -> Result<WireGuardConfig, AppError> {
            Ok(WireGuardConfig {
                private_key: "priv".to_string(),
                public_key: "pub".to_string(),
                endpoint: "1.2.3.4:51820".to_string(),
                allowed_ips: "0.0.0.0/0".to_string(),
                address: "10.0.0.2/32".to_string(),
                dns: None,
                preshared_key: None,
                obfuscation_key: None,
                pqc_ciphertext: None,
                pqc_handshake: None,
                pqc_provider: None,
            })
        }
        async fn get_servers(&self) -> Result<Vec<CommonVpnServer>, AppError> {
            Ok(vec![])
        }
        async fn measure_latency(&self, _endpoint: &str) -> Option<u32> {
            Some(50)
        }
    }

    #[test]
    fn test_initial_state() {
        fn app() -> Element {
            let settings = use_signal(SettingsState::default);
            let account = AccountState {
                account_number: use_signal(|| Some("1234".to_string())),
                auth_token: use_signal(|| Some("token".to_string())),
                refresh_token: use_signal(|| None),
                account_expiry: use_signal(|| None),
                device_name: use_signal(|| "test-device".to_string()),
            };

            let vpn_service = MockVpnService::new();
            let app_service = MockAppService;

            rsx! {
                crate::components::toast::ToastProvider {
                    TestComponent {
                        account,
                        settings,
                        vpn_service,
                        app_service
                    }
                }
            }
        }

        #[component]
        fn TestComponent(
            account: AccountState, 
            settings: Signal<SettingsState>, 
            vpn_service: MockVpnService, 
            app_service: MockAppService
        ) -> Element {
            let vpn_state = use_connection_with_service(account, settings, vpn_service, app_service);
            rsx! {
                div {
                    "{(vpn_state.status)():?}"
                }
            }
        }

        let mut dom = VirtualDom::new(app);
        dom.rebuild_in_place();
    }

    #[tokio::test]
    async fn test_connect_flow() {
        let vpn_service = MockVpnService::new();
        let app_service = MockAppService;
        let vpn_service_clone = vpn_service.clone();
        let app_service_clone = app_service.clone();

        #[component]
        fn App(vpn_service: MockVpnService, app_service: MockAppService) -> Element {
            let settings = use_signal(SettingsState::default);
            let account = AccountState {
                account_number: use_signal(|| Some("1234".to_string())),
                auth_token: use_signal(|| Some("token".to_string())),
                refresh_token: use_signal(|| None),
                account_expiry: use_signal(|| None),
                device_name: use_signal(|| "test-device".to_string()),
            };

            rsx! {
                crate::components::toast::ToastProvider {
                    TestConnectComponent {
                        account,
                        settings,
                        vpn_service,
                        app_service
                    }
                }
            }
        }

        #[component]
        fn TestConnectComponent(
            account: AccountState,
            settings: Signal<SettingsState>,
            vpn_service: MockVpnService,
            app_service: MockAppService
        ) -> Element {
            let vpn_state = use_connection_with_service(account, settings, vpn_service, app_service);
            
            use_effect(move || {
                vpn_state.vpn_action.send(VpnAction::Connect("Automatic".to_string()));
            });

            rsx! {
                div {
                    id: "status",
                    "{(vpn_state.status)():?}"
                }
            }
        }

        let mut dom = VirtualDom::new_with_props(App, AppProps { vpn_service: vpn_service_clone, app_service: app_service_clone });
        
        dom.rebuild_in_place(); 
        dom.wait_for_work().await; 
        
        let mut called = false;
        for _ in 0..40 {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            dom.wait_for_work().await;
            if *vpn_service.connect_calls.lock().unwrap() > 0 {
                called = true;
                break;
            }
        }
        
        assert!(called, "Expected VpnService::connect to be called");
    }
}