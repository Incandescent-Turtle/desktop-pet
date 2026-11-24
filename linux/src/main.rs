//! Desktop pet for Wayland compositors with layer-shell (COSMIC, KDE, Sway, Hyprland, ...).
//!
//! She lives in a small 100x100 overlay surface. Walking moves the surface (via its margins);
//! crossing onto another monitor recreates the surface there. Wayland plumbing is in wayland.rs.

mod pet;
mod sprites;
mod wayland;

use std::time::Duration;

use pet::{Keys, Pet, Rect, SIZE};
use smithay_client_toolkit::{
    compositor::CompositorState,
    output::OutputState,
    reexports::{
        calloop::{
            EventLoop,
            timer::{TimeoutAction, Timer},
        },
        calloop_wayland_source::WaylandSource,
        client::{
            Connection, QueueHandle,
            globals::registry_queue_init,
            protocol::{wl_keyboard, wl_output, wl_pointer, wl_shm},
        },
    },
    registry::RegistryState,
    seat::{
        SeatState,
        pointer::{BTN_LEFT, PointerEvent, PointerEventKind},
    },
    shell::{
        WaylandSurface,
        wlr_layer::{Anchor, KeyboardInteractivity, Layer, LayerShell, LayerSurface},
    },
    shm::{
        Shm,
        slot::{Buffer, SlotPool},
    },
};
use sprites::Sprites;

const TICK: Duration = Duration::from_millis(10);
/// 10 minutes of ticks between random wander attempts, like the Java wander timer.
const WANDER_TICKS: u32 = 60_000;
/// Pointer movement (px) before a press counts as a drag instead of a click.
const DRAG_THRESHOLD: f64 = 3.0;

// Linux evdev key codes, which is what wl_keyboard sends
const KEY_ESC: u32 = 1;
const KEY_W: u32 = 17;
const KEY_UP: u32 = 103;
const KEY_LEFT: u32 = 105;
const KEY_RIGHT: u32 = 106;
const KEY_DOWN: u32 = 108;

struct Screen {
    output: wl_output::WlOutput,
    rect: Rect,
    scale: i32,
}

/// The surface she's drawn in, on one monitor.
struct Overlay {
    layer: LayerSurface,
    screen: Screen,
    configured: bool,
}

/// Everything that changes what she looks like; used to skip redundant redraws.
type Look = ((i32, i32), pet::Action, usize, bool, Option<(pet::Bubble, usize)>, i32);

struct App {
    // Wayland plumbing, handled in wayland.rs
    registry: RegistryState,
    outputs: OutputState,
    seats: SeatState,
    compositor: CompositorState,
    layer_shell: LayerShell,
    shm: Shm,
    pool: SlotPool,
    buffer: Option<Buffer>,
    keyboard: Option<wl_keyboard::WlKeyboard>,
    pointer: Option<wl_pointer::WlPointer>,

    overlay: Option<Overlay>,
    last_look: Option<Look>,
    pet: Option<Pet>,
    sprites: Sprites,
    keys: Keys,
    wander_ticks: u32,
    press: Option<(f64, f64)>,
    dragging: bool,
    exit: bool,
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

    /// Runs every 10ms, like the Java main timer.
    fn tick(&mut self, qh: &QueueHandle<Self>) {
        let screens = self.screens();
        let rects: Vec<Rect> = screens.iter().map(|s| s.rect).collect();
        let Some(first) = rects.first() else { return };

        // start in the middle of the first monitor
        let pet = self.pet.get_or_insert_with(|| {
            Pet::new((first.x + (first.w - SIZE) / 2, first.y + (first.h - SIZE) / 2))
        });

        self.wander_ticks += 1;
        if self.wander_ticks >= WANDER_TICKS {
            self.wander_ticks = 0;
            pet.maybe_wander(&rects);
        }
        if !self.dragging {
            pet.tick(&self.keys, &rects);
        }

        // keep the overlay on whichever monitor her centre is on
        let (cx, cy) = (pet.pos.0 + SIZE / 2, pet.pos.1 + SIZE / 2);
        if let Some(screen) = screens.into_iter().find(|s| s.rect.contains(cx, cy)) {
            match &mut self.overlay {
                Some(o) if o.screen.output == screen.output => o.screen = screen,
                _ => self.create_overlay(qh, screen),
            }
        }

        self.draw();
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

        // replacing the old overlay destroys it
        self.overlay = Some(Overlay { layer, screen, configured: false });
        self.last_look = None;
        self.press = None;
        self.dragging = false;
    }

    fn draw(&mut self) {
        let (Some(o), Some(pet)) = (&self.overlay, &self.pet) else {
            return;
        };
        let look = (pet.pos, pet.action, pet.frame(), pet.flipped(), pet.bubble(), o.screen.scale);
        if !o.configured || self.last_look == Some(look) {
            return;
        }

        let size = SIZE * o.screen.scale;
        let (buffer, canvas) = self
            .pool
            .create_buffer(size, size, size * 4, wl_shm::Format::Argb8888)
            .expect("failed to create buffer");
        self.sprites.render(canvas, size as usize, pet);

        let surface = o.layer.wl_surface();
        let (left, top) = (pet.pos.0 - o.screen.rect.x, pet.pos.1 - o.screen.rect.y);
        o.layer.set_margin(top, 0, 0, left);
        surface.set_buffer_scale(o.screen.scale);
        surface.damage_buffer(0, 0, size, size);
        buffer.attach_to(surface).expect("failed to attach buffer");
        o.layer.commit();

        self.buffer = Some(buffer); // keep it alive while the compositor shows it
        self.last_look = Some(look);
    }

    fn on_pointer(&mut self, events: &[PointerEvent]) {
        let Some(pet) = &mut self.pet else { return };
        // where the overlay actually is on screen right now
        let origin = self.last_look.map_or(pet.pos, |look| look.0);
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

    fn on_key(&mut self, key: u32, down: bool) {
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

fn main() {
    let conn = Connection::connect_to_env().expect("not running under Wayland");
    let (globals, event_queue) = registry_queue_init(&conn).expect("wayland registry");
    let qh = event_queue.handle();
    let mut event_loop: EventLoop<App> = EventLoop::try_new().expect("event loop");
    WaylandSource::new(conn, event_queue)
        .insert(event_loop.handle())
        .expect("wayland event source");

    let shm = Shm::bind(&globals, &qh).expect("wl_shm missing");
    let mut app = App {
        registry: RegistryState::new(&globals),
        outputs: OutputState::new(&globals, &qh),
        seats: SeatState::new(&globals, &qh),
        compositor: CompositorState::bind(&globals, &qh).expect("wl_compositor missing"),
        layer_shell: LayerShell::bind(&globals, &qh)
            .expect("this compositor doesn't support layer-shell (GNOME doesn't)"),
        pool: SlotPool::new((SIZE * SIZE * 4) as usize, &shm).expect("shm pool"),
        shm,
        buffer: None,
        keyboard: None,
        pointer: None,
        overlay: None,
        last_look: None,
        pet: None,
        sprites: Sprites::load(),
        keys: Keys::default(),
        wander_ticks: 0,
        press: None,
        dragging: false,
        exit: false,
    };

    event_loop
        .handle()
        .insert_source(Timer::immediate(), move |_, _, app: &mut App| {
            app.tick(&qh);
            TimeoutAction::ToDuration(TICK)
        })
        .expect("timer");

    while !app.exit {
        event_loop.dispatch(None, &mut app).expect("event loop");
    }
}
