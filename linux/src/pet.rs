//! Pet's actions
//!
//! Always has one`Action` active
//! Each tick she decides her next action based on current action
//! keys > wandering > settling down after walking > idle animations finishing, then moves if it's a walk
//! Animation frames are derived from how many ticks she's been in the current action
//!
//! Timing: 1 tick = 10ms, so 100 ticks = 1 second

/// Her size on screen in logical pixels (the 32x32 sprites are scaled up to this).
pub const SIZE: i32 = 100;
pub const BUBBLE_SIZE: i32 = 30;

/// Both bubbles (heart and zzz) are 4-frame animations.
const BUBBLE_FRAMES: u32 = 4;
const HEART_FRAME_TICKS: u32 = 50;
const ZZZ_FRAME_TICKS: u32 = 30;
/// Heart plays through once (200 ticks = 2s).
/// Petting counted seperate from ticks, to not interrupt action
const HEART_TICKS: u32 = BUBBLE_FRAMES * HEART_FRAME_TICKS;
/// Laying's animation ends at tick 60; she holds that pose another 60 before dozing off.
const LAYING_TICKS: u32 = 120;
/// One play of the sitting animation (4 frames x 20 ticks), then she starts licking.
const SITTING_TICKS: u32 = 80;

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
    fn animation(self) -> (usize, u32) {
        match self {
            Action::Up | Action::Down | Action::Left | Action::Right => (4, 10),
            Action::Laying | Action::Sitting => (4, 20),
            Action::Licking => (4, 40),
            Action::Curled | Action::Held => (2, 40),
            Action::Sleep => (1, 10),
        }
    }

    fn is_walking(self) -> bool {
        matches!(self, Action::Up | Action::Down | Action::Left | Action::Right)
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
    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }

    fn union(&self, o: &Rect) -> Rect {
        let (x, y) = (self.x.min(o.x), self.y.min(o.y));
        let w = (self.x + self.w).max(o.x + o.w) - x;
        let h = (self.y + self.h).max(o.y + o.h) - y;
        Rect { x, y, w, h }
    }
}

#[derive(Clone, Copy, Default)]
pub struct Keys {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub w: bool,
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
    /// Ticks since she was petted, while the heart is showing.
    heart: Option<u32>,
}

impl Pet {
    pub fn new(pos: (i32, i32)) -> Self {
        Pet {
            pos,
            action: Action::Curled,
            ticks: 0,
            facing_left: false,
            wander_target: None,
            heart: None,
        }
    }

    pub fn tick(&mut self, keys: &Keys, screens: &[Rect]) {
        self.ticks += 1;
        if let Some(t) = &mut self.heart {
            *t += 1;
            if *t >= HEART_TICKS {
                self.heart = None;
            }
        }
        if keys.w {
            self.wander(screens);
        }
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
    /// For weird monitor layours, if horizontal is blocked, try vertical until we can move horitontal again.
    fn direction_to(&self, (tx, ty): (i32, i32), screens: &[Rect]) -> Option<Action> {
        let (x, y) = self.pos;
        let horizontal = if (x - tx).abs() < 3 {
            None
        } else if x > tx {
            Some(Action::Left)
        } else {
            Some(Action::Right)
        };
        let vertical = if (y - ty).abs() < 3 {
            None
        } else if y > ty {
            Some(Action::Up)
        } else {
            Some(Action::Down)
        };
        match (horizontal, vertical) {
            // If horizontal is blocked, try vertical until we can move horizontal again.
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
        let (dx, dy) = match dir {
            Action::Right => (1, 0),
            Action::Left => (-1, 0),
            Action::Up => (0, -1),
            Action::Down => (0, 1),
            _ => (0, 0),
        };
        // Rectangle created containing all monitors.
        let all = screens.iter().copied().reduce(|a, s| a.union(&s))?;
        let x = (self.pos.0 + dx).clamp(all.x - 10, all.x + all.w - SIZE);
        let y = (self.pos.1 + dy).clamp(all.y - 35, all.y + all.h - SIZE);
        let centre_on_a_monitor = screens.iter().any(|s| s.contains(x + SIZE / 2, y + SIZE / 2));
        if centre_on_a_monitor { Some((x, y)) } else { None }
    }

    pub fn maybe_wander(&mut self, screens: &[Rect]) {
        if fastrand::bool() {
            self.wander(screens);
        }
    }

    /// Pick a random point on a random monitor, preferably more than 400px away.
    fn wander(&mut self, screens: &[Rect]) {
        let mut target = self.pos;
        for _ in 0..100 {
            let Some(s) = fastrand::choice(screens) else { return };
            target = (
                s.x + 10 + fastrand::i32(0..(s.w - SIZE - 20).max(1)),
                s.y + 10 + fastrand::i32(0..(s.h - SIZE - 20).max(1)),
            );
            if (self.pos.0 - target.0).abs() > 400 || (self.pos.1 - target.1).abs() > 400 {
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
        self.pos = (cursor.0 - SIZE / 2, cursor.1 - SIZE / 2);
        self.set_action(Action::Held);
    }

    /// Let go after dragging: she flops down. Starts from laying frame 2 (starting crouched)
    pub fn release(&mut self) {
        if self.action == Action::Held {
            self.set_action(Action::Laying);
            self.ticks = Action::Laying.animation().1;
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
