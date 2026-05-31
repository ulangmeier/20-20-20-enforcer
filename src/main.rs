#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod autostart;

use std::num::NonZeroU32;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use winit::event::{Event, WindowEvent};
use winit::event_loop::ControlFlow;
use winit::window::{Window, WindowBuilder, Fullscreen};

#[derive(Clone, Copy)]
enum AppState {
    Waiting { next_break: Instant },
    Breaking { break_end: Instant },
}

enum UserEvent {
    ShowOverlay,
    UpdateTooltip,
}

fn create_eye_icon() -> tray_icon::Icon {
    let size = 32;
    let mut rgba = vec![0u8; size * size * 4];

    for y in 0..size {
        for x in 0..size {
            let cx = size as f32 / 2.0;
            let cy = size as f32 / 2.0;
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let dist = (dx * dx + dy * dy).sqrt();

            let idx = (y * size + x) * 4;
            if dist < 5.0 {
                // Pupil (black)
                rgba[idx] = 0;
                rgba[idx + 1] = 0;
                rgba[idx + 2] = 0;
                rgba[idx + 3] = 255;
            } else if dist < 12.0 {
                // Iris (blue)
                rgba[idx] = 30;
                rgba[idx + 1] = 144;
                rgba[idx + 2] = 255;
                rgba[idx + 3] = 255;
            } else if dist < 15.0 {
                // Outline (dark grey)
                rgba[idx] = 60;
                rgba[idx + 1] = 60;
                rgba[idx + 2] = 60;
                rgba[idx + 3] = 255;
            } else {
                // Transparent
                rgba[idx] = 0;
                rgba[idx + 1] = 0;
                rgba[idx + 2] = 0;
                rgba[idx + 3] = 0;
            }
        }
    }

    tray_icon::Icon::from_rgba(rgba, size as u32, size as u32).unwrap()
}

#[cfg(windows)]
fn force_foreground(window: &Window) {
    use raw_window_handle::HasRawWindowHandle;
    use winapi::um::winuser::{SetForegroundWindow, SetWindowPos, HWND_TOPMOST, SWP_SHOWWINDOW, SWP_NOMOVE, SWP_NOSIZE};
    use winapi::shared::windef::HWND;

    if let raw_window_handle::RawWindowHandle::Win32(handle) = window.raw_window_handle() {
        let hwnd = handle.hwnd as HWND;
        unsafe {
            SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_SHOWWINDOW | SWP_NOMOVE | SWP_NOSIZE);
            SetForegroundWindow(hwnd);
        }
    }
}

#[cfg(not(windows))]
fn force_foreground(_window: &Window) {}

fn format_duration(d: Duration) -> String {
    let secs = d.as_secs();
    format!("{:02}:{:02}", secs / 60, secs % 60)
}

fn main() {
    autostart::install();

    let test_mode = std::env::args().any(|a| a == "--test");
    let interval = if test_mode {
        Duration::from_secs(10)
    } else {
        Duration::from_secs(20 * 60)
    };
    let display_duration = if test_mode {
        Duration::from_secs(5)
    } else {
        Duration::from_secs(20)
    };

    let app_state = Arc::new(Mutex::new(AppState::Waiting {
        next_break: Instant::now() + interval,
    }));

    // Tray icon setup
    let icon = create_eye_icon();
    let menu = muda::Menu::new();
    let quit_item = muda::MenuItem::new("Quit", true, None);
    menu.append(&quit_item).unwrap();

    let tray = tray_icon::TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("20-20-20 eye protection active")
        .with_icon(icon)
        .build()
        .unwrap();

    // Handle menu events (Quit)
    let menu_channel = muda::MenuEvent::receiver();
    let quit_id = quit_item.id().clone();
    std::thread::spawn(move || {
        while let Ok(event) = menu_channel.recv() {
            if event.id == quit_id {
                std::process::exit(0);
            }
        }
    });

    // Event loop
    let event_loop = winit::event_loop::EventLoopBuilder::<UserEvent>::with_user_event().build();
    let proxy_overlay = event_loop.create_proxy();
    let proxy_tooltip = event_loop.create_proxy();

    // Overlay timer thread
    let state_for_timer = app_state.clone();
    std::thread::spawn(move || {
        std::thread::sleep(interval);
        loop {
            {
                let mut state = state_for_timer.lock().unwrap();
                *state = AppState::Waiting {
                    next_break: Instant::now() + interval,
                };
            }
            let _ = proxy_overlay.send_event(UserEvent::ShowOverlay);
            std::thread::sleep(interval);
        }
    });

    // Tooltip updater thread
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(Duration::from_secs(1));
            let _ = proxy_tooltip.send_event(UserEvent::UpdateTooltip);
        }
    });

    let mut overlay: Option<(Window, softbuffer::Context, softbuffer::Surface)> = None;
    let mut hide_at: Option<Instant> = None;
    let mut overlay_id: Option<winit::window::WindowId> = None;

    event_loop.run(move |event, target, control_flow| {
        match event {
            Event::UserEvent(UserEvent::ShowOverlay) => {
                if overlay.is_some() {
                    return;
                }

                let window = WindowBuilder::new()
                    .with_fullscreen(Some(Fullscreen::Borderless(None)))
                    .with_decorations(false)
                    .build(target)
                    .unwrap();

                force_foreground(&window);

                let context = unsafe { softbuffer::Context::new(&window).unwrap() };
                let surface = unsafe { softbuffer::Surface::new(&context, &window).unwrap() };

                window.request_redraw();

                let break_end = Instant::now() + display_duration;
                *app_state.lock().unwrap() = AppState::Breaking { break_end };

                overlay_id = Some(window.id());
                overlay = Some((window, context, surface));
                hide_at = Some(break_end);
                *control_flow = ControlFlow::WaitUntil(hide_at.unwrap());
            }
            Event::UserEvent(UserEvent::UpdateTooltip) => {
                let text = match *app_state.lock().unwrap() {
                    AppState::Waiting { next_break } => {
                        let remaining = next_break.saturating_duration_since(Instant::now());
                        format!(
                            "20-20-20 eye protection active, next break in {}",
                            format_duration(remaining)
                        )
                    }
                    AppState::Breaking { break_end } => {
                        let remaining = break_end.saturating_duration_since(Instant::now());
                        format!(
                            "20-20-20 eye protection active, break ends in {}",
                            format_duration(remaining)
                        )
                    }
                };
                let _ = tray.set_tooltip(Some(&text));
            }
            Event::RedrawRequested(id) => {
                if overlay_id == Some(id) {
                    if let Some((window, _, surface)) = &mut overlay {
                        let size = window.inner_size();
                        if let (Some(w), Some(h)) = (
                            NonZeroU32::new(size.width),
                            NonZeroU32::new(size.height),
                        ) {
                            surface.resize(w, h).unwrap();
                            let mut buffer = surface.buffer_mut().unwrap();
                            buffer.fill(0);
                            buffer.present().unwrap();
                        }
                    }
                }
            }
            Event::WindowEvent { event, window_id, .. } => {
                if overlay_id == Some(window_id) {
                    match event {
                        WindowEvent::CloseRequested | WindowEvent::Destroyed => {
                            overlay = None;
                            overlay_id = None;
                            hide_at = None;
                            *app_state.lock().unwrap() = AppState::Waiting {
                                next_break: Instant::now() + interval,
                            };
                            *control_flow = ControlFlow::Wait;
                        }
                        _ => {}
                    }
                }
            }
            Event::MainEventsCleared => {
                if let Some(t) = hide_at {
                    if Instant::now() >= t {
                        overlay = None;
                        overlay_id = None;
                        hide_at = None;
                        *app_state.lock().unwrap() = AppState::Waiting {
                            next_break: Instant::now() + interval,
                        };
                        *control_flow = ControlFlow::Wait;
                    } else {
                        *control_flow = ControlFlow::WaitUntil(t);
                    }
                } else {
                    *control_flow = ControlFlow::Wait;
                }
            }
            _ => {}
        }
    });
}
