#![feature(test)]
#![cfg(feature = "gpu_cache")]

extern crate test;

use rusttype::gpu_cache::*;
use rusttype::*;

/// Busy wait 2us
fn mock_gpu_upload(_region: Rect<u32>, _bytes: &[u8]) {
    use std::time::{Duration, Instant};

    let now = Instant::now();
    while now.elapsed() < Duration::from_micros(2) {}
}

fn test_glyphs<'a>(font: &Font<'a>, string: &str) -> Vec<PositionedGlyph<'a>> {
    let mut glyphs = vec![];
    // Set of scales, found through brute force, to reproduce GlyphNotCached issue
    // Cache settings also affect this, it occurs when position_tolerance is < 1.0
    for scale in &[25_f32, 24.5, 25.01, 24.7, 24.99] {
        for glyph in layout_paragraph(font, Scale::uniform(*scale), 500, string) {
            glyphs.push(glyph);
        }
    }
    glyphs
}

fn layout_paragraph<'a>(
    font: &Font<'a>,
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
            if c == '\n' {
                caret = point(0.0, caret.y + advance_height)
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

lazy_static::lazy_static! {
    static ref FONTS: Vec<Font<'static>> = vec![
        include_bytes!("../fonts/wqy-microhei/WenQuanYiMicroHei.ttf") as &[u8],
        include_bytes!("../fonts/dejavu/DejaVuSansMono.ttf") as &[u8],
        include_bytes!("../fonts/opensans/OpenSans-Italic.ttf") as &[u8],
    ]
    .into_iter()
    .map(|bytes| Font::from_bytes(bytes).unwrap())
    .collect();
}

const TEST_STR: &str = include_str!("../tests/lipsum.txt");

/// General use benchmarks.
mod cache {
    use super::*;

    /// Benchmark using a single font at "don't care" position tolerance
    #[bench]
    fn high_position_tolerance(b: &mut ::test::Bencher) {
        let font_id = 0;
        let glyphs = test_glyphs(&FONTS[font_id], TEST_STR);
        let mut cache = Cache::builder()
            .dimensions(1024, 1024)
            .scale_tolerance(0.1)
            .position_tolerance(1.0)
            .build();

        b.iter(|| {
            for glyph in &glyphs {
                cache.queue_glyph(font_id, glyph.clone());
            }

            cache.cache_queued(|_, _| {}).expect("cache_queued");

            for (index, glyph) in glyphs.iter().enumerate() {
                let rect = cache.rect_for(font_id, glyph);
                assert!(
                    rect.is_ok(),
                    "Gpu cache rect lookup failed ({:?}) for glyph index {}, id {}",
                    rect,
                    index,
                    glyph.id().0
                );
            }
        });
    }

    /// Benchmark using a single font with default tolerances
    #[bench]
    fn single_font(b: &mut ::test::Bencher) {
        let font_id = 0;
        let glyphs = test_glyphs(&FONTS[font_id], TEST_STR);
        let mut cache = Cache::builder().dimensions(1024, 1024).build();

        b.iter(|| {
            for glyph in &glyphs {
                cache.queue_glyph(font_id, glyph.clone());
            }

            cache.cache_queued(|_, _| {}).expect("cache_queued");

            for (index, glyph) in glyphs.iter().enumerate() {
                let rect = cache.rect_for(font_id, glyph);
                assert!(
                    rect.is_ok(),
                    "Gpu cache rect lookup failed ({:?}) for glyph index {}, id {}",
                    rect,
                    index,
                    glyph.id().0
                );
            }
        });
    }

    /// Benchmark using multiple fonts with default tolerances
    #[bench]
    fn multi_font(b: &mut ::test::Bencher) {
        // Use a smaller amount of the test string, to offset the extra font-glyph
        // bench load
        let up_to_index = TEST_STR
            .char_indices()
            .nth(TEST_STR.chars().count() / FONTS.len())
            .unwrap()
            .0;
        let string = &TEST_STR[..up_to_index];

        let font_glyphs: Vec<_> = FONTS
            .iter()
            .enumerate()
            .map(|(id, font)| (id, test_glyphs(font, string)))
            .collect();
        let mut cache = Cache::builder().dimensions(1024, 1024).build();

        b.iter(|| {
            for &(font_id, ref glyphs) in &font_glyphs {
                for glyph in glyphs {
                    cache.queue_glyph(font_id, glyph.clone());
                }
            }

            cache.cache_queued(|_, _| {}).expect("cache_queued");

            for &(font_id, ref glyphs) in &font_glyphs {
                for (index, glyph) in glyphs.iter().enumerate() {
                    let rect = cache.rect_for(font_id, glyph);
                    assert!(
                        rect.is_ok(),
                        "Gpu cache rect lookup failed ({:?}) for font {} glyph index {}, id {}",
                        rect,
                        font_id,
                        index,
                        glyph.id().0
                    );
                }
            }
        });
    }

    /// Benchmark using multiple fonts with default tolerances, clears the
    /// cache each run to test the population "first run" performance
    #[bench]
    fn multi_font_population(b: &mut ::test::Bencher) {
        // Use a much smaller amount of the test string, to offset the extra font-glyph
        // bench load & much slower performance of fresh population each run
        let up_to_index = TEST_STR.char_indices().nth(70).unwrap().0;
        let string = &TEST_STR[..up_to_index];

        let font_glyphs: Vec<_> = FONTS
            .iter()
            .enumerate()
            .map(|(id, font)| (id, test_glyphs(font, string)))
            .collect();

        b.iter(|| {
            let mut cache = Cache::builder().dimensions(1024, 1024).build();

            for &(font_id, ref glyphs) in &font_glyphs {
                for glyph in glyphs {
                    cache.queue_glyph(font_id, glyph.clone());
                }
            }

            cache.cache_queued(|_, _| {}).expect("cache_queued");

            for &(font_id, ref glyphs) in &font_glyphs {
                for (index, glyph) in glyphs.iter().enumerate() {
                    let rect = cache.rect_for(font_id, glyph);
                    assert!(
                        rect.is_ok(),
                        "Gpu cache rect lookup failed ({:?}) for font {} glyph index {}, id {}",
                        rect,
                        font_id,
                        index,
                        glyph.id().0
                    );
                }
            }
        });
    }

    /// Benchmark using multiple fonts and a different text group of glyphs
    /// each run
    #[bench]
    fn moving_text(b: &mut ::test::Bencher) {
        let chars: Vec<_> = TEST_STR.chars().collect();
        let subsection_len = chars.len() / FONTS.len();
        let distinct_subsection: Vec<_> = chars.windows(subsection_len).collect();

        let mut first_glyphs = vec![];
        let mut middle_glyphs = vec![];
        let mut last_glyphs = vec![];

        for (id, font) in FONTS.iter().enumerate() {
            let first_str: String = distinct_subsection[0].iter().collect();
            first_glyphs.push((id, test_glyphs(font, &first_str)));

            let middle_str: String = distinct_subsection[distinct_subsection.len() / 2]
                .iter()
                .collect();
            middle_glyphs.push((id, test_glyphs(font, &middle_str)));

            let last_str: String = distinct_subsection[distinct_subsection.len() - 1]
                .iter()
                .collect();
            last_glyphs.push((id, test_glyphs(font, &last_str)));
        }

        let test_variants = [first_glyphs, middle_glyphs, last_glyphs];
        let mut test_variants = test_variants.iter().cycle();

        let mut cache = Cache::builder()
            .dimensions(1500, 1500)
            .scale_tolerance(0.1)
            .position_tolerance(0.1)
            .build();

        b.iter(|| {
            // switch text variant each run to force cache to deal with moving text
            // requirements
            let glyphs = test_variants.next().unwrap();
            for &(font_id, ref glyphs) in glyphs {
                for glyph in glyphs {
                    cache.queue_glyph(font_id, glyph.clone());
                }
            }

            cache.cache_queued(|_, _| {}).expect("cache_queued");

            for &(font_id, ref glyphs) in glyphs {
                for (index, glyph) in glyphs.iter().enumerate() {
                    let rect = cache.rect_for(font_id, glyph);
                    assert!(
                        rect.is_ok(),
                        "Gpu cache rect lookup failed ({:?}) for font {} glyph index {}, id {}",
                        rect,
                        font_id,
                        index,
                        glyph.id().0
                    );
                }
            }
        });
    }
}

/// Benchmarks for cases that should generally be avoided by the cache user if
/// at all possible (ie by picking a better initial cache size).
mod cache_bad_cases {
    use super::*;

    /// Cache isn't large enough for a queue so a new cache is created to hold
    /// the queue.
    #[bench]
    fn resizing(b: &mut ::test::Bencher) {
        let up_to_index = TEST_STR.char_indices().nth(120).unwrap().0;
        let string = &TEST_STR[..up_to_index];

        let font_glyphs: Vec<_> = FONTS
            .iter()
            .enumerate()
            .map(|(id, font)| (id, test_glyphs(font, string)))
            .collect();

        b.iter(|| {
            let mut cache = Cache::builder().dimensions(256, 256).build();

            for &(font_id, ref glyphs) in &font_glyphs {
                for glyph in glyphs {
                    cache.queue_glyph(font_id, glyph.clone());
                }
            }

            cache
                .cache_queued(mock_gpu_upload)
                .expect_err("shouldn't fit");

            cache.to_builder().dimensions(512, 512).rebuild(&mut cache);

            cache.cache_queued(mock_gpu_upload).expect("should fit now");

            for &(font_id, ref glyphs) in &font_glyphs {
                for (index, glyph) in glyphs.iter().enumerate() {
                    let rect = cache.rect_for(font_id, glyph);
                    assert!(
                        rect.is_ok(),
                        "Gpu cache rect lookup failed ({:?}) for font {} glyph index {}, id {}",
                        rect,
                        font_id,
                        index,
                        glyph.id().0
                    );
                }
            }
        });
    }

    /// Benchmark using multiple fonts and a different text group of glyphs
    /// each run. The cache is only large enough to fit each run if it is
    /// cleared and re-built.
    #[bench]
    fn moving_text_thrashing(b: &mut ::test::Bencher) {
        let chars: Vec<_> = TEST_STR.chars().collect();
        let subsection_len = 60;
        let distinct_subsection: Vec<_> = chars.windows(subsection_len).collect();

        let mut first_glyphs = vec![];
        let mut middle_glyphs = vec![];
        let mut last_glyphs = vec![];

        for (id, font) in FONTS.iter().enumerate() {
            let first_str: String = distinct_subsection[0].iter().collect();
            first_glyphs.push((id, test_glyphs(font, &first_str)));

            let middle_str: String = distinct_subsection[distinct_subsection.len() / 2]
                .iter()
                .collect();
            middle_glyphs.push((id, test_glyphs(font, &middle_str)));

            let last_str: String = distinct_subsection[distinct_subsection.len() - 1]
                .iter()
                .collect();
            last_glyphs.push((id, test_glyphs(font, &last_str)));
        }

        let test_variants = [first_glyphs, middle_glyphs, last_glyphs];

        // Cache is only a little larger than each variants size meaning a lot of
        // re-ordering, re-rasterization & re-uploading has to occur.
        let mut cache = Cache::builder()
            .dimensions(450, 450)
            .scale_tolerance(0.1)
            .position_tolerance(0.1)
            .build();

        b.iter(|| {
            // switch text variant each run to force cache to deal with moving text
            // requirements
            for glyphs in &test_variants {
                for &(font_id, ref glyphs) in glyphs {
                    for glyph in glyphs {
                        cache.queue_glyph(font_id, glyph.clone());
                    }
                }

                cache.cache_queued(mock_gpu_upload).expect("cache_queued");

                for &(font_id, ref glyphs) in glyphs {
                    for (index, glyph) in glyphs.iter().enumerate() {
                        let rect = cache.rect_for(font_id, glyph);
                        assert!(
                            rect.is_ok(),
                            "Gpu cache rect lookup failed ({:?}) for font {} glyph index {}, id {}",
                            rect,
                            font_id,
                            index,
                            glyph.id().0
                        );
                    }
                }
            }
        });
    }
}
