use criterion::{criterion_group, criterion_main, Criterion};
use once_cell::sync::Lazy;
use rusttype::*;

static DEJA_VU_MONO: Lazy<Font<'static>> = Lazy::new(|| {
    Font::try_from_bytes(include_bytes!("../fonts/dejavu/DejaVuSansMono.ttf") as &[u8]).unwrap()
});
static OPEN_SANS_ITALIC: Lazy<Font<'static>> = Lazy::new(|| {
    Font::try_from_bytes(include_bytes!("../fonts/opensans/OpenSans-Italic.ttf") as &[u8]).unwrap()
});
static EXO2_OTF: Lazy<Font<'static>> =
    Lazy::new(|| Font::try_from_bytes(include_bytes!("../fonts/Exo2-Light.otf") as &[u8]).unwrap());
static EXO2_TTF: Lazy<Font<'static>> =
    Lazy::new(|| Font::try_from_bytes(include_bytes!("../fonts/Exo2-Light.ttf") as &[u8]).unwrap());

fn draw_big_biohazard(c: &mut Criterion) {
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
    });
}

fn draw_w(c: &mut Criterion) {
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
    });
}

fn draw_iota(c: &mut Criterion) {
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
    });
}

fn draw_otf_tailed_e(c: &mut Criterion) {
    let glyph = EXO2_OTF
        .glyph('ę')
        .scaled(Scale::uniform(300.0))
        .positioned(point(0.0, 0.0));

    const WIDTH: usize = 106;
    const HEIGHT: usize = 183;

    let bounds = glyph.pixel_bounding_box().unwrap();
    assert_eq!(
        (bounds.width() as usize, bounds.height() as usize),
        (WIDTH, HEIGHT)
    );

    let mut target = [0u8; WIDTH * HEIGHT];
    c.bench_function("draw_otf_tailed_e", |b| {
        b.iter(|| {
            glyph.draw(|x, y, alpha| {
                let (x, y) = (x as usize, y as usize);
                target[WIDTH * y + x] = (alpha * 255.0) as u8;
            })
        });
    });
}

fn draw_ttf_tailed_e(c: &mut Criterion) {
    let glyph = EXO2_TTF
        .glyph('ę')
        .scaled(Scale::uniform(300.0))
        .positioned(point(0.0, 0.0));

    const WIDTH: usize = 106;
    const HEIGHT: usize = 177;

    let bounds = glyph.pixel_bounding_box().unwrap();
    assert_eq!(
        (bounds.width() as usize, bounds.height() as usize),
        (WIDTH, HEIGHT)
    );

    let mut target = [0u8; WIDTH * HEIGHT];
    c.bench_function("draw_ttf_tailed_e", |b| {
        b.iter(|| {
            glyph.draw(|x, y, alpha| {
                let (x, y) = (x as usize, y as usize);
                target[WIDTH * y + x] = (alpha * 255.0) as u8;
            })
        });
    });
}

criterion_group!(
    draw_benches,
    draw_big_biohazard,
    draw_w,
    draw_iota,
    draw_otf_tailed_e,
    draw_ttf_tailed_e,
);

criterion_main!(draw_benches);
