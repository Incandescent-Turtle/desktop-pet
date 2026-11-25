//! The app's state, and what happens each tick.

mod input;
mod overlay;
mod wayland;

use input::Press;
use overlay::Overlay;
use smithay_client_toolkit::{
    compositor::CompositorState,
    output::OutputState,
    reexports::client::{
        QueueHandle,
        globals::GlobalList,
        protocol::{wl_keyboard, wl_pointer},
    },
    registry::RegistryState,
    seat::SeatState,
    shell::wlr_layer::LayerShell,
    shm::{Shm, slot::SlotPool},
};

use crate::pet::{Keys, Pet, SIZE};
use crate::sprites::Sprites;

/// 10 minutes of ticks between random wander attempts, like the Java wander timer.
const WANDER_TICKS: u32 = 60_000;

pub struct App {
    // Wayland plumbing, handled in app/wayland.rs
    registry: RegistryState,
    outputs: OutputState,
    seats: SeatState,
    compositor: CompositorState,
    layer_shell: LayerShell,
    shm: Shm,
    pool: SlotPool,
    keyboard: Option<wl_keyboard::WlKeyboard>,
    pointer: Option<wl_pointer::WlPointer>,

    /// Her surface. `None` until the monitors are known (or after her monitor is unplugged).
    overlay: Option<Overlay>,
    /// `None` until the first tick, when we know where the middle of the screen is.
    pet: Option<Pet>,
    sprites: Sprites,
    wander_ticks: u32,

    // input state, handled in app/input.rs
    keys: Keys,
    press: Press,
    pub exit: bool,
}

impl App {
    pub fn new(globals: &GlobalList, qh: &QueueHandle<Self>) -> Self {
        let shm = Shm::bind(globals, qh).expect("wl_shm missing");
        App {
            registry: RegistryState::new(globals),
            outputs: OutputState::new(globals, qh),
            seats: SeatState::new(globals, qh),
            compositor: CompositorState::bind(globals, qh).expect("wl_compositor missing"),
            layer_shell: LayerShell::bind(globals, qh)
                .expect("this compositor doesn't support layer-shell (GNOME doesn't)"),
            pool: SlotPool::new((SIZE * SIZE * 4) as usize, &shm).expect("shm pool"),
            shm,
            keyboard: None,
            pointer: None,
            overlay: None,
            pet: None,
            sprites: Sprites::load(),
            wander_ticks: 0,
            keys: Keys::default(),
            press: Press::default(),
            exit: false,
        }
    }

    pub fn tick(&mut self, qh: &QueueHandle<Self>) {
        let screens = self.screen_rects();
        let Some(first) = screens.first() else { return };

        let pet = self.pet.get_or_insert_with(|| {
            Pet::new((first.x + (first.w - SIZE) / 2, first.y + (first.h - SIZE) / 2))
        });

        self.wander_ticks += 1;
        if self.wander_ticks >= WANDER_TICKS {
            self.wander_ticks = 0;
            pet.maybe_wander(&screens);
        }
        pet.tick(&self.keys, &screens);

        self.follow_pet(qh);
        self.draw();
    }
}
