//! Wayland event plumbing required by smithay-client-toolkit. Each handler either does
//! nothing or forwards to a method on `App` in main.rs.

use smithay_client_toolkit::{
    compositor::CompositorHandler,
    delegate_dispatch2, delegate_registry,
    dispatch2::Dispatch2,
    output::{OutputHandler, OutputState},
    reexports::client::{
        Connection, QueueHandle, WEnum,
        protocol::{wl_keyboard, wl_output, wl_pointer, wl_seat, wl_surface},
    },
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        Capability, SeatHandler, SeatState,
        pointer::{PointerEvent, PointerHandler},
    },
    shell::wlr_layer::{LayerShellHandler, LayerSurface, LayerSurfaceConfigure},
    shm::{Shm, ShmHandler},
};

use crate::{App, Keys};

impl LayerShellHandler for App {
    fn configure(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        layer: &LayerSurface,
        _: LayerSurfaceConfigure,
        _: u32,
    ) {
        if let Some(o) = &mut self.overlay
            && &o.layer == layer
        {
            o.configured = true;
            self.draw();
        }
    }

    fn closed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &LayerSurface) {
        // the compositor removed it (e.g. its monitor went away): respawn on the next tick
        self.overlay = None;
        self.pet = None;
    }
}

impl OutputHandler for App {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.outputs
    }
    fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
    fn output_destroyed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        output: wl_output::WlOutput,
    ) {
        if self.overlay.as_ref().is_some_and(|o| o.screen.output == output) {
            self.overlay = None;
            self.pet = None;
        }
    }
}

impl SeatHandler for App {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seats
    }
    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}

    fn new_capability(
        &mut self,
        _: &Connection,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard && self.keyboard.is_none() {
            self.keyboard = Some(seat.get_keyboard(qh, KeyboardData));
        }
        if capability == Capability::Pointer && self.pointer.is_none() {
            self.pointer = self.seats.get_pointer(qh, &seat).ok();
        }
    }

    fn remove_capability(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard
            && let Some(k) = self.keyboard.take()
        {
            k.release();
            self.keys = Keys::default();
        }
        if capability == Capability::Pointer
            && let Some(p) = self.pointer.take()
        {
            p.release();
        }
    }
}

impl PointerHandler for App {
    fn pointer_frame(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_pointer::WlPointer,
        events: &[PointerEvent],
    ) {
        self.on_pointer(events);
    }
}

/// Our wl_keyboard handler. We only need raw key codes, so no keymap/xkbcommon.
struct KeyboardData;

impl Dispatch2<wl_keyboard::WlKeyboard, App> for KeyboardData {
    fn event(
        &self,
        app: &mut App,
        _: &wl_keyboard::WlKeyboard,
        event: wl_keyboard::Event,
        _: &Connection,
        _: &QueueHandle<App>,
    ) {
        match event {
            wl_keyboard::Event::Key { key, state: WEnum::Value(state), .. } => {
                app.on_key(key, state == wl_keyboard::KeyState::Pressed);
            }
            // focus moved elsewhere: forget held keys so she doesn't walk forever
            wl_keyboard::Event::Leave { .. } => app.keys = Keys::default(),
            _ => {}
        }
    }
}

impl CompositorHandler for App {
    fn scale_factor_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: i32,
    ) {
    }
    fn transform_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: wl_output::Transform,
    ) {
    }
    fn frame(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: u32) {}
    fn surface_enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: &wl_output::WlOutput,
    ) {
    }
    fn surface_leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: &wl_output::WlOutput,
    ) {
    }
}

impl ShmHandler for App {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

impl ProvidesRegistryState for App {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry
    }
    registry_handlers![OutputState, SeatState];
}

delegate_registry!(App);
delegate_dispatch2!(App);
