use dioxus::prelude::*;
use crate::models::{Region, ConnectionStatus};
use crate::state::ConnectionState;

#[component]
pub fn DashboardMap(regions: &'static [Region], country: String, status: ConnectionStatus) -> Element {
    let mut state = use_context::<ConnectionState>();

    rsx! {
        div { class: "absolute inset-0 opacity-30 pointer-events-none",
            svg { class: "w-full h-full", view_box: "0 0 800 600",
                // World Map background
                image {
                    href: asset!("/assets/world.svg"),
                    x: "0",
                    y: "0",
                    width: "800",
                    height: "600",
                    preserve_aspect_ratio: "xMidYMid slice",
                }

                // Abstract shapes (reduced opacity to not clash with map)
                path {
                    d: "M 150 100 Q 250 100 300 250 T 200 400 Z",
                    fill: "var(--accent)",
                    opacity: "0.1",
                }
                path {
                    d: "M 400 120 Q 500 100 600 150 T 550 300 Z",
                    fill: "var(--accent)",
                    opacity: "0.3",
                }

                if status == ConnectionStatus::Connected {
                    line {
                        x1: "400",
                        y1: "600",
                        x2: "460",
                        y2: "140",
                        stroke: "oklch(0.696 0.17 162.48)", // chart-2 / success
                        stroke_width: "2",
                        stroke_dasharray: "5,5",
                        class: "animate-pulse",
                    }
                }

                if status == ConnectionStatus::Connecting {
                    circle {
                        cx: "400",
                        cy: "600",
                        r: "300",
                        stroke: "oklch(0.769 0.188 70.08)", // chart-3 / warning
                        stroke_width: "2",
                        fill: "none",
                        opacity: "0.2",
                        class: "animate-ping",
                    }
                }

                // Map markers
                for region in regions {
                    g {
                        class: "cursor-pointer transition-all duration-300 pointer-events-auto",
                        class: if country == region.name { "opacity-100 scale-125" } else { "opacity-40 hover:opacity-100" },
                        onclick: move |_| {
                            state.current_location.set(format!("{}, Auto", region.name));
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
