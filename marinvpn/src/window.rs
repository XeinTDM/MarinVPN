use dioxus::desktop::tao::dpi::PhysicalPosition;
use dioxus::desktop::tao::event::Event;
use dioxus::desktop::{use_window, use_wry_event_handler, WindowEvent};
use dioxus::prelude::*;
use image::GenericImageView;
use std::io::Cursor;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
use tracing::error;
use tray_icon::{Icon, Rect, TrayIcon, TrayIconBuilder, TrayIconEvent};

pub const WINDOW_WIDTH: f64 = 315.0;
pub const WINDOW_HEIGHT: f64 = 560.0;

pub static TRAY_UPDATE_SENDER: OnceLock<tokio::sync::mpsc::UnboundedSender<String>> =
    OnceLock::new();
pub static TRAY_ICON_SENDER: OnceLock<tokio::sync::mpsc::UnboundedSender<Option<String>>> =
    OnceLock::new();

pub fn update_tray_tooltip(tooltip: &str) {
    if let Some(sender) = TRAY_UPDATE_SENDER.get() {
        let _ = sender.send(tooltip.to_string());
    }
}

pub fn update_tray_icon_path(path: Option<&str>) {
    if let Some(sender) = TRAY_ICON_SENDER.get() {
        let _ = sender.send(path.map(|p| p.to_string()));
    }
}

pub fn use_tray_management() {
    let window = use_window();
    let last_focus_lost =
        use_hook(|| Arc::new(Mutex::new(Instant::now() - Duration::from_secs(1))));

    let rx_holder = use_hook(|| {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<String>();
        let _ = TRAY_UPDATE_SENDER.set(tx);
        Arc::new(Mutex::new(Some(rx)))
    });
    let icon_rx_holder = use_hook(|| {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<Option<String>>();
        let _ = TRAY_ICON_SENDER.set(tx);
        Arc::new(Mutex::new(Some(rx)))
    });

    let last_focus_handler = last_focus_lost.clone();
    let window_handler = window.clone();
    use_wry_event_handler(move |event, _| {
        if let Event::WindowEvent {
            event: WindowEvent::Focused(false),
            ..
        } = event
        {
            if let Ok(mut last) = last_focus_handler.lock() {
                *last = Instant::now();
            }
            window_handler.window.set_visible(false);
        }
    });

    let window_coroutine = window.clone();
    let last_focus_coroutine = last_focus_lost.clone();

    let rx_holder_spawn = rx_holder.clone();
    let icon_rx_holder_spawn = icon_rx_holder.clone();
    use_hook(move || {
        spawn(async move {
            let tray = match create_tray_icon() {
                Some(t) => t,
                None => {
                    error!("Tray icon creation failed. Tray functionality will be disabled.");
                    return;
                }
            };
            let tray_channel = TrayIconEvent::receiver();
            let mut last_click = Instant::now();

            let mut rx = rx_holder_spawn.lock().unwrap().take().unwrap_or_else(|| {
                let (_, rx) = tokio::sync::mpsc::unbounded_channel();
                rx
            });
            let mut icon_rx = icon_rx_holder_spawn
                .lock()
                .unwrap()
                .take()
                .unwrap_or_else(|| {
                    let (_, rx) = tokio::sync::mpsc::unbounded_channel();
                    rx
                });

            loop {
                while let Ok(event) = tray_channel.try_recv() {
                    if let TrayIconEvent::Click { rect, .. } = event {
                        if last_click.elapsed().as_millis() < 200 {
                            continue;
                        }
                        last_click = Instant::now();

                        let was_just_hidden = if let Ok(last) = last_focus_coroutine.lock() {
                            last.elapsed().as_millis() < 200
                        } else {
                            false
                        };

                        let is_visible = window_coroutine.window.is_visible();

                        if is_visible {
                            window_coroutine.window.set_visible(false);
                        } else if !was_just_hidden {
                            position_window_at_tray(&window_coroutine, rect);
                            window_coroutine.window.set_visible(true);
                            window_coroutine.set_focus();
                        }
                    }
                }

                while let Ok(tooltip) = rx.try_recv() {
                    let _ = tray.set_tooltip(Some(tooltip));
                }

                while let Ok(path) = icon_rx.try_recv() {
                    let icon = match path {
                        Some(ref p) if !p.trim().is_empty() => {
                            load_tray_icon_from_path(p).unwrap_or_else(default_tray_icon)
                        }
                        _ => default_tray_icon(),
                    };
                    let _ = tray.set_icon(Some(icon));
                }

                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        });
    });
}

fn position_window_at_tray(window: &dioxus::desktop::DesktopContext, rect: Rect) {
    if let Some(monitor) = window.window.current_monitor() {
        let scale_factor = monitor.scale_factor();
        let w = WINDOW_WIDTH * scale_factor;
        let h = WINDOW_HEIGHT * scale_factor;
        let margin_y = 60.0 * scale_factor;

        let icon_center_x = rect.position.x + (rect.size.width as f64 / 2.0);
        let x = icon_center_x - (w / 2.0);

        let monitor_pos = monitor.position();
        let monitor_size = monitor.size();
        let y = (monitor_pos.y as f64 + monitor_size.height as f64) - h - margin_y;

        window
            .window
            .set_outer_position(PhysicalPosition::new(x as i32, y as i32));
    }
}

pub fn create_tray_icon() -> Option<TrayIcon> {
    let icon = default_tray_icon();

    match TrayIconBuilder::new()
        .with_tooltip("MarinVPN")
        .with_icon(icon)
        .build()
    {
        Ok(tray) => Some(tray),
        Err(e) => {
            error!("Failed to build system tray: {}", e);
            None
        }
    }
}

fn default_tray_icon() -> Icon {
    let icon_bytes = include_bytes!("../assets/favicon.ico");
    let icon_image = image::load(Cursor::new(icon_bytes), image::ImageFormat::Ico)
        .unwrap_or_else(|_| image::DynamicImage::new_rgba8(32, 32));
    let (width, height) = icon_image.dimensions();
    let rgba = icon_image.into_rgba8().into_vec();
    Icon::from_rgba(rgba, width, height).unwrap_or_else(|_| {
        let empty = vec![0u8; (32 * 32 * 4) as usize];
        Icon::from_rgba(empty, 32, 32).unwrap()
    })
}

fn load_tray_icon_from_path(path: &str) -> Option<Icon> {
    let image = image::open(path).ok()?;
    let (width, height) = image.dimensions();
    let rgba = image.into_rgba8().into_vec();
    Icon::from_rgba(rgba, width, height).ok()
}
