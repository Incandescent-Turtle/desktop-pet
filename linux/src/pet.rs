//! Pet behaviour, ported from Main.java. Runs at 100 ticks per second like the Java timer.

pub const SIZE: i32 = 100;

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
    pub fn frame_count(self) -> usize {
        match self {
            Action::Curled | Action::Rising => 2,
            Action::Sleep => 1,
            _ => 4,
        }
    }

    fn delay(self) -> u32 {
        match self {
            Action::Curled | Action::Licking | Action::Rising => 40,
            Action::Laying | Action::Sitting => 20,
            _ => 10,
        }
    }

    fn is_moving(self) -> bool {
        matches!(self, Action::Up | Action::Down | Action::Left | Action::Right)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Bubble {
    None,
    Zzz,
    Heart,
}

impl Bubble {
    fn delay(self) -> u32 {
        match self {
            Bubble::None => 0,
            Bubble::Zzz => 30,
            Bubble::Heart => 50,
        }
    }

    fn frame_count(self) -> usize {
        match self {
            Bubble::None => 0,
            _ => 4,
        }
    }
}

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
        let x = self.x.min(o.x);
        let y = self.y.min(o.y);
        Rect {
            x,
            y,
            w: (self.x + self.w).max(o.x + o.w) - x,
            h: (self.y + self.h).max(o.y + o.h) - y,
        }
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
    fn any_move(&self) -> bool {
        self.up || self.down || self.left || self.right
    }
}

pub struct Pet {
    /// Top-left corner in global logical coordinates.
    pub pos: (i32, i32),
    pub action: Action,
    pub frame: usize,
    anim_steps: u32,
    pub facing_left: bool,
    wandering: bool,
    wander_target: (i32, i32),
    pub bubble: Bubble,
    pub bubble_frame: usize,
    bubble_steps: u32,
}

impl Pet {
    pub fn new(pos: (i32, i32)) -> Self {
        Pet {
            pos,
            action: Action::Curled,
            frame: 0,
            anim_steps: 0,
            facing_left: false,
            wandering: false,
            wander_target: (0, 0),
            bubble: Bubble::None,
            bubble_frame: 0,
            bubble_steps: 0,
        }
    }

    pub fn tick(&mut self, keys: &Keys, screens: &[Rect]) {
        self.update_action(keys);
        self.do_action(screens);
        self.update_animation();
        self.manage_bubble();
        if keys.w {
            self.try_wander(true, screens);
        }
    }

    /// Returns whether the action changed.
    fn change(&mut self, a: Action) -> bool {
        if a != self.action {
            self.action = a;
            return true;
        }
        false
    }

    fn update_action(&mut self, keys: &Keys) {
        if self.action == Action::Rising {
            return;
        }
        let (x, y) = self.pos;
        if self.wandering && !keys.any_move() {
            let (tx, ty) = self.wander_target;
            if (x - tx).abs() >= 3 {
                self.change(if x > tx { Action::Left } else { Action::Right });
            } else {
                self.change(if y > ty { Action::Up } else { Action::Down });
            }
            let (dx, dy) = ((x - tx) as f64, (y - ty) as f64);
            if (dx * dx + dy * dy).sqrt() < 3.0 {
                self.wandering = false;
            }
        }
        let changed = if keys.up {
            self.change(Action::Up)
        } else if keys.down {
            self.change(Action::Down)
        } else if keys.left {
            self.change(Action::Left)
        } else if keys.right {
            self.change(Action::Right)
        } else if self.action.is_moving() {
            if self.action == Action::Left {
                self.facing_left = true;
            } else if self.action == Action::Right {
                self.facing_left = false;
            }
            if self.wandering {
                false
            } else if fastrand::u32(0..3) >= 1 {
                self.change(Action::Laying)
            } else {
                self.change(Action::Sitting)
            }
        } else {
            false
        };
        if changed {
            self.frame = 0;
        }
    }

    fn do_action(&mut self, screens: &[Rect]) {
        let (dx, dy) = match self.action {
            Action::Right => (1, 0),
            Action::Left => (-1, 0),
            Action::Up => (0, -1),
            Action::Down => (0, 1),
            _ => return,
        };
        let Some(first) = screens.first() else { return };
        let all = screens.iter().fold(*first, |a, s| a.union(s));
        // same margins as the Java version: the sprite has transparent padding
        let x = (self.pos.0 + dx).clamp(all.x - 10, all.x + all.w - SIZE);
        let y = (self.pos.1 + dy).clamp(all.y - 35, all.y + all.h - SIZE);
        // don't walk into gaps between monitors of different sizes
        let (cx, cy) = (x + SIZE / 2, y + SIZE / 2);
        if screens.iter().any(|s| s.contains(cx, cy)) {
            self.pos = (x, y);
        } else {
            self.wandering = false;
        }
    }

    fn update_animation(&mut self) {
        self.anim_steps += 1;
        let delay = self.action.delay();
        if self.anim_steps >= delay {
            let last = self.frame == self.action.frame_count() - 1;
            if self.action == Action::Laying && last {
                if self.anim_steps - delay > 40 {
                    self.anim_steps = 0;
                    self.frame = 0;
                    self.change(if fastrand::bool() { Action::Curled } else { Action::Sleep });
                }
            } else if self.action == Action::Sitting && last {
                self.change(Action::Licking);
                self.anim_steps = 0;
                self.frame = 0;
            } else {
                self.frame += 1;
                self.anim_steps = 0;
            }
        }
        if self.frame >= self.action.frame_count() {
            self.frame = 0;
        }
    }

    fn manage_bubble(&mut self) {
        if self.bubble != Bubble::Heart {
            match self.action {
                Action::Sleep | Action::Curled => self.bubble = Bubble::Zzz,
                Action::Licking | Action::Sitting => {}
                _ => self.bubble = Bubble::None,
            }
        }
        self.bubble_steps += 1;
        if self.bubble_steps >= self.bubble.delay() {
            self.bubble_frame += 1;
            self.bubble_steps = 0;
        }
        if self.bubble_frame >= self.bubble.frame_count() {
            self.bubble_frame = 0;
            if self.bubble == Bubble::Heart {
                self.bubble = Bubble::None;
            }
        }
    }

    pub fn try_wander(&mut self, force: bool, screens: &[Rect]) {
        // 50% of the time
        if (!force && fastrand::bool()) || screens.is_empty() {
            return;
        }
        self.wandering = true;
        for _ in 0..100 {
            // pick a random monitor, then a random point on it
            let s = screens[fastrand::usize(..screens.len())];
            let t = (
                s.x + fastrand::i32(0..(s.w - SIZE - 20).max(1)) + 10,
                s.y + fastrand::i32(0..(s.h - SIZE - 20).max(1)) + 10,
            );
            self.wander_target = t;
            let near = (self.pos.0 - t.0).abs() <= 400 && (self.pos.1 - t.1).abs() <= 400;
            if !near {
                break;
            }
        }
    }

    pub fn pet(&mut self) {
        self.bubble = Bubble::Heart;
        self.bubble_frame = 0;
    }

    /// Dragged so the cursor (global coordinates) is at her centre.
    pub fn drag_to(&mut self, cursor: (i32, i32)) {
        self.pos = (cursor.0 - SIZE / 2, cursor.1 - SIZE / 2);
        if self.change(Action::Rising) {
            self.frame = 0;
        }
    }

    pub fn release(&mut self) {
        if self.action == Action::Rising {
            self.change(Action::Laying);
            self.frame = 1;
        }
    }

    pub fn flipped(&self) -> bool {
        match self.action {
            Action::Laying | Action::Rising | Action::Sleep => self.facing_left,
            Action::Curled => !self.facing_left,
            _ => false,
        }
    }

    /// Where the 30x30 bubble is drawn inside the 100x100 pet.
    pub fn bubble_offset(&self) -> (i32, i32) {
        let (mut x, mut y) = (30, 40);
        match self.action {
            Action::Sleep | Action::Laying | Action::Left | Action::Right => {
                x += if self.facing_left { -30 } else { 30 }
            }
            Action::Up | Action::Licking | Action::Sitting => y -= 25,
            _ => {}
        }
        (x, y)
    }
}
