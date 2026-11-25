//! Desktop pet for Wayland compositors with layer-shell (COSMIC, KDE, Sway, Hyprland, ...).
//!
//! She lives in a small 100x100 layer-shell "overlay" surface that sits above all windows.
//! Walking moves the surface (via its margins); crossing onto another monitor recreates the
//! surface there. Clicking her gives her keyboard focus, so the arrow keys work.
//!
//! - `main.rs`: this file, starts the app and runs the 10ms timer
//! - `pet.rs`: her behaviour
//! - `sprites.rs`: the sprites and drawing them
//! - `app.rs`: the app's state and what happens each tick
//!   - `app/overlay.rs`: her on-screen surface
//!   - `app/input.rs`: mouse and keyboard
//!   - `app/wayland.rs`: event-handler boilerplate that forwards to methods in the files above

mod app;
mod pet;
mod sprites;

use std::time::Duration;

use app::App;
use smithay_client_toolkit::reexports::{
    calloop::{
        EventLoop,
        timer::{TimeoutAction, Timer},
    },
    calloop_wayland_source::WaylandSource,
    client::{Connection, globals::registry_queue_init},
};

const TICK: Duration = Duration::from_millis(10);

fn main() {
    let conn = Connection::connect_to_env().expect("not running under Wayland");
    let (globals, event_queue) = registry_queue_init(&conn).expect("wayland registry");
    let qh = event_queue.handle();
    let mut event_loop: EventLoop<App> = EventLoop::try_new().expect("event loop");
    WaylandSource::new(conn, event_queue)
        .insert(event_loop.handle())
        .expect("wayland event source");

    let mut app = App::new(&globals, &qh);

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
