//! Mouse and keyboard input.

use smithay_client_toolkit::seat::pointer::{BTN_LEFT, PointerEvent, PointerEventKind};

use super::App;
use crate::pet::Keys;

/// Pointer movement (px) before a press counts as a drag instead of a click.
const DRAG_THRESHOLD: f64 = 3.0;

// Linux evdev key codes, which is what wl_keyboard sends
const KEY_ESC: u32 = 1;
const KEY_W: u32 = 17;
const KEY_UP: u32 = 103;
const KEY_LEFT: u32 = 105;
const KEY_RIGHT: u32 = 106;
const KEY_DOWN: u32 = 108;

impl App {
    pub(super) fn on_pointer(&mut self, events: &[PointerEvent]) {
        for e in events {
            let pos = e.position; // relative to the overlay
            match e.kind {
                PointerEventKind::Press { button: BTN_LEFT, .. } => self.press = Some(pos),
                PointerEventKind::Motion { .. } => self.pointer_moved(pos),
                PointerEventKind::Release { button: BTN_LEFT, .. } => self.pointer_released(),
                _ => {}
            }
        }
    }

    /// While the button is held, moving far enough turns the press into a drag.
    fn pointer_moved(&mut self, (x, y): (f64, f64)) {
        let Some((press_x, press_y)) = self.press else { return };
        let distance_moved = (x - press_x).hypot(y - press_y);
        if !self.dragging && distance_moved < DRAG_THRESHOLD {
            return;
        }
        self.dragging = true;

        // pointer position is relative to the overlay; she needs screen coordinates
        let (Some((left, top)), Some(pet)) = (self.overlay_pos(), &mut self.pet) else {
            return;
        };
        pet.drag_to((left + x as i32, top + y as i32));
    }

    /// Letting go: after a drag she's dropped, otherwise it was a click and she's petted.
    fn pointer_released(&mut self) {
        if let Some(pet) = &mut self.pet {
            if self.dragging {
                pet.release();
            } else if self.press.is_some() {
                pet.pet();
            }
        }
        self.press = None;
        self.dragging = false;
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
            KEY_W => self.keys.w = down,
            KEY_UP => self.keys.up = down,
            KEY_DOWN => self.keys.down = down,
            KEY_LEFT => self.keys.left = down,
            KEY_RIGHT => self.keys.right = down,
            _ => {}
        }
    }
}
