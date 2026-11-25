//! Pet's actions
//!
//! Always has one `Action` active.
//! Each tick she decides her next action based on the current one:
//! keys > wandering > settling down after walking > idle animations finishing, then moves if it's a walk.
//! Animation frames are derived from how many ticks she's been in the current action.
//!
//! Timing: 1 tick = 10ms, so 100 ticks = 1 second

/// Her size on screen in logical pixels (the 32x32 sprites are scaled up to this).
pub const SIZE: i32 = 100;
pub const BUBBLE_SIZE: i32 = 30;

/// Both bubbles (heart and zzz) are 4-frame animations.
pub const BUBBLE_FRAMES: u32 = 4;
const HEART_FRAME_TICKS: u32 = 50;
const ZZZ_FRAME_TICKS: u32 = 30;
/// Heart plays through once (200 ticks = 2s).
const HEART_TICKS: u32 = BUBBLE_FRAMES * HEART_FRAME_TICKS;
/// Laying's animation ends at tick 60; she holds that pose another 60 before dozing off.
const LAYING_TICKS: u32 = 120;
/// One play of the sitting animation, then she starts licking.
const SITTING_TICKS: u32 = Action::Sitting.play_ticks();

/// 10 minutes between random wander attempts, like the Java wander timer.
const WANDER_TICKS: u32 = 60_000;
/// Wander targets are preferably further than this away (on either axis).
const WANDER_MIN_DISTANCE: i32 = 400;
/// Wander targets stay this far inside a monitor's edges.
const WANDER_EDGE_MARGIN: i32 = 10;
/// Within this distance of a wander target (on each axis) counts as arrived.
const ARRIVED_DISTANCE: i32 = 3;

/// How far she may go past the left and top edges of the screens (the Java version's limits).
const OFFSCREEN_LEFT: i32 = 10;
const OFFSCREEN_TOP: i32 = 35;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Action {
    Up,
    Down,
    Left,
    Right,
    Curled,
    Laying,
    Sitting,
    Licking,
    Held,
    Sleep,
}

impl Action {
    /// Every action, in declaration order, so `action as usize` indexes into it.
    pub const ALL: [Action; 10] = [
        Action::Up,
        Action::Down,
        Action::Left,
        Action::Right,
        Action::Curled,
        Action::Laying,
        Action::Sitting,
        Action::Licking,
        Action::Held,
        Action::Sleep,
    ];

    /// (frame count, ticks per frame)
    pub const fn animation(self) -> (usize, u32) {
        match self {
            Action::Up | Action::Down | Action::Left | Action::Right => (4, 10),
            Action::Laying | Action::Sitting => (4, 20),
            Action::Licking => (4, 40),
            Action::Curled => (2, 40),
            Action::Held | Action::Sleep => (1, 10),
        }
    }

    /// Ticks until `frame` (counting from 0) starts.
    const fn frame_start(self, frame: u32) -> u32 {
        frame * self.animation().1
    }

    /// Ticks for one play of the whole animation.
    const fn play_ticks(self) -> u32 {
        self.frame_start(self.animation().0 as u32)
    }

    /// The 1px step each tick of a walk takes, or `None` if this isn't a walk.
    fn step_delta(self) -> Option<(i32, i32)> {
        match self {
            Action::Up => Some((0, -1)),
            Action::Down => Some((0, 1)),
            Action::Left => Some((-1, 0)),
            Action::Right => Some((1, 0)),
            _ => None,
        }
    }

    fn is_walking(self) -> bool {
        self.step_delta().is_some()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Bubble {
    Zzz,
    Heart,
}

/// A monitor's area in global logical coordinates.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub fn contains(&self, (x, y): (i32, i32)) -> bool {
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }

    fn union(&self, o: &Rect) -> Rect {
        let (x, y) = (self.x.min(o.x), self.y.min(o.y));
        let w = (self.x + self.w).max(o.x + o.w) - x;
        let h = (self.y + self.h).max(o.y + o.h) - y;
        Rect { x, y, w, h }
    }

    fn centre(&self) -> (i32, i32) {
        (self.x + self.w / 2, self.y + self.h / 2)
    }
}

/// The centre of her square when its top-left corner is at `pos`.
fn centre_of((x, y): (i32, i32)) -> (i32, i32) {
    (x + SIZE / 2, y + SIZE / 2)
}

/// The top-left corner of her square when it's centred on `point`.
fn top_left_at((x, y): (i32, i32)) -> (i32, i32) {
    (x - SIZE / 2, y - SIZE / 2)
}

#[derive(Clone, Copy, Default)]
pub struct Keys {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
}

impl Keys {
    fn direction(&self) -> Option<Action> {
        if self.up {
            Some(Action::Up)
        } else if self.down {
            Some(Action::Down)
        } else if self.left {
            Some(Action::Left)
        } else if self.right {
            Some(Action::Right)
        } else {
            None
        }
    }
}

pub struct Pet {
    /// Top-left corner in global logical coordinates.
    pub pos: (i32, i32),
    pub action: Action,
    /// Ticks since the current action started.
    ticks: u32,
    facing_left: bool,
    wander_target: Option<(i32, i32)>,
    /// Ticks until the next random wander attempt.
    wander_ticks: u32,
    /// Ticks since she was petted, while the heart is showing. Counted separately from
    /// `ticks`, so petting doesn't interrupt what she's doing.
    heart: Option<u32>,
}

impl Pet {
    /// A new pet, curled up in the middle of `screen`.
    pub fn new(screen: &Rect) -> Self {
        Pet {
            pos: top_left_at(screen.centre()),
            action: Action::Curled,
            ticks: 0,
            facing_left: false,
            wander_target: None,
            wander_ticks: 0,
            heart: None,
        }
    }

    pub fn centre(&self) -> (i32, i32) {
        centre_of(self.pos)
    }

    pub fn tick(&mut self, keys: &Keys, screens: &[Rect]) {
        self.ticks += 1;
        if let Some(t) = &mut self.heart {
            *t += 1;
            if *t >= HEART_TICKS {
                self.heart = None;
            }
        }
        self.wander_ticks += 1;
        if self.wander_ticks >= WANDER_TICKS {
            self.wander_ticks = 0;
            if fastrand::bool() {
                self.wander(screens);
            }
        }
        // while she's held, only the drag moves her
        if self.action != Action::Held {
            let next = self.next_action(keys, screens);
            self.set_action(next);
        }
        self.walk(screens);
    }

    fn next_action(&mut self, keys: &Keys, screens: &[Rect]) -> Action {
        if let Some(dir) = keys.direction() {
            return dir;
        }
        if let Some(target) = self.wander_target {
            match self.direction_to(target, screens) {
                Some(dir) => return dir,
                None => self.wander_target = None, // arrived
            }
        }
        match self.action {
            // walking state but no key pressed and no target: lie down (2/3) or sit (1/3)
            a if a.is_walking() => {
                if fastrand::u32(0..3) > 0 {
                    Action::Laying
                } else {
                    Action::Sitting
                }
            }
            Action::Laying if self.ticks >= LAYING_TICKS => {
                if fastrand::bool() {
                    Action::Curled
                } else {
                    Action::Sleep
                }
            }
            Action::Sitting if self.ticks >= SITTING_TICKS => Action::Licking,
            a => a,
        }
    }

    /// Walk horizontally first, then vertically. `None` once she's there.
    fn direction_to(&self, (tx, ty): (i32, i32), screens: &[Rect]) -> Option<Action> {
        let (x, y) = self.pos;
        let horizontal = if (x - tx).abs() < ARRIVED_DISTANCE {
            None
        } else if x > tx {
            Some(Action::Left)
        } else {
            Some(Action::Right)
        };
        let vertical = if (y - ty).abs() < ARRIVED_DISTANCE {
            None
        } else if y > ty {
            Some(Action::Up)
        } else {
            Some(Action::Down)
        };
        match (horizontal, vertical) {
            // If horizontal is blocked (a gap between monitors), go vertical until it isn't.
            (Some(h), Some(v)) if self.step(h, screens).is_none() => Some(v),
            (Some(h), _) => Some(h),
            (None, v) => v,
        }
    }

    fn set_action(&mut self, action: Action) {
        if action != self.action {
            self.action = action;
            self.ticks = 0;
        }
        match action {
            Action::Left => self.facing_left = true,
            Action::Right => self.facing_left = false,
            _ => {}
        }
    }

    fn walk(&mut self, screens: &[Rect]) {
        if !self.action.is_walking() {
            return;
        }
        match self.step(self.action, screens) {
            Some(pos) => self.pos = pos,
            None => self.wander_target = None, // can't get there
        }
    }

    /// Where a 1px step in `dir` puts her, kept on screen. `None` if her centre would end up
    /// in a gap between monitors (the combined rectangle isn't all covered by monitors).
    fn step(&self, dir: Action, screens: &[Rect]) -> Option<(i32, i32)> {
        let (dx, dy) = dir.step_delta()?;
        // the smallest rectangle containing all monitors
        let all = screens.iter().copied().reduce(|a, s| a.union(&s))?;
        let x = (self.pos.0 + dx).clamp(all.x - OFFSCREEN_LEFT, all.x + all.w - SIZE);
        let y = (self.pos.1 + dy).clamp(all.y - OFFSCREEN_TOP, all.y + all.h - SIZE);
        let centre_on_a_monitor = screens.iter().any(|s| s.contains(centre_of((x, y))));
        if centre_on_a_monitor { Some((x, y)) } else { None }
    }

    /// Pick a random point on a random monitor, preferably far away.
    pub fn wander(&mut self, screens: &[Rect]) {
        let m = WANDER_EDGE_MARGIN;
        let mut target = self.pos;
        for _ in 0..100 {
            let Some(s) = fastrand::choice(screens) else { return };
            target = (
                s.x + m + fastrand::i32(0..(s.w - SIZE - 2 * m).max(1)),
                s.y + m + fastrand::i32(0..(s.h - SIZE - 2 * m).max(1)),
            );
            let (dx, dy) = (self.pos.0 - target.0, self.pos.1 - target.1);
            if dx.abs() > WANDER_MIN_DISTANCE || dy.abs() > WANDER_MIN_DISTANCE {
                break;
            }
        }
        self.wander_target = Some(target);
    }

    pub fn pet(&mut self) {
        self.heart = Some(0);
    }

    /// Being dragged: the cursor (global coordinates) holds her by the middle.
    pub fn drag_to(&mut self, cursor: (i32, i32)) {
        self.pos = top_left_at(cursor);
        self.set_action(Action::Held);
    }

    /// Let go after dragging: she flops down, starting crouched (laying's second frame).
    pub fn put_down(&mut self) {
        if self.action == Action::Held {
            self.set_action(Action::Laying);
            self.ticks = Action::Laying.frame_start(1);
        }
    }

    pub fn frame(&self) -> usize {
        let (frames, ticks_per_frame) = self.action.animation();
        let n = (self.ticks / ticks_per_frame) as usize;
        match self.action {
            // play once and hold the last frame
            Action::Laying | Action::Sitting => n.min(frames - 1),
            _ => n % frames,
        }
    }

    /// The bubble above her and its animation frame, if any.
    pub fn bubble(&self) -> Option<(Bubble, usize)> {
        if let Some(t) = self.heart {
            // Heart animation plays through once, then disappears
            Some((Bubble::Heart, (t / HEART_FRAME_TICKS) as usize))
        } else if matches!(self.action, Action::Curled | Action::Sleep) {
            // Zzz animation loops
            Some((Bubble::Zzz, (self.ticks / ZZZ_FRAME_TICKS % BUBBLE_FRAMES) as usize))
        } else {
            None
        }
    }

    /// Whether to mirror the sprite (only some animations have a left/right version).
    pub fn flipped(&self) -> bool {
        match self.action {
            Action::Laying | Action::Held | Action::Sleep => self.facing_left,
            Action::Curled => !self.facing_left,
            _ => false,
        }
    }

    /// Where the bubble's top-left corner goes inside her square, next to her head.
    pub fn bubble_offset(&self) -> (i32, i32) {
        match self.action {
            Action::Right => (70, 40), // her head sticks out further when walking right
            Action::Sleep | Action::Laying | Action::Left => {
                (if self.facing_left { 0 } else { 60 }, 40)
            }
            Action::Up | Action::Licking | Action::Sitting => (30, 15),
            _ => (30, 40),
        }
    }
}
