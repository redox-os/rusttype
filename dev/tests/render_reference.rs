use image::{DynamicImage, LumaA};
use once_cell::sync::Lazy;
use rusttype::{point, Font, Scale, ScaledGlyph};
use std::io::Cursor;

static DEJA_VU_MONO: Lazy<Font<'static>> = Lazy::new(|| {
    Font::try_from_bytes(include_bytes!("../fonts/dejavu/DejaVuSansMono.ttf") as &[u8]).unwrap()
});
static OPEN_SANS_ITALIC: Lazy<Font<'static>> = Lazy::new(|| {
    Font::try_from_bytes(include_bytes!("../fonts/opensans/OpenSans-Italic.ttf") as &[u8]).unwrap()
});
static EXO2_OFT: Lazy<Font<'static>> =
    Lazy::new(|| Font::try_from_bytes(include_bytes!("../fonts/Exo2-Light.otf") as &[u8]).unwrap());

fn draw_luma_alpha(glyph: ScaledGlyph<'_>) -> image::GrayAlphaImage {
    let glyph = glyph.positioned(point(0.0, 0.0));
    let bounds = glyph.pixel_bounding_box().unwrap();
    let mut glyph_image =
        DynamicImage::new_luma_a8(bounds.width() as _, bounds.height() as _).into_luma_alpha8();

    glyph.draw(|x, y, v| glyph_image.put_pixel(x, y, LumaA([128, (v * 255.0) as u8])));

    glyph_image
}

/// Render a 600px U+2623 character require it to match the reference with
/// 8-bit accuracy
#[test]
fn render_to_reference_big_biohazard() {
    let new_image = draw_luma_alpha(DEJA_VU_MONO.glyph('☣').scaled(Scale::uniform(600.0)));

    // save the new render for manual inspection
    new_image.save("../target/big_biohazard.png").unwrap();

    let reference = image::load(
        Cursor::new(include_bytes!("reference_big_biohazard.png") as &[u8]),
        image::ImageFormat::Png,
    )
    .expect("!image::load")
    .into_luma_alpha8();

    assert_eq!(reference.dimensions(), new_image.dimensions());

    for y in 0..reference.height() {
        for x in 0..reference.width() {
            assert_eq!(
                reference.get_pixel(x, y),
                new_image.get_pixel(x, y),
                "unexpected alpha difference at ({}, {})",
                x,
                y
            );
        }
    }
}

/// Render a 16px 'w' character require it to match the reference with 8-bit
/// accuracy
#[test]
fn render_to_reference_w() {
    let new_image = draw_luma_alpha(DEJA_VU_MONO.glyph('w').scaled(Scale::uniform(16.0)));

    // save the new render for manual inspection
    new_image.save("../target/w.png").unwrap();

    let reference = image::load(
        Cursor::new(include_bytes!("reference_w.png") as &[u8]),
        image::ImageFormat::Png,
    )
    .expect("!image::load")
    .into_luma_alpha8();

    assert_eq!(reference.dimensions(), new_image.dimensions());

    for y in 0..reference.height() {
        for x in 0..reference.width() {
            assert_eq!(
                reference.get_pixel(x, y),
                new_image.get_pixel(x, y),
                "unexpected alpha difference at ({}, {})",
                x,
                y
            );
        }
    }
}

/// Render a 60px 'ΐ' character require it to match the reference with 8-bit
/// accuracy
#[test]
fn render_to_reference_iota() {
    let new_image = draw_luma_alpha(OPEN_SANS_ITALIC.glyph('ΐ').scaled(Scale::uniform(60.0)));

    // save the new render for manual inspection
    new_image.save("../target/iota.png").unwrap();

    let reference = image::load(
        Cursor::new(include_bytes!("reference_iota.png") as &[u8]),
        image::ImageFormat::Png,
    )
    .expect("!image::load")
    .into_luma_alpha8();

    assert_eq!(reference.dimensions(), new_image.dimensions());

    for y in 0..reference.height() {
        for x in 0..reference.width() {
            assert_eq!(
                reference.get_pixel(x, y),
                new_image.get_pixel(x, y),
                "unexpected alpha difference at ({}, {})",
                x,
                y
            );
        }
    }
}

/// Render a 300px 'ę' character that uses cubic beziers & require it to match
/// the reference.
#[test]
fn render_to_reference_oft_tailed_e() {
    let new_image = draw_luma_alpha(EXO2_OFT.glyph('ę').scaled(Scale::uniform(300.0)));

    // save the new render for manual inspection
    new_image.save("../target/otf_tailed_e.png").unwrap();

    let reference = image::load(
        Cursor::new(include_bytes!("reference_otf_tailed_e.png") as &[u8]),
        image::ImageFormat::Png,
    )
    .expect("!image::load")
    .into_luma_alpha8();

    assert_eq!(reference.dimensions(), new_image.dimensions());

    for y in 0..reference.height() {
        for x in 0..reference.width() {
            assert_eq!(
                reference.get_pixel(x, y),
                new_image.get_pixel(x, y),
                "unexpected alpha difference at ({}, {})",
                x,
                y
            );
        }
    }
}
