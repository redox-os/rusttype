use glium::*;
use glutin::{
    event::{Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::ControlFlow,
};
use rusttype::gpu_cache::Cache;
use rusttype::{point, vector, Font, PositionedGlyph, Rect, Scale};
use std::borrow::Cow;
use std::env;
use std::error::Error;

fn layout_paragraph<'a>(
    font: &Font<'a>,
    scale: Scale,
    width: u32,
    text: &str,
) -> Vec<PositionedGlyph<'a>> {
    let mut result = Vec::new();
    let v_metrics = font.v_metrics(scale);
    let advance_height = v_metrics.ascent - v_metrics.descent + v_metrics.line_gap;
    let mut caret = point(0.0, v_metrics.ascent);
    let mut last_glyph_id = None;
    for c in text.chars() {
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
                glyph.set_position(caret);
                last_glyph_id = None;
            }
        }
        caret.x += glyph.unpositioned().h_metrics().advance_width;
        result.push(glyph);
    }
    result
}

fn main() -> Result<(), Box<dyn Error>> {
    if cfg!(target_os = "linux") && env::var("WINIT_UNIX_BACKEND").is_err() {
        env::set_var("WINIT_UNIX_BACKEND", "x11");
    }

    let font_data = include_bytes!("../fonts/wqy-microhei/WenQuanYiMicroHei.ttf");
    let font: Font<'static> = Font::from_bytes(font_data as &[u8])?;

    let window = glium::glutin::window::WindowBuilder::new()
        .with_inner_size(glium::glutin::dpi::PhysicalSize::new(512, 512))
        .with_title("RustType GPU cache example");
    let context = glium::glutin::ContextBuilder::new().with_vsync(true);
    let event_loop = glium::glutin::event_loop::EventLoop::new();
    let display = glium::Display::new(window, context, &event_loop)?;

    let scale = display.gl_window().window().scale_factor();

    let (cache_width, cache_height) = ((512.0 * scale) as u32, (512.0 * scale) as u32);
    let mut cache: Cache<'static> = Cache::builder()
        .dimensions(cache_width, cache_height)
        .build();

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
    })?;
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
    )?;
    let mut text: String = "A japanese poem:\r
\r
色は匂へど散りぬるを我が世誰ぞ常ならむ有為の奥山今日越えて浅き夢見じ酔ひもせず\r
\r
Feel free to type out some text, and delete it with Backspace. \
You can also try resizing this window."
        .into();

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            virtual_keycode: Some(VirtualKeyCode::Escape),
                            ..
                        },
                    ..
                }
                | WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::ReceivedCharacter(c) => {
                    match c {
                        '\u{8}' => { text.pop(); },
                        _ if c != '\u{7f}' => text.push(c),
                        _ => {}
                    }
                    display.gl_window().window().request_redraw();
                }
                _ => (),
            }
            Event::RedrawRequested(_) => {
                let scale = display.gl_window().window().scale_factor();
                let (width, _): (u32, _) = display
                    .gl_window()
                    .window()
                    .inner_size()
                    .into();
                let scale = scale as f32;

                let glyphs = layout_paragraph(&font, Scale::uniform(24.0 * scale), width, &text);
                for glyph in &glyphs {
                    cache.queue_glyph(0, glyph.clone());
                }
                cache.cache_queued(|rect, data| {
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
                }).unwrap();

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
                target.draw(
                    &vertex_buffer,
                    glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList),
                    &program,
                    &uniforms,
                    &glium::DrawParameters {
                        blend: glium::Blend::alpha_blending(),
                        ..Default::default()
                    },
                ).unwrap();

                target.finish().unwrap();
            }
            _ => (),
        }
    });
}
