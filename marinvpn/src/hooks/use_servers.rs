use crate::models::{City, Region};
use crate::services::{AppService, ProductionAppService};
use dioxus::prelude::*;
use futures_util::future;
use std::time::Duration;

pub fn use_servers() -> Signal<Vec<Region>> {
    let app_service = use_hook(|| ProductionAppService);
    use_servers_internal(app_service)
}

pub fn use_servers_with_service<S: AppService>(service: S) -> Signal<Vec<Region>> {
    use_servers_internal(service)
}

fn use_servers_internal<S: AppService>(app_service: S) -> Signal<Vec<Region>> {
    let mut regions = use_signal(crate::data::get_default_regions);

    let service = app_service.clone();
    use_future(move || {
        let service = service.clone();
        async move {
            loop {
                // service is safe to use here because we cloned it into the outer closure
                // and it's moved into this async block.
                // But wait, the async block runs in a loop? No, the loop is INSIDE the async block.
                // So `service` is moved into the async block once and reused in the loop.
                // The error was because `service.clone()` inside the loop for `ping_tasks`
                // was trying to move out of `service` which was already moved into the async block.
                // We need to clone it for `ping_tasks` properly.
                
                match service.get_servers().await {
                    Ok(api_servers) => {
                        let mut ping_tasks = Vec::new();
                        for s in &api_servers {
                            let endpoint = s.endpoint.clone();
                            let svc = service.clone(); // Clone for each task
                            ping_tasks.push(async move {
                                svc.measure_latency(&endpoint).await.unwrap_or(999)
                            });
                        }

                        let pings = future::join_all(ping_tasks).await;

                        let mut new_regions: Vec<Region> = Vec::new();
                        for (i, s) in api_servers.into_iter().enumerate() {
                            let ping = pings[i];

                            if let Some(reg) = new_regions.iter_mut().find(|r| r.name == s.country) {
                                if !reg.cities.iter().any(|c| c.name == s.city) {
                                    reg.cities.push(City {
                                        name: s.city,
                                        load: 0,
                                        ping: ping as u8,
                                    });
                                }
                            } else {
                                let defaults = crate::data::get_default_regions();
                                let (flag, x, y) = defaults
                                    .iter()
                                    .find(|r| r.name == s.country)
                                    .map(|r| (r.flag.clone(), r.map_x, r.map_y))
                                    .unwrap_or(("🌐".to_string(), 0.0, 0.0));

                                new_regions.push(Region {
                                    name: s.country,
                                    flag,
                                    map_x: x,
                                    map_y: y,
                                    cities: vec![City {
                                        name: s.city,
                                        load: 0,
                                        ping: ping as u8,
                                    }],
                                });
                            }
                        }
                        if !new_regions.is_empty() {
                            regions.set(new_regions);
                        }
                    }
                    Err(e) => tracing::error!("Failed to sync server list: {}", e),
                }
                tokio::time::sleep(Duration::from_secs(60)).await;
            }
        }
    });

    regions
}
