//! # Interactive Virtual Koi Pond for macOS
//!
//! A minimal, highly resource-efficient macOS desktop application written in Rust.
//! It acts as an interactive, transparent virtual koi pond running permanently
//! behind all other desktop applications.
//!
//! ### Core Features:
//! - Stays strictly behind normal windows (`below kCGNormalWindowLevel`, targeting desktop level).
//! - Never steals focus, becomes key, or intercepts keyboard events when clicked.
//! - Visible across all virtual desktops (Spaces) and stationary during Mission Control.
//! - Completely transparent, borderless, matching primary screen resolution.
//! - Autonomous koi fish with smooth steering behaviors and spine inverse kinematics.
//! - Interactive surface ripple creation and fish scatter upon mouse clicks.
//! - Memory footprint strictly under 25 MB (bypassing heavy Metal swapchains via direct CALayer compositing).

mod app;
mod mac_window;
mod render;
mod simulation;

use app::PondApp;
use winit::event_loop::EventLoop;
use winit::platform::macos::{ActivationPolicy, EventLoopBuilderExtMacOS};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut builder = EventLoop::builder();
    builder.with_activation_policy(ActivationPolicy::Accessory);

    let event_loop = builder.build()?;
    let mut app = PondApp::new();

    event_loop.run_app(&mut app)?;

    Ok(())
}
