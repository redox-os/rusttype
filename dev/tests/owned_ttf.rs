use rusttype::*;

#[test]
fn move_and_use() {
    let owned_data = include_bytes!("../fonts/opensans/OpenSans-Italic.ttf").to_vec();
    let pin_font = Font::try_from_vec(owned_data).unwrap();

    let ascent = pin_font.v_metrics_unscaled().ascent;

    // force a move
    let moved = Box::new(pin_font);

    assert_eq!(moved.v_metrics_unscaled().ascent, ascent);
}
