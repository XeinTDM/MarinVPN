use dioxus::prelude::*;
use dioxus::desktop::{use_window, use_wry_event_handler, WindowEvent};
use dioxus::desktop::tao::event::Event;
use dioxus::desktop::tao::dpi::PhysicalPosition;
use tray_icon::{TrayIconEvent, TrayIconBuilder, Icon, Rect};
use std::time::{Instant, Duration};
use std::sync::{Arc, Mutex};
use image::GenericImageView;
use std::io::Cursor;

pub const WINDOW_WIDTH: f64 = 315.0;
pub const WINDOW_HEIGHT: f64 = 560.0;

pub fn use_tray_management() {
    let window = use_window();
    let last_focus_lost = use_hook(|| Arc::new(Mutex::new(Instant::now() - Duration::from_secs(1))));

    // Hide window when it loses focus
    let last_focus_handler = last_focus_lost.clone();
    let window_handler = window.clone();
    use_wry_event_handler(move |event, _| {
        if let Event::WindowEvent { event: WindowEvent::Focused(false), .. } = event {
            if let Ok(mut last) = last_focus_handler.lock() {
                *last = Instant::now();
            }
            window_handler.window.set_visible(false);
        }
    });

    let window_coroutine = window.clone();
    let last_focus_coroutine = last_focus_lost.clone();
    
    use_coroutine(move |_: UnboundedReceiver<()>| {
        let window = window_coroutine.clone();
        let last_focus = last_focus_coroutine.clone();
        async move {
            let tray_channel = TrayIconEvent::receiver();
            let mut last_click = Instant::now();
            loop {
                while let Ok(event) = tray_channel.try_recv() {
                    match event {
                        TrayIconEvent::Click { rect, .. } => {
                             if last_click.elapsed().as_millis() < 200 {
                                 continue;
                             }
                             last_click = Instant::now();

                             let was_just_hidden = if let Ok(last) = last_focus.lock() {
                                 last.elapsed().as_millis() < 200
                             } else {
                                 false
                             };

                             let is_visible = window.window.is_visible();
                             
                             if is_visible {
                                 window.window.set_visible(false);
                             }
                             else if !was_just_hidden {
                                 position_window_at_tray(&window, rect);
                                 window.window.set_visible(true);
                                 window.set_focus();
                             }
                        }
                        _ => {}
                    }
                }
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        }
    });
}

fn position_window_at_tray(window: &dioxus::desktop::DesktopContext, rect: Rect) {
    if let Some(monitor) = window.window.current_monitor() {
        let scale_factor = monitor.scale_factor();
        let w = WINDOW_WIDTH * scale_factor;
        let h = WINDOW_HEIGHT * scale_factor;
        let margin_y = 60.0 * scale_factor;

        // Center window horizontally over the tray icon center
        let icon_center_x = rect.position.x as f64 + (rect.size.width as f64 / 2.0);
        let x = icon_center_x - (w / 2.0);

        // Position above the tray area (monitor bottom - height - margin)
        let monitor_pos = monitor.position();
        let monitor_size = monitor.size();
        let y = (monitor_pos.y as f64 + monitor_size.height as f64) - h - margin_y;

        window.window.set_outer_position(PhysicalPosition::new(x as i32, y as i32));
    }
}

pub fn create_tray_icon() -> tray_icon::TrayIcon {
    let icon_bytes = include_bytes!("../assets/favicon.ico");
    let icon_image = image::load(Cursor::new(icon_bytes), image::ImageFormat::Ico)
        .expect("Failed to load icon");
    let (width, height) = icon_image.dimensions();
    let rgba = icon_image.into_rgba8().into_vec();
    let icon = Icon::from_rgba(rgba, width, height).expect("Failed to create icon");

    TrayIconBuilder::new()
        .with_tooltip("MarinVPN")
        .with_icon(icon)
        .build()
        .unwrap()
}
