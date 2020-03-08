use blake2::{Blake2s, Digest};
use criterion::{criterion_group, criterion_main, Criterion};
use once_cell::sync::Lazy;
use rusttype::*;

static DEJA_VU_MONO: Lazy<Font<'static>> = Lazy::new(|| {
    Font::try_from_bytes(include_bytes!("../fonts/dejavu/DejaVuSansMono.ttf") as &[u8]).unwrap()
});
static OPEN_SANS_ITALIC: Lazy<Font<'static>> = Lazy::new(|| {
    Font::try_from_bytes(include_bytes!("../fonts/opensans/OpenSans-Italic.ttf") as &[u8]).unwrap()
});

fn bench_draw_big_biohazard(c: &mut Criterion) {
    let glyph = DEJA_VU_MONO
        .glyph('☣')
        .scaled(Scale::uniform(600.0))
        .positioned(point(0.0, 0.0));

    const WIDTH: usize = 294;
    const HEIGHT: usize = 269;

    let bounds = glyph.pixel_bounding_box().unwrap();
    assert_eq!(
        (bounds.width() as usize, bounds.height() as usize),
        (WIDTH, HEIGHT)
    );

    let mut target = [0u8; WIDTH * HEIGHT];
    c.bench_function("draw_big_biohazard", |b| {
        b.iter(|| {
            glyph.draw(|x, y, alpha| {
                let (x, y) = (x as usize, y as usize);
                target[WIDTH * y + x] = (alpha * 255.0) as u8;
            })
        });
        // verify the draw result against static reference hash
        assert_eq!(
            format!("{:x}", Blake2s::digest(&target)),
            "307a2514a191b827a214174d6c5d109599f0ec4b42d466bde91d10bdd5f8e22d"
        );
    });
}

fn bench_draw_w(c: &mut Criterion) {
    let glyph = DEJA_VU_MONO
        .glyph('w')
        .scaled(Scale::uniform(16.0))
        .positioned(point(0.0, 0.0));

    const WIDTH: usize = 9;
    const HEIGHT: usize = 8;

    let bounds = glyph.pixel_bounding_box().unwrap();
    assert_eq!(
        (bounds.width() as usize, bounds.height() as usize),
        (WIDTH, HEIGHT)
    );

    let mut target = [0u8; WIDTH * HEIGHT];
    c.bench_function("draw_w", |b| {
        b.iter(|| {
            glyph.draw(|x, y, alpha| {
                let (x, y) = (x as usize, y as usize);
                target[WIDTH * y + x] = (alpha * 255.0) as u8;
            })
        });
        // verify the draw result against static reference hash
        assert_eq!(
            format!("{:x}", Blake2s::digest(&target)),
            "c0e795601e3412144d1bfdc0cd94d9507aa9775a0f0f4f9862fe7ec7e83d7684"
        );
    });
}

fn bench_draw_iota(c: &mut Criterion) {
    let glyph = OPEN_SANS_ITALIC
        .glyph('ΐ')
        .scaled(Scale::uniform(60.0))
        .positioned(point(0.0, 0.0));

    const WIDTH: usize = 14;
    const HEIGHT: usize = 38;

    let bounds = glyph.pixel_bounding_box().unwrap();
    assert_eq!(
        (bounds.width() as usize, bounds.height() as usize),
        (WIDTH, HEIGHT)
    );

    let mut target = [0u8; WIDTH * HEIGHT];
    c.bench_function("draw_iota", |b| {
        b.iter(|| {
            glyph.draw(|x, y, alpha| {
                let (x, y) = (x as usize, y as usize);
                target[WIDTH * y + x] = (alpha * 255.0) as u8;
            })
        });
        // verify the draw result against static reference hash
        assert_eq!(
            format!("{:x}", Blake2s::digest(&target)),
            "d8fa90d375a7dc2c8c821395e8cef8baefb78046e4a7a93d87f96509add6a65c"
        );
    });
}

criterion_group!(
    benches,
    bench_draw_big_biohazard,
    bench_draw_w,
    bench_draw_iota,
);

criterion_main!(benches);
