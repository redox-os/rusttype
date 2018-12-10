use rusttype::*;

static ROBOTO_REGULAR: &[u8] = include_bytes!("../fonts/Roboto-Regular.ttf");

#[test]
fn consistent_bounding_box_subpixel_size_proxy() {
    let font = Font::from_bytes(ROBOTO_REGULAR).unwrap();
    let height_at_y = |y| {
        font.glyph('s')
            .scaled(rusttype::Scale::uniform(20.0))
            .positioned(rusttype::Point { x: 0.0, y })
            .pixel_bounding_box()
            .unwrap()
            .height()
    };
    assert_eq!(height_at_y(50.833_336), height_at_y(110.833_336));
}

#[test]
fn consistent_bounding_box_subpixel_size_standalone() {
    let font = Font::from_bytes(ROBOTO_REGULAR).unwrap();
    let height_at_y = |y| {
        font.glyph('s')
            .standalone()
            .scaled(rusttype::Scale::uniform(20.0))
            .positioned(rusttype::Point { x: 0.0, y })
            .pixel_bounding_box()
            .unwrap()
            .height()
    };
    assert_eq!(height_at_y(50.833_336), height_at_y(110.833_336));
}
