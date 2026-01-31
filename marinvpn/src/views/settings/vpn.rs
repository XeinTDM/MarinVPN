use dioxus::prelude::*;
use crate::state::ConnectionState;
use crate::models::IpVersion;
use crate::components::*;

#[component]
pub fn VpnSettings(dns_expanded: Signal<bool>) -> Element {
    let mut state = use_context::<ConnectionState>();
    let nav = use_navigator();
    let s = state.settings.read();
    let i18n = crate::hooks::use_i18n();
    let mut show_local_sharing_info = use_signal(|| false);
    let mut show_dns_info = use_signal(|| false);
    let mut show_ipv6_info = use_signal(|| false);
    let mut show_kill_switch_info = use_signal(|| false);
    let mut show_lockdown_info = use_signal(|| false);
    let mut show_quantum_info = use_signal(|| false);
    let mut show_ip_version_info = use_signal(|| false);

    rsx! {
        div { class: "divide-y divide-border/30 -mx-4",
            if show_local_sharing_info() {
                InfoDialog {
                    title: i18n.tr("title_local_sharing").to_string(),
                    onclose: move |_| show_local_sharing_info.set(false),
                    content: rsx! {
                        p { class: "mb-3",
                            "This feature allows access to other devices on the local network, such as for sharing, printing, streaming, etc."
                        }
                        p { class: "mb-3",
                            "It does this by allowing network communication outside the tunnel to local multicast and broadcast ranges as well as to and from these private IP ranges:"
                        }
                        ul { class: "space-y-1 font-mono text-[10px] bg-accent/30 p-3 rounded-xl",
                            li { "· 10.0.0.0/8" }
                            li { "· 172.16.0.0/12" }
                            li { "· 192.168.0.0/16" }
                            li { "· 169.254.0.0/16" }
                            li { "· fe80::/10" }
                            li { "· fc00::/7" }
                        }
                    },
                }
            }

            if show_dns_info() {
                InfoDialog {
                    title: i18n.tr("title_dns_blocking").to_string(),
                    onclose: move |_| show_dns_info.set(false),
                    content: rsx! {
                        p { class: "mb-3",
                            "When this feature is enabled it stops the device from contacting certain domains or websites known for distributing ads, malware, trackers and more."
                        }
                        p { class: "mb-3", "This might cause issues on certain websites, services, and apps." }
                        p { class: "font-bold text-primary",
                            "Attention: this setting cannot be used in combination with Use custom DNS server"
                        }
                    },
                }
            }

            if show_ipv6_info() {
                InfoDialog {
                    title: i18n.tr("title_ipv6").to_string(),
                    onclose: move |_| show_ipv6_info.set(false),
                    content: rsx! {
                        p { class: "mb-3",
                            "When this feature is enabled, IPv6 can be used alongside IPv4 in the VPN tunnel to communicate with internet services."
                        }
                        p {
                            "IPv4 is always enabled and the majority of websites and applications use this protocol. We do not recommend enabling IPv6 unless you know you need it."
                        }
                    },
                }
            }

            if show_kill_switch_info() {
                InfoDialog {
                    title: i18n.tr("title_kill_switch").to_string(),
                    onclose: move |_| show_kill_switch_info.set(false),
                    content: rsx! {
                        p { class: "mb-3",
                            "This built-in feature prevents your traffic from leaking outside of the VPN tunnel if your network suddenly stops working or if the tunnel fails, it does this by blocking your traffic until your connection is reestablished."
                        }
                        p {
                            "The difference between the Kill Switch and Lockdown Mode is that the Kill Switch will prevent any leaks from happening during automatic tunnel reconnects, software crashes and similar accidents. With Lockdown Mode enabled, you must be connected to a Mullvad VPN server to be able to reach the internet. Manually disconnecting or quitting the app will block your connection."
                        }
                    },
                }
            }

            if show_lockdown_info() {
                InfoDialog {
                    title: i18n.tr("title_lockdown").to_string(),
                    onclose: move |_| show_lockdown_info.set(false),
                    content: rsx! {
                        p { class: "mb-3",
                            "The difference between the Kill Switch and Lockdown Mode is that the Kill Switch will prevent any leaks from happening during automatic tunnel reconnects, software crashes and similar accidents."
                        }
                        p {
                            "With Lockdown Mode enabled, you must be connected to a Mullvad VPN server to be able to reach the internet. Manually disconnecting or quitting the app will block your connection."
                        }
                    },
                }
            }

            if show_quantum_info() {
                InfoDialog {
                    title: i18n.tr("title_quantum").to_string(),
                    onclose: move |_| show_quantum_info.set(false),
                    content: rsx! {
                        p { class: "mb-3",
                            "This feature makes the WireGuard tunnel resistant to potential attacks from quantum computers."
                        }
                        p {
                            "It does this by performing an extra key exchange using a quantum safe algorithm and mixing the result into WireGuard's regular encryption. This extra step uses approximately 500 kiB of traffic every time a new tunnel is established."
                        }
                    },
                }
            }

            if show_ip_version_info() {
                InfoDialog {
                    title: i18n.tr("title_ip_version").to_string(),
                    onclose: move |_| show_ip_version_info.set(false),
                    content: rsx! {
                        p { class: "mb-3",
                            "This feature allows you to choose whether to use only IPv4, only IPv6, or allow the app to automatically decide the best option when connecting to a server."
                        }
                        p { "It can be useful when you are aware of problems caused by a certain IP version." }
                    },
                }
            }

            // Launch app on start-up
            SettingRow {
                label: i18n.tr("launch_startup").to_string(),
                checked: s.launch_on_startup,
                onclick: move |_| {
                    state.settings.with_mut(|s| s.launch_on_startup = !s.launch_on_startup);
                },
            }
            // Auto-connect
            div { class: "flex flex-col",
                SettingRow {
                    label: i18n.tr("auto_connect").to_string(),
                    checked: s.auto_connect,
                    onclick: move |_| {
                        state.settings.with_mut(|s| s.auto_connect = !s.auto_connect);
                    },
                }
                SettingDescription { text: i18n.tr("desc_auto_connect").to_string() }
                SettingGap { height: 20, class: Some("!border-t-0".to_string()) }
            }

            // Local network sharing
            div { class: "flex flex-col",
                SettingRow {
                    id: "local-sharing",
                    label: i18n.tr("local_sharing").to_string(),
                    show_info: true,
                    oninfo: move |_| show_local_sharing_info.set(true),
                    checked: s.local_sharing,
                    onclick: move |_| {
                        state.settings.with_mut(|s| s.local_sharing = !s.local_sharing);
                    },
                }
                SettingGap { height: 17, class: Some("!border-t-0".to_string()) }
            }

            // DNS content blockers
            div { class: "flex flex-col",
                SettingCollapsible {
                    id: "dns-blocking",
                    label: i18n.tr("dns_blocking").to_string(),
                    expanded: dns_expanded(),
                    show_info: true,
                    oninfo: move |_| show_dns_info.set(true),
                    onclick: move |_| dns_expanded.set(!dns_expanded()),
                }

                if dns_expanded() {
                    div { class: "bg-accent/5 divide-y divide-border/20",
                        SettingRow {
                            label: i18n.tr("ads").to_string(),
                            checked: s.dns_blocking.ads,
                            onclick: move |_| {
                                state.settings.with_mut(|s| s.dns_blocking.ads = !s.dns_blocking.ads);
                            },
                        }
                        SettingRow {
                            label: i18n.tr("trackers").to_string(),
                            checked: s.dns_blocking.trackers,
                            onclick: move |_| {
                                state.settings.with_mut(|s| s.dns_blocking.trackers = !s.dns_blocking.trackers);
                            },
                        }
                        SettingRow {
                            label: i18n.tr("malware").to_string(),
                            checked: s.dns_blocking.malware,
                            onclick: move |_| {
                                state.settings.with_mut(|s| s.dns_blocking.malware = !s.dns_blocking.malware);
                            },
                        }
                        SettingRow {
                            label: i18n.tr("gambling").to_string(),
                            checked: s.dns_blocking.gambling,
                            onclick: move |_| {
                                state.settings.with_mut(|s| s.dns_blocking.gambling = !s.dns_blocking.gambling);
                            },
                        }
                        SettingRow {
                            label: i18n.tr("adult_content").to_string(),
                            checked: s.dns_blocking.adult_content,
                            onclick: move |_| {
                                state
                                    .settings
                                    .with_mut(|s| s.dns_blocking.adult_content = !s.dns_blocking.adult_content);
                            },
                        }
                        SettingRow {
                            label: i18n.tr("social_media").to_string(),
                            checked: s.dns_blocking.social_media,
                            onclick: move |_| {
                                state
                                    .settings
                                    .with_mut(|s| s.dns_blocking.social_media = !s.dns_blocking.social_media);
                            },
                        }
                    }
                }

                SettingRow {
                    label: "Use custom DNS server".to_string(),
                    checked: s.custom_dns,
                    onclick: move |_| {
                        state.settings.with_mut(|s| s.custom_dns = !s.custom_dns);
                    },
                }
                if s.custom_dns {
                    div { class: "px-4 py-2",
                        input {
                            class: "w-full bg-accent/20 border border-border rounded-xl px-3 py-2 text-xs font-mono focus:outline-none focus:ring-2 focus:ring-primary/20 transition-all",
                            value: "{s.custom_dns_server}",
                            placeholder: "e.g. 1.1.1.1",
                            oninput: move |e| {
                                let val = e.value();
                                state.settings.with_mut(|s| s.custom_dns_server = val);
                            }
                        }
                    }
                }
                SettingDescription { text: i18n.tr("desc_dns_blocking").to_string() }
                SettingGap { height: 20, class: Some("!border-t-0".to_string()) }
            }

            // In-tunnel IPv6
            div { class: "flex flex-col",
                SettingRow {
                    id: "ipv6-support",
                    label: i18n.tr("ipv6_support").to_string(),
                    show_info: true,
                    oninfo: move |_| show_ipv6_info.set(true),
                    checked: s.ipv6_support,
                    onclick: move |_| {
                        state.settings.with_mut(|s| s.ipv6_support = !s.ipv6_support);
                    },
                }
                SettingDescription { text: i18n.tr("desc_ipv6").to_string() }
                SettingGap { height: 20, class: Some("!border-t-0".to_string()) }
            }

            // Kill switch & Lockdown mode
            div { class: "flex flex-col",
                SettingRow {
                    id: "kill-switch",
                    label: i18n.tr("kill_switch").to_string(),
                    show_info: true,
                    oninfo: move |_| show_kill_switch_info.set(true),
                    checked: s.kill_switch,
                    onclick: move |_| {
                        state.settings.with_mut(|s| s.kill_switch = !s.kill_switch);
                    },
                }
                SettingRow {
                    id: "lockdown-mode",
                    label: i18n.tr("lockdown_mode").to_string(),
                    show_info: true,
                    oninfo: move |_| show_lockdown_info.set(true),
                    checked: s.lockdown_mode,
                    onclick: move |_| {
                        state.settings.with_mut(|s| s.lockdown_mode = !s.lockdown_mode);
                    },
                }
                SettingRow {
                    label: "Traffic Padding (DAITA)".to_string(),
                    checked: s.daita_enabled,
                    onclick: move |_| {
                        state.settings.with_mut(|s| s.daita_enabled = !s.daita_enabled);
                    },
                }
                SettingGap { height: 17, class: Some("!border-t-0".to_string()) }
            }

            // Anti-censorship
            div { class: "flex flex-col",
                SettingAction {
                    label: i18n.tr("anti_censorship").to_string(),
                    value: Some(i18n.tr("auto").to_string()),
                    onclick: move |_| {
                        nav.push(crate::Route::AntiCensorshipSettings {});
                    },
                }
                SettingDescription { text: "Hides the fact that you are using a VPN by mimicking regular internet traffic.".to_string() }
                SettingGap { height: 17, class: Some("!border-t-0".to_string()) }
            }

            // Quantum-resistant tunnel
            div { class: "flex flex-col",
                SettingRow {
                    id: "quantum-resistant",
                    label: i18n.tr("quantum_resistant").to_string(),
                    show_info: true,
                    oninfo: move |_| show_quantum_info.set(true),
                    checked: s.quantum_resistant,
                    onclick: move |_| {
                        state.settings.with_mut(|s| s.quantum_resistant = !s.quantum_resistant);
                    },
                }
                SettingGap { height: 17, class: Some("!border-t-0".to_string()) }
            }

            // Device IP version
            div { class: "flex flex-col",
                SettingTitle {
                    label: i18n.tr("ip_version").to_string(),
                    show_info: true,
                    oninfo: move |_| show_ip_version_info.set(true),
                }
                SettingSelectRow {
                    label: i18n.tr("automatic").to_string(),
                    selected: s.ip_version == IpVersion::Automatic,
                    onclick: move |_| {
                        state.settings.with_mut(|s| s.ip_version = IpVersion::Automatic);
                    },
                }
                SettingSelectRow {
                    label: "IPv4".to_string(),
                    selected: s.ip_version == IpVersion::Ipv4,
                    onclick: move |_| {
                        state.settings.with_mut(|s| s.ip_version = IpVersion::Ipv4);
                    },
                }
                SettingSelectRow {
                    label: "IPv6".to_string(),
                    selected: s.ip_version == IpVersion::Ipv6,
                    onclick: move |_| {
                        state.settings.with_mut(|s| s.ip_version = IpVersion::Ipv6);
                    },
                }
                SettingGap { height: 17, class: Some("!border-t-0".to_string()) }
            }

            // MTU
            div { class: "flex flex-col",
                SettingInput {
                    label: i18n.tr("mtu").to_string(),
                    value: s.mtu.to_string(),
                    oninput: move |e: Event<FormData>| {
                        if let Ok(val) = e.value().parse::<u32>() {
                            state.settings.with_mut(|s| s.mtu = val);
                        }
                    },
                }
                SettingDescription { text: i18n.tr("desc_mtu").to_string() }
                SettingGap { height: 20, class: Some("!border-t-0".to_string()) }
            }

            // Server IP override
            SettingAction {
                label: i18n.tr("server_override").to_string(),
                onclick: move |_| {
                    nav.push(crate::Route::ServerOverrideSettings {});
                },
            }
        }
    }
}