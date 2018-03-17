extern crate image;
extern crate rusttype;

use rusttype::{point, FontCollection, Scale};
use image::{DynamicImage, Rgba};

fn main() {
    // Load the font
    let font_data = include_bytes!("../fonts/wqy-microhei/WenQuanYiMicroHei.ttf");
    let collection = FontCollection::from_bytes(font_data as &[u8]);
    // This only succeeds if collection consists of one font
    let font = collection.into_font().unwrap();

    // Create a new rgba image
    let mut image = DynamicImage::new_rgba8(500, 100).to_rgba();

    // The font size to use
    let size = 32.0;
    let scale = Scale { x: size, y: size };

    // The text to render
    let text = "This is RustType rendered into a png!";

    // Use a dark red colour
    let colour = (150, 0, 0);

    // The starting positioning of the glyphs (top left corner)
    let start = point(20.0, 50.0);

    // Loop through the glyphs in the text, positing each one on a line
    for glyph in font.layout(text, scale, start) {
        if let Some(bounding_box) = glyph.pixel_bounding_box() {
            // Draw the glyph into the image per-pixel by using the draw closure
            glyph.draw(|x, y, v| {
                image.put_pixel(
                    // Offset the position by the glyph bounding box
                    x + bounding_box.min.x as u32,
                    y + bounding_box.min.y as u32,
                    // Turn the coverage into an alpha value
                    Rgba {
                        data: [colour.0, colour.1, colour.2, (v * 255.0) as u8],
                    },
                )
            });
        }
    }

    // Save the image to a png file
    image.save("image_example.png").unwrap();
}
