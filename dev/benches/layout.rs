use blake2::{Blake2s, Digest};
use criterion::{criterion_group, criterion_main, Criterion};
use rusttype::*;
use std::io::Write;

fn bench_layout_a_sentence(c: &mut Criterion) {
    const SENTENCE: &str =
        "a set of words that is complete in itself, typically containing a subject and predicate, \
         conveying a statement, question, exclamation, or command, and consisting of a main \
         clause and sometimes one or more subordinate clauses.";

    c.bench_function("layout_a_sentence", |b| {
        let font =
            Font::try_from_bytes(include_bytes!("../fonts/opensans/OpenSans-Italic.ttf") as &[u8])
                .unwrap();
        let mut glyphs = vec![];

        b.iter(|| {
            glyphs.clear();
            glyphs.extend(font.layout(SENTENCE, Scale::uniform(25.0), point(100.0, 25.0)))
        });

        // verify the layout result against static reference hash
        let mut hash = Blake2s::default();
        for g in glyphs {
            write!(
                hash,
                "{id}:{scale_x}:{scale_y}:{pos_x}:{pos_y}",
                id = g.id().0,
                scale_x = g.scale().x,
                scale_y = g.scale().y,
                pos_x = g.position().x,
                pos_y = g.position().y,
            )
            .unwrap();
        }
        assert_eq!(
            format!("{:x}", hash.finalize()),
            "c2a3483ddf5598ec869440c62d17efa5a4fe72f9893bcc05dd17be2adcaa7629"
        );
    });

    c.bench_function("layout_a_sentence (try_from_vec)", |b| {
        let font =
            Font::try_from_vec(include_bytes!("../fonts/opensans/OpenSans-Italic.ttf").to_vec())
                .unwrap();
        let mut glyphs = vec![];

        b.iter(|| {
            glyphs.clear();
            glyphs.extend(font.layout(SENTENCE, Scale::uniform(25.0), point(100.0, 25.0)))
        });

        // verify the layout result against static reference hash
        let mut hash = Blake2s::default();
        for g in glyphs {
            write!(
                hash,
                "{id}:{scale_x}:{scale_y}:{pos_x}:{pos_y}",
                id = g.id().0,
                scale_x = g.scale().x,
                scale_y = g.scale().y,
                pos_x = g.position().x,
                pos_y = g.position().y,
            )
            .unwrap();
        }
        assert_eq!(
            format!("{:x}", hash.finalize()),
            "c2a3483ddf5598ec869440c62d17efa5a4fe72f9893bcc05dd17be2adcaa7629"
        );
    });

    c.bench_function("layout_a_sentence (exo2-otf)", |b| {
        let font = Font::try_from_bytes(include_bytes!("../fonts/Exo2-Light.otf")).unwrap();
        let mut glyphs = vec![];

        b.iter(|| {
            glyphs.clear();
            glyphs.extend(font.layout(SENTENCE, Scale::uniform(25.0), point(100.0, 25.0)))
        });

        // verify the layout result against static reference hash
        let mut hash = Blake2s::default();
        for g in glyphs {
            write!(
                hash,
                "{id}:{scale_x}:{scale_y}:{pos_x}:{pos_y}",
                id = g.id().0,
                scale_x = g.scale().x,
                scale_y = g.scale().y,
                pos_x = g.position().x,
                pos_y = g.position().y,
            )
            .unwrap();
        }
        assert_eq!(
            format!("{:x}", hash.finalize()),
            "255381ba7ae154c0208f8e73a8620f78dcc2728b15fb9ec952026a99797d4ddc"
        );
    });

    c.bench_function("layout_a_sentence (exo2-ttf)", |b| {
        let font = Font::try_from_bytes(include_bytes!("../fonts/Exo2-Light.ttf")).unwrap();
        let mut glyphs = vec![];

        b.iter(|| {
            glyphs.clear();
            glyphs.extend(font.layout(SENTENCE, Scale::uniform(25.0), point(100.0, 25.0)))
        });

        // verify the layout result against static reference hash
        let mut hash = Blake2s::default();
        for g in glyphs {
            write!(
                hash,
                "{id}:{scale_x}:{scale_y}:{pos_x}:{pos_y}",
                id = g.id().0,
                scale_x = g.scale().x,
                scale_y = g.scale().y,
                pos_x = g.position().x,
                pos_y = g.position().y,
            )
            .unwrap();
        }
        assert_eq!(
            format!("{:x}", hash.finalize()),
            "255381ba7ae154c0208f8e73a8620f78dcc2728b15fb9ec952026a99797d4ddc"
        );
    });
}

criterion_group!(
    name = benches;
    config = Criterion::default().sample_size(400);
    targets = bench_layout_a_sentence);

criterion_main!(benches);
