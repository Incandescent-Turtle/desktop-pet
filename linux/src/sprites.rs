//! Sprites and rendering.
//!
//! The PNGs are the Java version's (`src/main/resources`), compiled into the binary.
//! Drawing is a simple nearest-neighbour scale, which keeps the pixel art crisp.

use crate::pet::{Action, BUBBLE_SIZE, Bubble, Pet, SIZE};

/// Premultiplied pixels in wl_shm ARGB8888 byte order (B, G, R, A).
struct Image {
    w: usize,
    h: usize,
    px: Vec<[u8; 4]>,
}

/// Builds a list of embedded PNG frames. `frames!("up": 1, 2)` expands to:
///
/// ```ignore
/// &[include_bytes!("../../src/main/resources/up/up_1.png"),
///   include_bytes!("../../src/main/resources/up/up_2.png")]
/// ```
///
/// `include_bytes!` reads each file at compile time, so the binary needs no PNGs next to it.
/// Paths are relative to this file, hence `../../` up to the repo root.
macro_rules! frames {
    ($dir:literal: $($n:literal),+) => {
        &[$(include_bytes!(concat!(
            "../../src/main/resources/", $dir, "/", $dir, "_", $n, ".png"
        ))),+]
    };
}

fn action_pngs(a: Action) -> &'static [&'static [u8]] {
    match a {
        Action::Up => frames!("up": 1, 2, 3, 4),
        Action::Down => frames!("down": 1, 2, 3, 4),
        Action::Left => frames!("left": 1, 2, 3, 4),
        Action::Right => frames!("right": 1, 2, 3, 4),
        Action::Curled => frames!("curled": 1, 2),
        Action::Laying => frames!("laying": 1, 2, 3, 4),
        Action::Sitting => frames!("sitting": 1, 2, 3, 4),
        Action::Licking => frames!("licking": 1, 2, 3, 4),
        Action::Held => frames!("rising": 1, 2),
        Action::Sleep => frames!("sleep": 1),
    }
}

pub struct Sprites {
    /// Frames per action, indexed by `action as usize`.
    actions: Vec<Vec<Image>>,
    zzz: Vec<Image>,
    heart: Vec<Image>,
}

impl Sprites {
    pub fn load() -> Self {
        let decode_all = |pngs: &[&[u8]]| pngs.iter().map(|b| decode(b)).collect::<Vec<_>>();
        Sprites {
            actions: Action::ALL.iter().map(|&a| decode_all(action_pngs(a))).collect(),
            zzz: decode_all(frames!("zzz": 1, 2, 3, 4)),
            heart: decode_all(frames!("heart": 1, 2, 3, 4)),
        }
    }

    /// Draws the pet into a `size`x`size` ARGB8888 canvas (size = 100 * buffer scale).
    pub fn render(&self, canvas: &mut [u8], size: usize, pet: &Pet) {
        canvas.fill(0);
        let img = &self.actions[pet.action as usize][pet.frame()];
        blit(canvas, size, img, 0, 0, size, pet.flipped());

        let Some((bubble, frame)) = pet.bubble() else { return };
        let frames = match bubble {
            Bubble::Zzz => &self.zzz,
            Bubble::Heart => &self.heart,
        };
        let scale = size as f64 / SIZE as f64;
        let (bx, by) = pet.bubble_offset();
        let at = |v: i32| (v as f64 * scale) as usize;
        blit(canvas, size, &frames[frame], at(bx), at(by), at(BUBBLE_SIZE), false);
    }
}

/// Draws `img` stretched to a `size`x`size` square with its top-left at (`left`, `top`),
/// on top of what's already on the (square) canvas. Pixels past the canvas edge are skipped.
fn blit(
    canvas: &mut [u8],
    canvas_size: usize,
    img: &Image,
    left: usize,
    top: usize,
    size: usize,
    mirrored: bool,
) {
    for y in 0..size {
        for x in 0..size {
            let (canvas_x, canvas_y) = (left + x, top + y);
            if canvas_x >= canvas_size || canvas_y >= canvas_size {
                continue;
            }
            // nearest-neighbour scaling: use the sprite pixel this canvas pixel lands on
            let mut sprite_x = x * img.w / size;
            let sprite_y = y * img.h / size;
            if mirrored {
                sprite_x = img.w - 1 - sprite_x;
            }
            let pixel = img.px[sprite_y * img.w + sprite_x];
            let i = (canvas_y * canvas_size + canvas_x) * 4; // 4 bytes per pixel
            blend_over(&mut canvas[i..i + 4], pixel);
        }
    }
}

/// Alpha-blends premultiplied `src` over `dst`. Needed because some sprite pixels are
/// semi-transparent(shadow), so what's underneath partly shows through.
fn blend_over(dst: &mut [u8], src: [u8; 4]) {
    let show_through = 255 - src[3] as u32;
    for (d, s) in dst.iter_mut().zip(src) {
        *d = (s as u32 + *d as u32 * show_through / 255) as u8;
    }
}

/// Decodes a PNG (any 8-bit-or-less format) into premultiplied BGRA pixels.
fn decode(bytes: &[u8]) -> Image {
    let mut dec = png::Decoder::new(std::io::Cursor::new(bytes));
    dec.set_transformations(
        png::Transformations::normalize_to_color8() | png::Transformations::ALPHA,
    );
    let mut reader = dec.read_info().expect("bad sprite png");
    let mut buf = vec![0; reader.output_buffer_size().expect("sprite too large")];
    let info = reader.next_frame(&mut buf).expect("bad sprite png");
    let (w, h) = (info.width as usize, info.height as usize);
    let channels = info.color_type.samples();
    let px = buf[..w * h * channels]
        .chunks_exact(channels)
        .map(|p| {
            let (r, g, b, a) = match p {
                [r, g, b, a] => (*r, *g, *b, *a),
                [l, a] => (*l, *l, *l, *a),
                _ => panic!("unexpected sprite pixel format"),
            };
            let pm = |c: u8| (c as u32 * a as u32 / 255) as u8;
            [pm(b), pm(g), pm(r), a]
        })
        .collect();
    Image { w, h, px }
}
