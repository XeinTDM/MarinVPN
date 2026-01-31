use dioxus::prelude::*;
use crate::models::{Region, ConnectionStatus};
use crate::hooks::use_vpn_client;
use crate::state::ConnectionState;

const MAP_WIDTH: f64 = 800.0;
const MAP_HEIGHT: f64 = 600.0;
const ZOOM_LEVEL: f64 = 2.5;
const ANIMATION_SPEED: f64 = 0.1;
const COLOR_CONNECTED: &str = "oklch(0.696 0.17 162.48)";
const COLOR_CONNECTING: &str = "oklch(0.769 0.188 70.08)";

#[component]
pub fn DashboardMap(regions: Vec<Region>, country: String, status: ConnectionStatus) -> Element {
    let vpn = use_vpn_client();
    let state = use_context::<ConnectionState>();
    let settings = state.settings.read();

    let active_region = regions.iter().find(|r| r.name == country);

    let target_view = if let Some(region) = active_region {
        let w = MAP_WIDTH / ZOOM_LEVEL;
        let h = MAP_HEIGHT / ZOOM_LEVEL;
        let x = (region.map_x - w / 2.0).clamp(0.0, MAP_WIDTH - w);
        let y = (region.map_y - h / 2.0).clamp(0.0, MAP_HEIGHT - h);
        (x, y, w, h)
    } else {
        (0.0, 0.0, MAP_WIDTH, MAP_HEIGHT)
    };

    let mut current_view = use_signal(|| target_view);

    use_effect(move || {
        let target = target_view;
        spawn(async move {
            let mut steps = 0;
            loop {
                let (cx, cy, cw, ch) = current_view.peek().clone();
                let (tx, ty, tw, th) = target;
                
                let nx = cx + (tx - cx) * ANIMATION_SPEED;
                let ny = cy + (ty - cy) * ANIMATION_SPEED;
                let nw = cw + (tw - cw) * ANIMATION_SPEED;
                let nh = ch + (th - ch) * ANIMATION_SPEED;

                if (nx - tx).abs() < 0.1 && (ny - ty).abs() < 0.1 && (nw - tw).abs() < 0.1 {
                    current_view.set(target);
                    break;
                }

                current_view.set((nx, ny, nw, nh));
                tokio::time::sleep(std::time::Duration::from_millis(16)).await;
                
                steps += 1;
                if steps > 200 { break; }
            }
        });
    });

    let (view_x, view_y, view_w, view_h) = *current_view.read();
    let view_box = format!("{:.2} {:.2} {:.2} {:.2}", view_x, view_y, view_w, view_h);

    let (ping_cx, ping_cy, ping_r) = if let Some(region) = active_region {
        (region.map_x, region.map_y, 60.0)
    } else {
        (400.0, 600.0, 300.0)
    };

    rsx! {
        div { class: "absolute inset-0 pointer-events-none",
            svg { class: "w-full h-full", view_box: "{view_box}",
                image {
                    href: asset!("/assets/world.svg"),
                    x: "0",
                    y: "0",
                    width: "{MAP_WIDTH}",
                    height: "{MAP_HEIGHT}",
                    preserve_aspect_ratio: "xMidYMid slice",
                    opacity: "0.3",
                }

                if status == ConnectionStatus::Connected {
                    if let Some(region) = active_region {
                        if settings.multi_hop {
                            {
                                let entry_info = crate::models::LocationInfo::from_string(
                                    &settings.entry_location,
                                );
                                let exit_info = crate::models::LocationInfo::from_string(
                                    &settings.exit_location,
                                );
                                let entry_region = regions.iter().find(|r| r.name == entry_info.country);
                                let exit_region = regions.iter().find(|r| r.name == exit_info.country);
                                if let (Some(en), Some(ex)) = (entry_region, exit_region) {
                                    rsx! {
                                        line {
                                            x1: en.map_x,
                                            y1: en.map_y,
                                            x2: ex.map_x,
                                            y2: ex.map_y,
                                            stroke: "{COLOR_CONNECTED}",
                                            stroke_width: "2",
                                            stroke_dasharray: "5,5",
                                            class: "animate-pulse",
                                        }
                                        circle {
                                            cx: en.map_x,
                                            cy: en.map_y,
                                            r: "3",
                                            fill: "{COLOR_CONNECTED}",
                                        }
                                    }
                                } else {
                                    rsx! {}
                                }
                            }
                        } else {
                            line {
                                x1: "400",
                                y1: "600",
                                x2: "{region.map_x}",
                                y2: "{region.map_y}",
                                stroke: "{COLOR_CONNECTED}",
                                stroke_width: "2",
                                stroke_dasharray: "5,5",
                                class: "animate-pulse",
                            }
                        }
                    }
                }

                if status == ConnectionStatus::Connecting {
                    circle {
                        cx: "{ping_cx}",
                        cy: "{ping_cy}",
                        r: "{ping_r}",
                        stroke: "{COLOR_CONNECTING}",
                        stroke_width: "2",
                        fill: "none",
                        opacity: "0.2",
                        class: "animate-ping",
                    }
                }

                for region in regions.iter().cloned() {
                    {
                        let name = region.name.clone();
                        rsx! {
                            g {
                                key: "{name}",
                                class: "cursor-pointer transition-all duration-300 pointer-events-auto",
                                class: if country == region.name { "opacity-100 scale-125" } else { "opacity-40 hover:opacity-100" },
                                onclick: move |_| {
                                    vpn.connect(format!("{}, Auto", name));
                                },
                                circle {
                                    cx: "{region.map_x}",
                                    cy: "{region.map_y}",
                                    r: "5",
                                    fill: if country == region.name { "var(--primary)" } else { "var(--muted-foreground)" },
                                }
                                if country == region.name {
                                    circle {
                                        cx: "{region.map_x}",
                                        cy: "{region.map_y}",
                                        r: "12",
                                        fill: "var(--primary)",
                                        opacity: "0.2",
                                        class: "animate-pulse",
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
