use dioxus::prelude::*;
use crate::state::ConnectionState;
use crate::components::DashboardMap;
use crate::models::ConnectionStatus;

#[component]
pub fn Dashboard() -> Element {
    let state = use_context::<ConnectionState>();
    let status = (state.status)();
    let download_speed = (state.download_speed)();
    let upload_speed = (state.upload_speed)();
    
    let location_text = (state.current_location)();
    let location = crate::models::LocationInfo::from_string(&location_text);

    let regions = state.regions.read();
    
    rsx! {
        div { class: "relative w-full flex-1 bg-background overflow-hidden flex flex-col",
            DashboardMap { regions: regions.clone(), country: location.country, status }
            
            if status == ConnectionStatus::Connected {
                div { class: "absolute top-4 left-4 flex flex-col gap-2 pointer-events-none",
                    div { class: "bg-background/40 backdrop-blur-md border border-white/10 rounded-xl p-3 flex flex-col gap-1 shadow-lg",
                        div { class: "flex items-center gap-2",
                            div { class: "w-1.5 h-1.5 rounded-full bg-status-success" }
                            span { class: "text-[10px] font-bold text-muted-foreground uppercase tracking-widest", "Down" }
                        }
                        span { class: "text-sm font-bold font-mono", "{download_speed:.1} Mbps" }
                    }
                    div { class: "bg-background/40 backdrop-blur-md border border-white/10 rounded-xl p-3 flex flex-col gap-1 shadow-lg",
                        div { class: "flex items-center gap-2",
                            div { class: "w-1.5 h-1.5 rounded-full bg-primary" }
                            span { class: "text-[10px] font-bold text-muted-foreground uppercase tracking-widest", "Up" }
                        }
                        span { class: "text-sm font-bold font-mono", "{upload_speed:.1} Mbps" }
                    }
                }
            }
        }
    }
}
