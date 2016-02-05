extern crate rusttype;
use rusttype::*;
fn main() {
    use std::io::Write;
    let font_data = include_bytes!("Gudea-Regular.ttf");
    let collection = FontCollection::<'static>::from_bytes(font_data as &[u8]);
    let font = collection.into_font().unwrap(); // only succeeds if collection consists of one font
    {
        let height: f32 = 20.1; // to get 80 chars across (fits most terminals); adjust as desired
        let pixel_height = height.ceil() as usize;
        let scale = PixelsXY(height*2.0, height);
        let v_metrics = font.v_metrics(scale);
        let offset = point(0.0, v_metrics.ascent);
        // 2x scale in x direction to counter the aspect ratio of monospace characters.
        let to_draw: Vec<_> = font.layout("Hello!", PixelsXY(height*2.0, height), offset).collect();
        // Finds the most visually pleasing width to display
        let width = to_draw.iter().map(|g| g.h_metrics().advance_width).fold(0.0, |x, y| x + y).ceil() as usize;
        println!("width: {}, height: {}", width, pixel_height);
        // Rasterise directly into ASCII art.
        let mut data = vec![b'@'; width * pixel_height];
        let mapping = b"@%#x+=:-. "; // The approximation of greyscale
        let mapping_scale = (mapping.len()-1) as f32;
        for g in to_draw {
            if let Some(bb) = g.pixel_bounding_box() {
                g.draw(|x, y, v| {
                    let i = (v*mapping_scale + 0.5) as usize;
                    // Something's wrong if you get $ in the output.
                    let c = mapping.get(i).cloned().unwrap_or(b'$');
                    let x = (x as i32 + bb.min.x) as usize;
                    let y = (y as i32 + bb.min.y) as usize;
                    data[(x + y * width)] = c;
                })
            }
        }
        // Print it out
        let stdout = ::std::io::stdout();
        let mut handle = stdout.lock();
        for j in 0..pixel_height {
            handle.write(&data[j*width..(j+1)*width]).unwrap();
            handle.write(b"\n").unwrap();
        }
    }
}
