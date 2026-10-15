use std::sync::Arc;
use std::time::{Duration, Instant};

use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::platform::macos::WindowAttributesExtMacOS;
use winit::window::{Window, WindowId};

use crate::mac_window::configure_macos_window;
use crate::render::PondRenderer;
use crate::simulation::PondWorld;

pub struct PondApp {
    window: Option<Arc<Window>>,
    renderer: Option<PondRenderer>,
    world: Option<PondWorld>,
    last_frame_time: Instant,
    scale_factor: f32,
    cursor_pos: (f32, f32),
}

impl PondApp {
    pub fn new() -> Self {
        Self {
            window: None,
            renderer: None,
            world: None,
            last_frame_time: Instant::now(),
            scale_factor: 1.0,
            cursor_pos: (0.0, 0.0),
        }
    }
}

impl Default for PondApp {
    fn default() -> Self {
        Self::new()
    }
}

impl ApplicationHandler for PondApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        // 1. Identify primary monitor resolution & positioning
        let primary_monitor = event_loop
            .primary_monitor()
            .or_else(|| event_loop.available_monitors().next())
            .expect("No display monitor detected");

        let physical_size = primary_monitor.size();
        let monitor_position = primary_monitor.position();
        let display_scale = primary_monitor.scale_factor() as f32;

        // Size internal buffer for ultra-lean memory (< 20 MB RAM)
        let base_w = (physical_size.width as f32 / display_scale).round() as u32;
        let logical_width = (base_w * 4 / 5).max(800);
        let base_h = (physical_size.height as f32 / display_scale).round() as u32;
        let logical_height = (base_h * 4 / 5).max(600);
        self.scale_factor = (physical_size.width as f32) / (logical_width as f32);

        // 2. Build borderless, transparent, non-resizable window matching primary screen
        let window_attributes = Window::default_attributes()
            .with_title("Virtual Koi Pond")
            .with_inner_size(physical_size)
            .with_position(monitor_position)
            .with_decorations(false)
            .with_resizable(false)
            .with_transparent(true)
            .with_has_shadow(false)
            .with_accepts_first_mouse(true);

        let window = Arc::new(
            event_loop
                .create_window(window_attributes)
                .expect("Failed to create native desktop window"),
        );

        // 3. Configure native macOS AppKit properties (window level, focus, collection behavior)
        configure_macos_window(&window);

        // 4. Initialize CoreAnimation / CoreGraphics CALayer renderer
        let renderer = PondRenderer::new(
            window.clone(),
            physical_size.width,
            physical_size.height,
            logical_width,
            logical_height,
        )
        .expect("Failed to initialize CALayer graphics pipeline");

        // 5. Initialize koi pond simulation world
        let world = PondWorld::new(logical_width as f32, logical_height as f32);

        self.window = Some(window);
        self.renderer = Some(renderer);
        self.world = Some(world);
        self.last_frame_time = Instant::now();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let dt = now
                    .duration_since(self.last_frame_time)
                    .as_secs_f32()
                    .min(0.05);
                self.last_frame_time = now;

                if let (Some(world), Some(renderer)) = (self.world.as_mut(), self.renderer.as_mut())
                {
                    world.update(dt);
                    if let Err(e) = renderer.render(world) {
                        eprintln!("Render error: {:?}", e);
                    }
                }
            }

            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                let base_scale = scale_factor as f32;
                if let Some(w) = self.window.as_ref() {
                    let phys_w = w.inner_size().width as f32;
                    let log_w = (phys_w / base_scale * 0.8).max(800.0);
                    self.scale_factor = phys_w / log_w;
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                let logical_x = position.x as f32 / self.scale_factor;
                let logical_y = position.y as f32 / self.scale_factor;
                self.cursor_pos = (logical_x, logical_y);

                if let Some(world) = self.world.as_mut() {
                    world.set_cursor(Some(self.cursor_pos));
                }
            }

            WindowEvent::CursorLeft { .. } => {
                if let Some(world) = self.world.as_mut() {
                    world.set_cursor(None);
                }
            }

            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                if let Some(world) = self.world.as_mut() {
                    world.handle_click(self.cursor_pos.0, self.cursor_pos.1);
                }
                if let Some(w) = self.window.as_ref() {
                    w.request_redraw();
                }
            }

            WindowEvent::Resized(new_size) => {
                let logical_w = (new_size.width as f32 / self.scale_factor).round() as u32;
                let logical_h = (new_size.height as f32 / self.scale_factor).round() as u32;

                if let Some(renderer) = self.renderer.as_mut() {
                    let _ = renderer.resize(new_size.width, new_size.height, logical_w, logical_h);
                }
                if let Some(world) = self.world.as_mut() {
                    world.resize(logical_w as f32, logical_h as f32);
                }
            }

            WindowEvent::CloseRequested => {
                event_loop.exit();
            }

            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        const TARGET_FRAME_DURATION: Duration = Duration::from_millis(16);
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_frame_time);

        if elapsed >= TARGET_FRAME_DURATION {
            if let Some(w) = self.window.as_ref() {
                w.request_redraw();
            }
            event_loop.set_control_flow(ControlFlow::WaitUntil(now + TARGET_FRAME_DURATION));
        } else {
            let next_deadline = self.last_frame_time + TARGET_FRAME_DURATION;
            event_loop.set_control_flow(ControlFlow::WaitUntil(next_deadline));
        }
    }
}
