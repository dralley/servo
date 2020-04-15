/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

//! A headless window implementation.

use crate::events_loop::EventsLoop;
use crate::window_trait::WindowPortsMethods;
use euclid::{Point2D, Rotation3D, Scale, Size2D, UnknownUnit, Vector3D};
use winit;
use servo::compositing::windowing::{AnimationState, WindowEvent};
use servo::compositing::windowing::{EmbedderCoordinates, WindowMethods};
use servo::servo_geometry::DeviceIndependentPixel;
use servo::style_traits::DevicePixel;
use servo::webrender_api::units::{DeviceIntRect, DeviceIntSize};
use servo_media::player::context as MediaPlayerCtxt;
use servo::webrender_surfman::WebrenderSurfman;
use std::cell::Cell;
use std::rc::Rc;
use surfman::Connection;
use surfman::Device;
use surfman::NativeWidget;
use surfman::SurfaceType;

pub struct Window {
    webrender_surfman: WebrenderSurfman,
    animation_state: Cell<AnimationState>,
    fullscreen: Cell<bool>,
    device_pixels_per_px: Option<f32>,
    inner_size: Cell<Size2D<i32, DeviceIndependentPixel>>,
    size_changed: Cell<bool>, // We need to transmit resize events, but don't have/need an event queue
}

impl Window {
    pub fn new(
        size: Size2D<u32, DeviceIndependentPixel>,
        device_pixels_per_px: Option<f32>,
    ) -> Rc<dyn WindowPortsMethods> {
        // Initialize surfman
        let connection = Connection::new().expect("Failed to create connection");
        let adapter = connection.create_software_adapter().expect("Failed to create adapter");
        let surface_type = SurfaceType::Generic { size: size.to_untyped().to_i32() };
        let webrender_surfman = WebrenderSurfman::create(
            &connection,
            &adapter,
            surface_type,
        ).expect("Failed to create WR surfman");

        let window = Window {
            webrender_surfman,
            animation_state: Cell::new(AnimationState::Idle),
            fullscreen: Cell::new(false),
            device_pixels_per_px,
            inner_size: Cell::new(size.to_i32()),
            size_changed: Cell::new(false),
        };

        Rc::new(window)
    }

    fn servo_hidpi_factor(&self) -> Scale<f32, DeviceIndependentPixel, DevicePixel> {
        match self.device_pixels_per_px {
            Some(device_pixels_per_px) => Scale::new(device_pixels_per_px),
            _ => Scale::new(1.0),
        }
    }
}

impl WindowPortsMethods for Window {
    fn get_events(&self) -> Vec<WindowEvent> {
        let mut vec = Vec::new();

        if self.size_changed.take() {
            vec.push(WindowEvent::Resize);
        }

        vec
    }

    fn set_inner_size(&self, size: DeviceIntSize) {
        let (width, height) = size.into();
        // Clamp width and height to 1 - webrender_surfman doesn't accept 0-dimension windows
        let width = if width > 0 { width } else { 1 };
        let height = if height > 0 { height } else { 1 };

        let new_size = Size2D::new(width, height);
        if self.inner_size.get() != new_size {
            self.webrender_surfman.resize(new_size.to_untyped()).expect("Failed to resize");
            self.inner_size.set(new_size);
        }
        self.size_changed.set(true);
    }

    fn has_events(&self) -> bool {
        self.size_changed.get()
    }

    fn id(&self) -> winit::WindowId {
        unsafe { winit::WindowId::dummy() }
    }

    fn page_height(&self) -> f32 {
        let height = self.webrender_surfman
            .context_surface_info()
            .unwrap_or(None)
            .map(|info| info.size.height)
            .unwrap_or(0);
        let dpr = self.servo_hidpi_factor();
        height as f32 * dpr.get()
    }

    fn set_fullscreen(&self, state: bool) {
        self.fullscreen.set(state);
    }

    fn get_fullscreen(&self) -> bool {
        return self.fullscreen.get();
    }

    fn is_animating(&self) -> bool {
        self.animation_state.get() == AnimationState::Animating
    }

    fn winit_event_to_servo_event(&self, _event: winit::WindowEvent) {
        // Not expecting any winit events.
        unreachable!("Did not expect a WindowEvent for a headless session");
    }

    fn new_glwindow(&self, _events_loop: &EventsLoop) -> Box<dyn webxr::glwindow::GlWindow> {
        unimplemented!()
    }
}

impl WindowMethods for Window {
     fn get_coordinates(&self) -> EmbedderCoordinates {
        let dpr = self.servo_hidpi_factor();
        let size = self.webrender_surfman
            .context_surface_info()
            .unwrap_or(None)
            .map(|info| Size2D::from_untyped(info.size))
            .unwrap_or(Size2D::new(0, 0));
        let viewport = DeviceIntRect::new(Point2D::zero(), size);
        EmbedderCoordinates {
            viewport,
            framebuffer: size,
            window: (size, Point2D::zero()),
            screen: size,
            screen_avail: size,
            hidpi_factor: dpr,
        }
    }

     fn set_animation_state(&self, state: AnimationState) {
        self.animation_state.set(state);
    }

     fn get_gl_context(&self) -> MediaPlayerCtxt::GlContext {
        MediaPlayerCtxt::GlContext::Unknown
    }

    fn get_native_display(&self) -> MediaPlayerCtxt::NativeDisplay {
        MediaPlayerCtxt::NativeDisplay::Unknown
    }

    fn get_gl_api(&self) -> MediaPlayerCtxt::GlApi {
        MediaPlayerCtxt::GlApi::None
    }

    fn webrender_surfman(&self) -> WebrenderSurfman {
        self.webrender_surfman.clone()
    }
}

impl webxr::glwindow::GlWindow for Window {
    fn get_native_widget(&self, _device: &Device) -> NativeWidget {
        unimplemented!()
    }

    fn get_rotation(&self) -> Rotation3D<f32, UnknownUnit, UnknownUnit> {
        Rotation3D::identity()
    }

    fn get_translation(&self) -> Vector3D<f32, UnknownUnit> {
        Vector3D::zero()
    }
}
