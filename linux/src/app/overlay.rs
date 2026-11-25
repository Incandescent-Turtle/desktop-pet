//! Her on-screen surface: a layer-shell overlay on the monitor she's on, moved and redrawn
//! each tick.

use smithay_client_toolkit::{
    reexports::client::{
        QueueHandle,
        protocol::{wl_output, wl_shm},
    },
    shell::{
        WaylandSurface,
        wlr_layer::{Anchor, KeyboardInteractivity, Layer, LayerSurface},
    },
    shm::slot::Buffer,
};

use super::App;
use crate::pet::{self, Rect, SIZE};

struct Screen {
    output: wl_output::WlOutput,
    rect: Rect,
    scale: i32,
}

/// The 100x100 surface she's drawn in. It's a layer-shell surface on the compositor's
/// "overlay" layer, so it sits above all normal windows. It's anchored to its monitor's
/// top-left corner, and setting its margins moves it.
pub(super) struct Overlay {
    layer: LayerSurface,
    screen: Screen,
    /// The compositor has told us the surface is ready; we can't draw before that.
    configured: bool,
    last_look: Option<Look>,
    /// The frame currently on screen; kept alive until the next one replaces it.
    buffer: Option<Buffer>,
}

/// Everything that affects what's on screen. If it hasn't changed, we skip the redraw.
#[derive(PartialEq, Clone, Copy)]
struct Look {
    pos: (i32, i32),
    action: pet::Action,
    frame: usize,
    flipped: bool,
    bubble: Option<(pet::Bubble, usize)>,
    scale: i32,
}

impl App {
    fn screens(&self) -> Vec<Screen> {
        self.outputs
            .outputs()
            .filter_map(|output| {
                let info = self.outputs.info(&output)?;
                let (x, y) = info.logical_position?;
                let (w, h) = info.logical_size?;
                let scale = info.scale_factor.max(1);
                Some(Screen { output, rect: Rect { x, y, w, h }, scale })
            })
            .collect()
    }

    pub(super) fn screen_rects(&self) -> Vec<Rect> {
        self.screens().into_iter().map(|s| s.rect).collect()
    }

    /// Makes sure her overlay is on the monitor she's on.
    ///
    /// A layer-shell surface belongs to one monitor and can't move to another, so when her
    /// centre crosses onto a different monitor the overlay is replaced with a new one there.
    pub(super) fn follow_pet(&mut self, qh: &QueueHandle<Self>) {
        let Some(pet) = &self.pet else { return };
        let her_screen = self.screens().into_iter().find(|s| s.rect.contains(pet.centre()));
        let Some(her_screen) = her_screen else {
            return; // centre is between monitors (mid-drag): leave the overlay where it is
        };
        match &mut self.overlay {
            // same monitor: just refresh its details, in case its scale or position changed
            Some(o) if o.screen.output == her_screen.output => o.screen = her_screen,
            // no overlay yet, or she's crossed onto another monitor
            _ => self.create_overlay(qh, her_screen),
        }
    }

    fn create_overlay(&mut self, qh: &QueueHandle<Self>, screen: Screen) {
        let surface = self.compositor.create_surface(qh);
        let layer = self.layer_shell.create_layer_surface(
            qh,
            surface,
            Layer::Overlay,
            Some("desktop-pet"),
            Some(&screen.output),
        );
        layer.set_anchor(Anchor::TOP | Anchor::LEFT);
        layer.set_size(SIZE as u32, SIZE as u32);
        layer.set_exclusive_zone(-1); // ignore panels, so margins are from the monitor's corner
        layer.set_keyboard_interactivity(KeyboardInteractivity::OnDemand); // focus on click
        layer.commit();

        // replacing the old overlay destroys it; a drag in progress ends with it
        self.overlay =
            Some(Overlay { layer, screen, configured: false, last_look: None, buffer: None });
        self.cancel_press();
    }

    pub(super) fn draw(&mut self) {
        let (Some(o), Some(pet)) = (&mut self.overlay, &self.pet) else {
            return;
        };
        let look = Look {
            pos: pet.pos,
            action: pet.action,
            frame: pet.frame(),
            flipped: pet.flipped(),
            bubble: pet.bubble(),
            scale: o.screen.scale,
        };
        if !o.configured || o.last_look == Some(look) {
            return;
        }

        let size = SIZE * o.screen.scale;
        let (buffer, canvas) = self
            .pool
            .create_buffer(size, size, size * 4, wl_shm::Format::Argb8888)
            .expect("failed to create buffer");
        self.sprites.render(canvas, o.screen.scale, pet);

        let surface = o.layer.wl_surface();
        let (left, top) = (pet.pos.0 - o.screen.rect.x, pet.pos.1 - o.screen.rect.y);
        o.layer.set_margin(top, 0, 0, left);
        surface.set_buffer_scale(o.screen.scale);
        surface.damage_buffer(0, 0, size, size);
        buffer.attach_to(surface).expect("failed to attach buffer");
        o.layer.commit();

        o.buffer = Some(buffer);
        o.last_look = Some(look);
    }

    /// Where the overlay is on screen right now (it lags `pet.pos` until the next draw).
    pub(super) fn overlay_pos(&self) -> Option<(i32, i32)> {
        self.overlay.as_ref()?.last_look.map(|look| look.pos)
    }

    pub(super) fn overlay_configured(&mut self, layer: &LayerSurface) {
        if let Some(o) = &mut self.overlay
            && &o.layer == layer
        {
            o.configured = true;
            self.draw();
        }
    }

    /// The compositor removed the overlay (e.g. its monitor went away): respawn on the next tick.
    pub(super) fn overlay_closed(&mut self) {
        self.overlay = None;
        self.pet = None;
    }

    pub(super) fn output_removed(&mut self, output: &wl_output::WlOutput) {
        if self.overlay.as_ref().is_some_and(|o| &o.screen.output == output) {
            self.overlay_closed();
        }
    }
}
