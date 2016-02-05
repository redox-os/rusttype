extern crate rusttype;

use rusttype::{FontCollection, PixelsXY, point, PositionedGlyph};
use std::io::Write;

fn main() {
    let font_data = include_bytes!("Gudea-Regular.ttf");
    let collection = FontCollection::from_bytes(font_data as &[u8]);
    let font = collection.into_font().unwrap(); // only succeeds if collection consists of one font

    // Desired font pixel height
    let height: f32 = 12.5; // to get 80 chars across (fits most terminals); adjust as desired
    let pixel_height = height.ceil() as usize;

    // 2x scale in x direction to counter the aspect ratio of monospace characters.
    let scale = PixelsXY(height*2.0, height);

    // The origin of a line of text is at the baseline (roughly where non-descending letters sit).
    // We don't want to clip the text, so we shift it down with an offset when laying it out.
    // v_metrics.ascent is the distance between the baseline and the highest edge of any glyph in
    // the font. That's enough to guarantee that there's no clipping.
    let v_metrics = font.v_metrics(scale);
    let offset = point(0.0, v_metrics.ascent);

    // Glyphs to draw for "RustType". Feel free to try other strings.
    let glyphs: Vec<PositionedGlyph> = font.layout("RustType", scale, offset).collect();

    // Find the most visually pleasing width to display
    let width = glyphs.iter().map(|g| g.h_metrics().advance_width)
        .fold(0.0, |x, y| x + y).ceil() as usize;

    println!("width: {}, height: {}", width, pixel_height);

    // Rasterise directly into ASCII art.
    let mut pixel_data = vec![b'@'; width * pixel_height];
    let mapping = b"@%#x+=:-. "; // The approximation of greyscale
    let mapping_scale = (mapping.len()-1) as f32;
    for g in glyphs {
        if let Some(bb) = g.pixel_bounding_box() {
            g.draw(|x, y, v| {
                // v should be in the range 0.0 to 1.0
                let i = (v*mapping_scale + 0.5) as usize;
                // so something's wrong if you get $ in the output.
                let c = mapping.get(i).cloned().unwrap_or(b'$');
                let x = (x as i32 + bb.min.x) as usize;
                let y = (y as i32 + bb.min.y) as usize;
                pixel_data[(x + y * width)] = c;
            })
        }
    }

    // Print it out
    let stdout = ::std::io::stdout();
    let mut handle = stdout.lock();
    for j in 0..pixel_height {
        handle.write(&pixel_data[j*width..(j+1)*width]).unwrap();
        handle.write(b"\n").unwrap();
    }
}
