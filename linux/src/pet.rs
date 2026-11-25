//! What the pet does. Runs at 100 ticks per second, same as the Java version.
//!
//! She always has one `Action`. Each tick she decides her next action (keys > wandering >
//! settling down after walking > idle animations finishing), then moves if it's a walk.
//! Animation frames are derived from how many ticks she's been in the current action.

/// Her size on screen in logical pixels (the 32x32 sprites are scaled up to this).
pub const SIZE: i32 = 100;
pub const BUBBLE_SIZE: i32 = 30;

/// How long the heart bubble shows after being petted (4 frames x 50 ticks).
const HEART_TICKS: u32 = 200;
/// Laying plays its 4 frames, then holds the last one before she dozes off.
const LAYING_TICKS: u32 = 121;
/// Sitting plays its 4 frames once, then she starts licking.
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
    Rising,
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
        Action::Rising,
        Action::Sleep,
    ];

    /// (frame count, ticks per frame)
    fn animation(self) -> (usize, u32) {
        match self {
            Action::Up | Action::Down | Action::Left | Action::Right => (4, 10),
            Action::Laying | Action::Sitting => (4, 20),
            Action::Licking => (4, 40),
            Action::Curled | Action::Rising => (2, 40),
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
        if self.action != Action::Rising {
            let next = self.next_action(keys);
            self.set_action(next);
        }
        self.walk(screens);
    }

    fn next_action(&mut self, keys: &Keys) -> Action {
        if let Some(dir) = keys.direction() {
            return dir;
        }
        if let Some(target) = self.wander_target {
            match self.direction_to(target) {
                Some(dir) => return dir,
                None => self.wander_target = None, // arrived
            }
        }
        match self.action {
            // stopped walking: lie down (2/3) or sit (1/3)
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
    fn direction_to(&self, (tx, ty): (i32, i32)) -> Option<Action> {
        let (x, y) = self.pos;
        if (x - tx).abs() >= 3 {
            Some(if x > tx { Action::Left } else { Action::Right })
        } else if (y - ty).abs() >= 3 {
            Some(if y > ty { Action::Up } else { Action::Down })
        } else {
            None
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

    /// Moves 1px if walking, staying on screen and out of gaps between monitors.
    fn walk(&mut self, screens: &[Rect]) {
        let (dx, dy) = match self.action {
            Action::Right => (1, 0),
            Action::Left => (-1, 0),
            Action::Up => (0, -1),
            Action::Down => (0, 1),
            _ => return,
        };
        let Some(all) = screens.iter().copied().reduce(|a, s| a.union(&s)) else { return };
        // same margins as the Java version: the sprite has transparent padding
        let x = (self.pos.0 + dx).clamp(all.x - 10, all.x + all.w - SIZE);
        let y = (self.pos.1 + dy).clamp(all.y - 35, all.y + all.h - SIZE);
        if screens.iter().any(|s| s.contains(x + SIZE / 2, y + SIZE / 2)) {
            self.pos = (x, y);
        } else {
            self.wander_target = None; // can't get there
        }
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
        self.set_action(Action::Rising);
    }

    /// Let go after dragging: she flops down, skipping the first laying frame.
    pub fn release(&mut self) {
        if self.action == Action::Rising {
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
            Some((Bubble::Heart, (t / 50) as usize))
        } else if matches!(self.action, Action::Curled | Action::Sleep) {
            Some((Bubble::Zzz, (self.ticks / 30 % 4) as usize))
        } else {
            None
        }
    }

    /// Whether to mirror the sprite (only some animations have a left/right version).
    pub fn flipped(&self) -> bool {
        match self.action {
            Action::Laying | Action::Rising | Action::Sleep => self.facing_left,
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
