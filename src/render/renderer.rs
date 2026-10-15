use core_graphics::base::{kCGBitmapByteOrder32Big, kCGImageAlphaPremultipliedLast};
use core_graphics::color_space::CGColorSpace;
use core_graphics::data_provider::CGDataProvider;
use core_graphics::image::CGImage;
use objc2::msg_send;
use objc2::runtime::{AnyObject, Bool};
use std::sync::Arc;
use tiny_skia::PixmapMut;
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window;

use crate::render::fish_draw::FishDrawer;
use crate::simulation::world::PondWorld;

/// Lightweight CoreAnimation / CoreGraphics rendering pipeline.
///
/// Bypasses heavy GPU swapchains (Metal/wgpu IOSurfaces) to maintain an ultra-lean
/// memory footprint (~18-22 MB physical footprint) while delivering hardware-accelerated
/// desktop compositing directly via `CALayer`.
pub struct PondRenderer {
    sublayer: *mut AnyObject,
    color_space: CGColorSpace,
    buffer: Vec<u8>,
    width: u32,
    height: u32,
}

// Safety: The sublayer pointer is only accessed on the main AppKit thread.
unsafe impl Send for PondRenderer {}
unsafe impl Sync for PondRenderer {}

impl PondRenderer {
    /// Initializes the CoreAnimation sublayer attached to the window's content view.
    pub fn new(
        window: Arc<Window>,
        _surface_width: u32,
        _surface_height: u32,
        buffer_width: u32,
        buffer_height: u32,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let raw_handle = window.window_handle()?.as_raw();

        let root_layer: *mut AnyObject = match raw_handle {
            RawWindowHandle::AppKit(handle) => {
                let view: *mut AnyObject = handle.ns_view.as_ptr().cast();
                let _: () = unsafe { msg_send![view, setWantsLayer: Bool::YES] };
                let layer: *mut AnyObject = unsafe { msg_send![view, layer] };
                assert!(!layer.is_null(), "Failed to acquire root CALayer");
                layer
            }
            _ => panic!("Expected AppKit window handle on macOS"),
        };

        let sublayer: *mut AnyObject = unsafe {
            let cls = objc2::class!(CALayer);
            let layer: *mut AnyObject = msg_send![cls, layer];
            let bounds: objc2_foundation::CGRect = msg_send![root_layer, bounds];
            let _: () = msg_send![layer, setFrame: bounds];
            let _: () = msg_send![layer, setOpaque: Bool::NO];
            // Match resizing mask so it tracks window dimensions automatically
            let _: () = msg_send![layer, setAutoresizingMask: 18u64]; // kCALayerWidthSizable | kCALayerHeightSizable
            let _: () = msg_send![root_layer, addSublayer: layer];
            layer
        };

        let buffer_size = (buffer_width as usize) * (buffer_height as usize) * 4;
        let buffer = vec![0u8; buffer_size];
        let color_space = CGColorSpace::create_device_rgb();

        Ok(Self {
            sublayer,
            color_space,
            buffer,
            width: buffer_width,
            height: buffer_height,
        })
    }

    /// Resizes the internal drawing buffer and updates layer bounds.
    pub fn resize(
        &mut self,
        _surface_width: u32,
        _surface_height: u32,
        buffer_width: u32,
        buffer_height: u32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if buffer_width > 0
            && buffer_height > 0
            && (self.width != buffer_width || self.height != buffer_height)
        {
            self.width = buffer_width;
            self.height = buffer_height;
            let buffer_size = (buffer_width as usize) * (buffer_height as usize) * 4;
            self.buffer.resize(buffer_size, 0);
        }
        Ok(())
    }

    /// Clears the pixel buffer with transparent pixels (alpha = 0), renders all active
    /// ripples and koi fish using `tiny-skia`, and assigns the frame to the `CALayer`.
    pub fn render(&mut self, world: &PondWorld) -> Result<(), Box<dyn std::error::Error>> {
        // 1. Transparent background: zero out all RGBA bytes
        self.buffer.fill(0);

        // 2. Anti-aliased 2D vector rasterization
        if let Some(mut pixmap) = PixmapMut::from_bytes(&mut self.buffer, self.width, self.height) {
            for ripple in &world.ripples {
                FishDrawer::draw_ripple(&mut pixmap, ripple);
            }

            for fish in &world.fish {
                FishDrawer::draw_fish(&mut pixmap, fish);
            }
        }

        // 3. Create CGImage wrapping the buffer and update CALayer contents
        let provider = unsafe { CGDataProvider::from_slice(&self.buffer) };
        let image = CGImage::new(
            self.width as usize,
            self.height as usize,
            8,
            32,
            (self.width as usize) * 4,
            &self.color_space,
            kCGBitmapByteOrder32Big | kCGImageAlphaPremultipliedLast,
            &provider,
            false,
            0,
        );

        unsafe {
            let catransaction = objc2::class!(CATransaction);
            let _: () = msg_send![catransaction, begin];
            let _: () = msg_send![catransaction, setDisableActions: Bool::YES];
            let image_ptr = image.as_ref() as *const _ as *mut AnyObject;
            let _: () = msg_send![self.sublayer, setContents: image_ptr];
            let _: () = msg_send![catransaction, commit];
        }

        Ok(())
    }
}
