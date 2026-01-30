use dioxus::prelude::*;
use std::time::Duration;
use crate::state::ConnectionState;
use crate::data::get_regions;
use crate::components::DashboardMap;

#[component]
pub fn Dashboard() -> Element {
    let state = use_context::<ConnectionState>();
    let status = (state.status)();
    
    let location_text = (state.current_location)();
    let location_parts: Vec<&str> = location_text.split(',').collect();
    let country = location_parts.get(0).unwrap_or(&"Unknown").trim();

    let regions = get_regions();
    
    rsx! {
        div { class: "relative w-full flex-1 bg-background overflow-hidden flex flex-col",
            DashboardMap { regions, country: country.to_string(), status }
        }
    }
}

#[component]
fn ConnectionTimer(since: f64) -> Element {
    let mut elapsed = use_signal(|| {
        let now = chrono::Utc::now().timestamp() as f64;
        (now - since).max(0.0) as i32
    });

    use_future(move || async move {
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let now = chrono::Utc::now().timestamp() as f64;
            elapsed.set((now - since).max(0.0) as i32);
        }
    });

    let format_time = |seconds: i32| {
        let h = seconds / 3600;
        let m = (seconds % 3600) / 60;
        let s = seconds % 60;
        format!("{:02}:{:02}:{:02}", h, m, s)
    };

    rsx! {
        span { class: "font-mono font-bold text-foreground", "{format_time(elapsed())}" }
    }
}