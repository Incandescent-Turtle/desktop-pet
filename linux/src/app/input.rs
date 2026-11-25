//! Mouse and keyboard input.

use smithay_client_toolkit::seat::pointer::{BTN_LEFT, PointerEvent, PointerEventKind};

use super::App;

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
        let overlay_pos = self.overlay_pos();
        let Some(pet) = &mut self.pet else { return };
        let origin = overlay_pos.unwrap_or(pet.pos);
        for e in events {
            let (x, y) = e.position; // relative to the overlay
            match e.kind {
                PointerEventKind::Press { button: BTN_LEFT, .. } => self.press = Some((x, y)),
                PointerEventKind::Motion { .. } => {
                    let Some((px, py)) = self.press else { continue };
                    if self.dragging || (x - px).hypot(y - py) >= DRAG_THRESHOLD {
                        self.dragging = true;
                        pet.drag_to((origin.0 + x as i32, origin.1 + y as i32));
                    }
                }
                PointerEventKind::Release { button: BTN_LEFT, .. } => {
                    if self.dragging {
                        pet.release();
                    } else if self.press.is_some() {
                        pet.pet();
                    }
                    self.press = None;
                    self.dragging = false;
                }
                _ => {}
            }
        }
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
