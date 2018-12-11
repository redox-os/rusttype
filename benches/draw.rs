#![feature(test)]

extern crate test;

use blake2::{Blake2s, Digest};
use rusttype::*;

lazy_static::lazy_static! {
    static ref DEJA_VU_MONO: Font<'static> =
        Font::from_bytes(include_bytes!("../fonts/dejavu/DejaVuSansMono.ttf") as &[u8]).unwrap();
    static ref OPEN_SANS_ITALIC: Font<'static> =
        Font::from_bytes(include_bytes!("../fonts/opensans/OpenSans-Italic.ttf") as &[u8]).unwrap();
}

#[bench]
fn draw_big_biohazard(b: &mut test::Bencher) {
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
    b.iter(|| {
        glyph.draw(|x, y, alpha| {
            let (x, y) = (x as usize, y as usize);
            target[WIDTH * y + x] = (alpha * 255.0) as u8;
        })
    });

    // verify the draw result against static reference hash
    assert_eq!(
        format!("{:x}", Blake2s::digest(&target)),
        "8e3927a33c6d563d45f82fb9620dea8036274b403523a2e98cd5f93eafdb2125"
    );
}

#[bench]
fn draw_w(b: &mut test::Bencher) {
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
}

#[bench]
fn draw_iota(b: &mut test::Bencher) {
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
    b.iter(|| {
        glyph.draw(|x, y, alpha| {
            let (x, y) = (x as usize, y as usize);
            target[WIDTH * y + x] = (alpha * 255.0) as u8;
        })
    });

    // verify the draw result against static reference hash
    assert_eq!(
        format!("{:x}", Blake2s::digest(&target)),
        "cdad348e38263a13f68ae41a95ce3b900d2881375a745232309ebd568a27cd4c"
    );
}
