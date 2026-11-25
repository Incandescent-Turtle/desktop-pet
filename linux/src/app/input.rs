//! Mouse and keyboard input.

use std::mem;

use smithay_client_toolkit::seat::pointer::{BTN_LEFT, PointerEvent, PointerEventKind};

use super::App;
use crate::pet::{Keys, SIZE};

/// Pointer movement (px) before a press counts as a drag instead of a click.
const DRAG_THRESHOLD: f64 = 3.0;

// Linux evdev key codes, which is what wl_keyboard sends
const KEY_ESC: u32 = 1;
const KEY_W: u32 = 17;
const KEY_UP: u32 = 103;
const KEY_LEFT: u32 = 105;
const KEY_RIGHT: u32 = 106;
const KEY_DOWN: u32 = 108;

/// The left mouse button, on her.
#[derive(Clone, Copy, Default)]
pub(super) enum Press {
    #[default]
    None,
    /// Went down at this point (overlay coordinates), and hasn't moved far enough to drag.
    Down(f64, f64),
    Dragging,
}

impl App {
    pub(super) fn on_pointer(&mut self, events: &[PointerEvent]) {
        for e in events {
            let pos = e.position; // relative to the overlay
            match e.kind {
                PointerEventKind::Press { button: BTN_LEFT, .. } => {
                    self.press = Press::Down(pos.0, pos.1);
                }
                PointerEventKind::Motion { .. } => self.pointer_moved(pos),
                PointerEventKind::Release { button: BTN_LEFT, .. } => self.pointer_released(),
                _ => {}
            }
        }
    }

    /// While the button is held, moving far enough turns the press into a drag.
    fn pointer_moved(&mut self, (x, y): (f64, f64)) {
        match self.press {
            Press::None => return,
            Press::Down(press_x, press_y) if (x - press_x).hypot(y - press_y) < DRAG_THRESHOLD => {
                return;
            }
            _ => self.press = Press::Dragging,
        }

        // Once the cursor is off her, COSMIC keeps measuring it from where the overlay was
        // when it left, not where we've moved it since, which sends her flying. So a cursor
        // that gets ahead of her drops her, and she has to be picked up again.
        let size = SIZE as f64;
        if !(0.0..size).contains(&x) || !(0.0..size).contains(&y) {
            self.cancel_press();
            return;
        }

        // pointer position is relative to the overlay; she needs screen coordinates
        let (Some((left, top)), Some(pet)) = (self.overlay_pos(), &mut self.pet) else {
            return;
        };
        pet.drag_to((left + x as i32, top + y as i32));
        // move now rather than next tick, so a fast cursor is less likely to get off her
        self.draw();
    }

    /// Letting go: after a drag she's dropped, otherwise it was a click and she's petted.
    fn pointer_released(&mut self) {
        let press = mem::take(&mut self.press);
        let Some(pet) = &mut self.pet else { return };
        match press {
            Press::Dragging => pet.release(),
            Press::Down(..) => pet.pet(),
            Press::None => {}
        }
    }

    /// Forgets the button being held (e.g. her overlay was replaced, so the compositor
    /// won't tell us when it's let go). If she was being dragged, she's dropped.
    pub(super) fn cancel_press(&mut self) {
        if matches!(mem::take(&mut self.press), Press::Dragging)
            && let Some(pet) = &mut self.pet
        {
            pet.release();
        }
    }

    /// Focus moved elsewhere (or the keyboard went away): forget held keys, otherwise
    /// she'd keep walking because we'd never see the key being released.
    pub(super) fn keyboard_focus_lost(&mut self) {
        self.keys = Keys::default();
    }

    /// Keyboard input (only arrives while she has focus, i.e. after clicking her).
    pub(super) fn on_key(&mut self, key: u32, down: bool) {
        match key {
            KEY_ESC if down => self.exit = true,
            KEY_W if down => {
                let screens = self.screen_rects();
                if let Some(pet) = &mut self.pet {
                    pet.wander(&screens);
                }
            }
            KEY_UP => self.keys.up = down,
            KEY_DOWN => self.keys.down = down,
            KEY_LEFT => self.keys.left = down,
            KEY_RIGHT => self.keys.right = down,
            _ => {}
        }
    }
}
