use rusttype::{point, FontCollection, PositionedGlyph, Scale};
use std::io::Write;

fn main() {
    let font_data = include_bytes!("../fonts/wqy-microhei/WenQuanYiMicroHei.ttf");
    let collection = FontCollection::from_bytes(font_data as &[u8]).unwrap_or_else(|e| {
        panic!("error constructing a FontCollection from bytes: {}", e);
    });
    let font = collection
        .into_font() // only succeeds if collection consists of one font
        .unwrap_or_else(|e| {
            panic!("error turning FontCollection into a Font: {}", e);
        });

    // Desired font pixel height
    let height: f32 = 12.4; // to get 80 chars across (fits most terminals); adjust as desired
    let pixel_height = height.ceil() as usize;

    // 2x scale in x direction to counter the aspect ratio of monospace characters.
    let scale = Scale {
        x: height * 2.0,
        y: height,
    };

    // The origin of a line of text is at the baseline (roughly where
    // non-descending letters sit). We don't want to clip the text, so we shift
    // it down with an offset when laying it out. v_metrics.ascent is the
    // distance between the baseline and the highest edge of any glyph in
    // the font. That's enough to guarantee that there's no clipping.
    let v_metrics = font.v_metrics(scale);
    let offset = point(0.0, v_metrics.ascent);

    // Glyphs to draw for "RustType". Feel free to try other strings.
    let glyphs: Vec<PositionedGlyph<'_>> = font.layout("RustType", scale, offset).collect();

    // Find the most visually pleasing width to display
    let width = glyphs
        .iter()
        .rev()
        .map(|g| g.position().x as f32 + g.unpositioned().h_metrics().advance_width)
        .next()
        .unwrap_or(0.0)
        .ceil() as usize;

    println!("width: {}, height: {}", width, pixel_height);

    // Rasterise directly into ASCII art.
    let mut pixel_data = vec![b'@'; width * pixel_height];
    let mapping = b"@%#x+=:-. "; // The approximation of greyscale
    let mapping_scale = (mapping.len() - 1) as f32;
    for g in glyphs {
        if let Some(bb) = g.pixel_bounding_box() {
            g.draw(|x, y, v| {
                // v should be in the range 0.0 to 1.0
                let i = (v * mapping_scale + 0.5) as usize;
                // so something's wrong if you get $ in the output.
                let c = mapping.get(i).cloned().unwrap_or(b'$');
                let x = x as i32 + bb.min.x;
                let y = y as i32 + bb.min.y;
                // There's still a possibility that the glyph clips the boundaries of the bitmap
                if x >= 0 && x < width as i32 && y >= 0 && y < pixel_height as i32 {
                    let x = x as usize;
                    let y = y as usize;
                    pixel_data[(x + y * width)] = c;
                }
            })
        }
    }

    // Print it out
    let stdout = ::std::io::stdout();
    let mut handle = stdout.lock();
    for j in 0..pixel_height {
        handle
            .write_all(&pixel_data[j * width..(j + 1) * width])
            .unwrap();
        handle.write_all(b"\n").unwrap();
    }
}
