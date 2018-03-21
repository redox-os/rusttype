extern crate arrayvec;
#[macro_use]
extern crate glium;
extern crate rusttype;
extern crate unicode_normalization;

use glium::{glutin, Surface};
use rusttype::{point, vector, Font, PositionedGlyph, Rect, Scale};
use rusttype::gpu_cache::CacheBuilder;
use std::borrow::Cow;

fn layout_paragraph<'a>(
    font: &'a Font,
    scale: Scale,
    width: u32,
    text: &str,
) -> Vec<PositionedGlyph<'a>> {
    use unicode_normalization::UnicodeNormalization;
    let mut result = Vec::new();
    let v_metrics = font.v_metrics(scale);
    let advance_height = v_metrics.ascent - v_metrics.descent + v_metrics.line_gap;
    let mut caret = point(0.0, v_metrics.ascent);
    let mut last_glyph_id = None;
    for c in text.nfc() {
        if c.is_control() {
            match c {
                '\r' => {
                    caret = point(0.0, caret.y + advance_height);
                }
                '\n' => {}
                _ => {}
            }
            continue;
        }
        let base_glyph = font.glyph(c);
        if let Some(id) = last_glyph_id.take() {
            caret.x += font.pair_kerning(scale, id, base_glyph.id());
        }
        last_glyph_id = Some(base_glyph.id());
        let mut glyph = base_glyph.scaled(scale).positioned(caret);
        if let Some(bb) = glyph.pixel_bounding_box() {
            if bb.max.x > width as i32 {
                caret = point(0.0, caret.y + advance_height);
                glyph = glyph.into_unpositioned().positioned(caret);
                last_glyph_id = None;
            }
        }
        caret.x += glyph.unpositioned().h_metrics().advance_width;
        result.push(glyph);
    }
    result
}

fn main() {
    let font_data = include_bytes!("../fonts/wqy-microhei/WenQuanYiMicroHei.ttf");
    let font = Font::from_bytes(font_data as &[u8]).unwrap();

    let window = glutin::WindowBuilder::new()
        .with_dimensions(512, 512)
        .with_title("RustType GPU cache example");
    let context = glutin::ContextBuilder::new().with_vsync(true);
    let mut events_loop = glutin::EventsLoop::new();
    let display = glium::Display::new(window, context, &events_loop).unwrap();

    let dpi_factor = display.gl_window().hidpi_factor();

    let (cache_width, cache_height) = (512 * dpi_factor as u32, 512 * dpi_factor as u32);
    let mut cache = CacheBuilder {
        width: cache_width,
        height: cache_height,
        ..CacheBuilder::default()
    }.build();

    let program = program!(
        &display,
        140 => {
            vertex: "
                #version 140

                in vec2 position;
                in vec2 tex_coords;
                in vec4 colour;

                out vec2 v_tex_coords;
                out vec4 v_colour;

                void main() {
                    gl_Position = vec4(position, 0.0, 1.0);
                    v_tex_coords = tex_coords;
                    v_colour = colour;
                }
            ",

            fragment: "
                #version 140
                uniform sampler2D tex;
                in vec2 v_tex_coords;
                in vec4 v_colour;
                out vec4 f_colour;

                void main() {
                    f_colour = v_colour * vec4(1.0, 1.0, 1.0, texture(tex, v_tex_coords).r);
                }
            "
        }).unwrap();
    let cache_tex = glium::texture::Texture2d::with_format(
        &display,
        glium::texture::RawImage2d {
            data: Cow::Owned(vec![128u8; cache_width as usize * cache_height as usize]),
            width: cache_width,
            height: cache_height,
            format: glium::texture::ClientFormat::U8,
        },
        glium::texture::UncompressedFloatFormat::U8,
        glium::texture::MipmapsOption::NoMipmap,
    ).unwrap();
    let mut text: String = "A japanese poem:\r
\r
色は匂へど散りぬるを我が世誰ぞ常ならむ有為の奥山今日越えて浅き夢見じ酔ひもせず\r
\r
Feel free to type out some text, and delete it with Backspace. \
You can also try resizing this window."
        .into();
    loop {
        let (width, dpi_factor) = {
            let window = display.gl_window();
            (window.get_inner_size().unwrap().0, window.hidpi_factor())
        };

        let mut finished = false;
        events_loop.poll_events(|event| {
            use glutin::*;

            if let Event::WindowEvent { event, .. } = event {
                match event {
                    WindowEvent::Closed => finished = true,
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(keypress),
                                ..
                            },
                        ..
                    } => match keypress {
                        VirtualKeyCode::Escape => finished = true,
                        VirtualKeyCode::Back => {
                            text.pop();
                        }
                        _ => (),
                    },
                    WindowEvent::ReceivedCharacter(c) => if c != '\u{7f}' && c != '\u{8}' {
                        text.push(c);
                    },
                    _ => {}
                }
            }
        });
        if finished {
            break;
        }

        let glyphs = layout_paragraph(&font, Scale::uniform(24.0 * dpi_factor), width, &text);
        for glyph in &glyphs {
            cache.queue_glyph(0, glyph.clone());
        }
        cache
            .cache_queued(|rect, data| {
                cache_tex.main_level().write(
                    glium::Rect {
                        left: rect.min.x,
                        bottom: rect.min.y,
                        width: rect.width(),
                        height: rect.height(),
                    },
                    glium::texture::RawImage2d {
                        data: Cow::Borrowed(data),
                        width: rect.width(),
                        height: rect.height(),
                        format: glium::texture::ClientFormat::U8,
                    },
                );
            })
            .unwrap();

        let uniforms = uniform! {
            tex: cache_tex.sampled().magnify_filter(glium::uniforms::MagnifySamplerFilter::Nearest)
        };

        let vertex_buffer = {
            #[derive(Copy, Clone)]
            struct Vertex {
                position: [f32; 2],
                tex_coords: [f32; 2],
                colour: [f32; 4],
            }

            implement_vertex!(Vertex, position, tex_coords, colour);
            let colour = [0.0, 0.0, 0.0, 1.0];
            let (screen_width, screen_height) = {
                let (w, h) = display.get_framebuffer_dimensions();
                (w as f32, h as f32)
            };
            let origin = point(0.0, 0.0);
            let vertices: Vec<Vertex> = glyphs
                .iter()
                .flat_map(|g| {
                    if let Ok(Some((uv_rect, screen_rect))) = cache.rect_for(0, g) {
                        let gl_rect = Rect {
                            min: origin
                                + (vector(
                                    screen_rect.min.x as f32 / screen_width - 0.5,
                                    1.0 - screen_rect.min.y as f32 / screen_height - 0.5,
                                )) * 2.0,
                            max: origin
                                + (vector(
                                    screen_rect.max.x as f32 / screen_width - 0.5,
                                    1.0 - screen_rect.max.y as f32 / screen_height - 0.5,
                                )) * 2.0,
                        };
                        arrayvec::ArrayVec::<[Vertex; 6]>::from([
                            Vertex {
                                position: [gl_rect.min.x, gl_rect.max.y],
                                tex_coords: [uv_rect.min.x, uv_rect.max.y],
                                colour,
                            },
                            Vertex {
                                position: [gl_rect.min.x, gl_rect.min.y],
                                tex_coords: [uv_rect.min.x, uv_rect.min.y],
                                colour,
                            },
                            Vertex {
                                position: [gl_rect.max.x, gl_rect.min.y],
                                tex_coords: [uv_rect.max.x, uv_rect.min.y],
                                colour,
                            },
                            Vertex {
                                position: [gl_rect.max.x, gl_rect.min.y],
                                tex_coords: [uv_rect.max.x, uv_rect.min.y],
                                colour,
                            },
                            Vertex {
                                position: [gl_rect.max.x, gl_rect.max.y],
                                tex_coords: [uv_rect.max.x, uv_rect.max.y],
                                colour,
                            },
                            Vertex {
                                position: [gl_rect.min.x, gl_rect.max.y],
                                tex_coords: [uv_rect.min.x, uv_rect.max.y],
                                colour,
                            },
                        ])
                    } else {
                        arrayvec::ArrayVec::new()
                    }
                })
                .collect();

            glium::VertexBuffer::new(&display, &vertices).unwrap()
        };

        let mut target = display.draw();
        target.clear_color(1.0, 1.0, 1.0, 0.0);
        target
            .draw(
                &vertex_buffer,
                glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList),
                &program,
                &uniforms,
                &glium::DrawParameters {
                    blend: glium::Blend::alpha_blending(),
                    ..Default::default()
                },
            )
            .unwrap();

        target.finish().unwrap();
    }
}
