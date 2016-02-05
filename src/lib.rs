//! RustType is a pure Rust alternative to libraries like FreeType.
//!
//! The current capabilities of RustType:
//!
//! * Reading TrueType formatted fonts and font collections. This includes `*.ttf` as well as a subset
//!   of `*.otf` font files.
//! * Retrieving glyph shapes and commonly used properties for a font and its glyphs.
//! * Laying out glyphs horizontally using horizontal and vertical metrics, and glyph-pair-specific kerning.
//! * Rasterising glyphs with sub-pixel positioning using an accurate analytical algorithm
//!   (not based on sampling).
//!
//! Notable things that RustType does not support *yet*:
//!
//! * OpenType formatted fonts that are not just TrueType fonts (OpenType is a superset of TrueType). Notably
//!   there is no support yet for cubic Bezier curves used in glyphs.
//! * Ligatures of any kind
//! * Some less common TrueType sub-formats.
//! * Right-to-left and vertical text layout.
//!
//! # Getting Started
//!
//! Add the following to your Cargo.toml:
//!
//! ```toml
//! [dependencies]
//! rusttype = "0.1"
//! ```
//!
//! To hit the ground running with RustType, look at the `simple.rs` example supplied with the crate. It
//! demonstrates loading a font file, rasterising an arbitrary string, and displaying the result as ASCII art.
//! If you prefer to just look at the documentation, the entry point for loading fonts is `FontCollection`,
//! from which you can access individual fonts, then their glyphs.
//!
//! # Glyphs
//!
//! The glyph API uses an inheritance-style approach using `Deref` to incrementally augment a glyph with
//! information such as scaling and positioning, making relevant methods that make use of this information
//! available as appropriate. For example, given a `Glyph` `glyph` obtained directly from a `Font`:
//!
//! ```no_run
//! # use rusttype::*;
//! # let glyph: Glyph<'static> = unimplemented!();
//! // One of the few things you can do with an unsized, positionless glyph is get its id.
//! let id = glyph.id();
//! let glyph = glyph.scaled(Pixels(10.0));
//! // Now glyph is a ScaledGlyph, you can do more with it, as well as what you can do with Glyph.
//! // For example, you can access the correctly scaled horizontal metrics for the glyph.
//! let h_metrics = glyph.h_metrics();
//! let glyph = glyph.positioned(point(5.0, 3.0));
//! // Now glyph is a PositionedGlyph, and you can do even more with it, e.g. drawing.
//! glyph.draw(|x, y, v| {}); // In this case the pixel values are not used.
//! ```
//!
//! # Unicode terminology
//!
//! This crate uses terminology for computerised typography as specified by the Unicode standard. If you are
//! not sure of the differences between a code point, a character, and a glyph, you may want to check the
//! [official Unicode glossary](http://unicode.org/glossary/), or alternatively, here's my take on it from a
//! practical perspective:
//!
//! * A character is what you would conventionally call a single symbol, independent of its appearance or
//!   representation in a particular font. Examples include `a`, `A`, `ä`, `å`, `1`, `*`, `Ω`, etc.
//! * A Unicode code point is the particular number that the Unicode standard associates with a particular
//!   character.
//!   Note however that code points also exist for things not conventionally thought of as characters by
//!   themselves, but can be combined to form characters, such as diacritics like accents. These
//!   "characters" are known in Unicode as "combining characters".
//!   E.g., a diaeresis (`¨`) has the code point U+0308. If this code point follows the code point U+0055
//!   (the letter `u`), this sequence represents the character `ü`. Note that there is also a
//!   single codepoint for `ü`, U+00FC. This means that what visually looks like the same string can have
//!   multiple different Unicode representations. Some fonts will have glyphs (see below) for one sequence of
//!   codepoints, but not another that has the same meaning. To deal with this problem it is recommended to use
//!   Unicode normalisation, as provided by, for example, the
//!   [unicode-normalization](http://crates.io/crates/unicode-normalization) crate, to convert to code point
//!   sequences that work with the font in question. Typically a font is more likely to support a single code
//!   point vs. a sequence with the same meaning, so the best normalisation to use is "canonical recomposition",
//!   known as NFC in the normalisation crate.
//! * A glyph is a particular font's shape to draw the character for a particular Unicode code point. This will
//!   have its own identifying number unique to the font, its ID.
extern crate arrayvec;
extern crate stb_truetype;

mod geometry;
mod rasterizer;

use std::borrow::Cow;
use std::ops;
use std::sync::Arc;

pub use geometry::{Rect, Point, point, Vector, vector, Line, Curve};
use stb_truetype as tt;

/// A collection of fonts read straight from a font file's data. The data in the collection is not validated.
/// This structure may or may not own the font data.
pub struct FontCollection<'a>(Cow<'a, [u8]>);
/// A single font. This may or may not own the font data.
pub struct Font<'a> {
    info: tt::FontInfo<'a>
}
/// A newtype wrapper for `Cow<[u8]>` that can be targetted by `From` and `Into`. This is intended
/// to be used for providing convenient APIs that can accept `Vec<u8>`s or `&[u8]`s or other types
/// that can be read as `[u8]`s (feel free to provide your own `From` implementations).
pub struct Bytes<'a>(pub Cow<'a, [u8]>);
/// Represents a Unicode code point.
#[derive(Copy, Clone, Debug)]
pub struct Codepoint(pub u32);
/// Represents either a Unicode code point, or a glyph identifier for a font.
///
/// This is used as input for functions that can accept code points or glyph identifiers.
///
/// You typically won't construct this type directly, instead relying on `From` and `Into`.
#[derive(Copy, Clone, Debug)]
pub enum CodepointOrGlyphId {
    Codepoint(Codepoint),
    GlyphId(GlyphId)
}
/// Represents a glyph identifier for a particular font. This identifier will not necessarily correspond to
/// the correct glyph in a font other than the one that it was obtained from.
#[derive(Copy, Clone, Debug)]
pub struct GlyphId(pub u32);
/// A single glyph of a font. this may either be a thin wrapper referring to the font and the glyph id, or
/// it may be a standalone glyph that owns the data needed by it.
///
/// A `Glyph` does not have an inherent scale or position associated with it. To augment a glyph with a
/// size, give it a scale using `scaled`. You can then position it using `positioned`.
#[derive(Clone)]
pub struct Glyph<'a> {
    inner: GlyphInner<'a>
}

#[derive(Clone)]
enum GlyphInner<'a> {
    Proxy(&'a Font<'a>, u32),
    Shared(Arc<SharedGlyphData>)
}

struct SharedGlyphData {
    id: u32,
    extents: Option<Rect<i32>>,
    scale_for_1_pixel: f32,
    unit_h_metrics: HMetrics,
    shape: Option<Vec<tt::Vertex>>
}
/// The "horizontal metrics" of a glyph. This is useful for calculating the horizontal offset of a glyph
/// from the previous one in a string when laying a string out horizontally.
#[derive(Copy, Clone, Debug)]
pub struct HMetrics {
    /// The horizontal offset that the origin of the next glyph should be from the origin of this glyph.
    pub advance_width: f32,
    /// The horizontal offset between the origin of this glyph and the leftmost edge/point of the glyph.
    pub left_side_bearing: f32
}
#[derive(Copy, Clone, Debug)]
/// The "vertical metrics" of a font at a particular scale. This is useful for calculating the amount of
/// vertical space to give a line of text, and for computing the vertical offset between successive lines.
pub struct VMetrics {
    /// The highest point that any glyph in the font extends to above the baseline. Typically positive.
    pub ascent: f32,
    /// The lowest point that any glyph in the font extends to below the baseline. Typically negative.
    pub descent: f32,
    /// The gap to leave between the descent of one line and the ascent of the next. This is of
    /// course only a guideline given by the font's designers.
    pub line_gap: f32
}
/// A glyph augmented with scaling information. You can query such a glyph for information that depends
/// on the scale of the glyph.
#[derive(Clone)]
pub struct SizedGlyph<'a> {
    g: Glyph<'a>,
    scale: Vector<f32>
}
/// A glyph augmented with positioning and scaling information. You can query such a glyph for information
/// that depends on the scale and position of the glyph.
#[derive(Clone)]
pub struct PositionedGlyph<'a> {
    sg: SizedGlyph<'a>,
    position: Point<f32>,
    bb: Option<Rect<i32>>
}
/// A uniform font scaling that makes the height of the rendered font a specific number of pixels. For example,
/// if you want to render a font with a height of 20 pixels, use `Pixels(20.0)`.
#[derive(Copy, Clone)]
pub struct Pixels(pub f32);
/// A nonuniform font scaling. `PixelsXY(x, y)` produces a scaling that makes the height of the rendered font
/// `y` pixels high, with a horizontal scale factor on top of that of `x/y`. For example, if you want to render
/// a font with a height of 20 pixels, but have it horizontally stretched by a factor of two, use
/// `PixelsXY(40.0, 20.0)`.
#[derive(Copy, Clone)]
pub struct PixelsXY(pub f32, pub f32);
/// An opaque struct representing a common format for font scaling. You typically won't use this struct directly,
/// instead using `Pixels` or `PixelsXY` and the `Into` trait to pass them to functions.
///
/// You can however implement your own scales for use with functions that accept `S: Into<Scale>`. As a simplified
/// example, if you want to write scales in a different unit, like inches, and you know that there are 96 pixels
/// in an inch in your use case, you can create a struct like the following:
///
/// ```
/// use rusttype::{Pixels, Scale};
///
/// struct Inches(f32);
/// impl From<Inches> for Scale {
///     fn from(i: Inches) -> Scale {
///         Pixels(i.0 * 96.0).into()
///     }
/// }
/// ```
///
/// You can then use `Inches` wherever you could use `Pixels` or `PixelsXY` before.
#[derive(Copy, Clone)]
pub struct Scale(f32, f32);
impl From<Pixels> for Scale {
    fn from(p: Pixels) -> Scale {
        Scale(p.0, p.0)
    }
}
impl From<PixelsXY> for Scale {
    fn from(p: PixelsXY) -> Scale {
        Scale(p.0, p.1)
    }
}
impl From<Box<[u8]>> for Bytes<'static> {
    fn from(b: Box<[u8]>) -> Bytes<'static> {
        Bytes(Cow::Owned(b.into_vec()))
    }
}
impl From<Vec<u8>> for Bytes<'static> {
    fn from(v: Vec<u8>) -> Bytes<'static> {
        Bytes(Cow::Owned(v))
    }
}
impl<'a> From<Cow<'a, [u8]>> for Bytes<'a> {
    fn from(c: Cow<'a, [u8]>) -> Bytes<'a> {
        Bytes(c)
    }
}
impl<'a> From<&'a [u8]> for Bytes<'a> {
    fn from(s: &'a [u8]) -> Bytes<'a> {
        Bytes(Cow::Borrowed(s))
    }
}
impl From<char> for Codepoint {
    fn from(c: char) -> Codepoint {
        Codepoint(c as u32)
    }
}
impl From<Codepoint> for CodepointOrGlyphId {
    fn from(c: Codepoint) -> CodepointOrGlyphId {
        CodepointOrGlyphId::Codepoint(c)
    }
}
impl From<GlyphId> for CodepointOrGlyphId {
    fn from(g: GlyphId) -> CodepointOrGlyphId {
        CodepointOrGlyphId::GlyphId(g)
    }
}
impl From<char> for CodepointOrGlyphId {
    fn from(c: char) -> CodepointOrGlyphId {
        Codepoint(c as u32).into()
    }
}
impl<'a> FontCollection<'a> {
    /// Constructs a font collection from an array of bytes, typically loaded from a font file.
    /// This array may be owned (e.g. `Vec<u8>`), or borrowed (`&[u8]`).
    /// As long as `From<T>` is implemented for `Bytes` for some type `T`, `T` can be used as input.
    pub fn from_bytes<B: Into<Bytes<'a>>>(bytes: B) -> FontCollection<'a> {
        FontCollection(bytes.into().0)
    }
    /// In the common case that a font collection consists of only one font, this function
    /// consumes this font collection and turns it into a font. If this is not the case,
    /// or the font is not valid (read: not supported by this library), `None` is returned.
    pub fn into_font(self) -> Option<Font<'a>> {
        if tt::is_font(&self.0) && tt::get_font_offset_for_index(&self.0, 1).is_none() {
            tt::FontInfo::new(self.0, 0).map(
                |info| Font {
                    info: info
                })
        } else {
            None
        }
    }
    /// Gets the font at index `i` in the font collection, if it exists and is valid.
    /// The produced font borrows the font data that is either borrowed or owned by this font collection.
    pub fn font_at(&self, i: usize) -> Option<Font> {
        use std::borrow::{Cow, Borrow};
        tt::get_font_offset_for_index(&self.0, i as i32)
            .and_then(|o| tt::FontInfo::new(Cow::Borrowed(self.0.borrow()), o as usize))
            .map(|info| Font { info: info })
    }
}
impl<'a> Font<'a> {

    /// The "vertical metrics" for this font at a given scale. These metrics are shared by all of the glyphs
    /// in the font.
    /// See `VMetrics` for more detail.
    pub fn v_metrics<S: Into<Scale>>(&self, scale: S) -> VMetrics {
        let scale = scale.into();
        let vm = self.info.get_v_metrics();
        let scale = self.info.scale_for_pixel_height(scale.1);
        VMetrics {
            ascent: vm.ascent as f32 * scale,
            descent: vm.descent as f32 * scale,
            line_gap: vm.line_gap as f32 * scale
        }
    }

    /// The number of glyphs present in this font. Glyph identifiers for this font will always be in the range
    /// `0..self.glyph_count()`
    pub fn glyph_count(&self) -> usize {
        self.info.get_num_glyphs() as usize
    }

    /// Returns the corresponding glyph for a Unicode code point or a glyph id for this font.
    /// If id corresponds to a glyph identifier, the identifier must be valid (smaller than `self.glyph_count()`),
    /// otherwise `None` is returned.
    ///
    /// Note that code points without corresponding glyphs in this font map to the "undef" glyph, glyph 0.
    pub fn glyph<C: Into<CodepointOrGlyphId>>(&self, id: C) -> Option<Glyph> {
        let gid = match id.into() {
            CodepointOrGlyphId::Codepoint(Codepoint(c)) => self.info.find_glyph_index(c),
            CodepointOrGlyphId::GlyphId(GlyphId(gid)) => gid
        };
        Some(Glyph::new(GlyphInner::Proxy(self, gid)))
    }
    /// A convenience function.
    ///
    /// Returns an iterator that produces the glyphs corresponding to the code points or glyph ids produced
    /// by the given iterator `itr`.
    ///
    /// This is equivalent in behaviour to `itr.map(|c| font.glyph(c).unwrap())`.
    pub fn glyphs_for<I: Iterator>(&self, itr: I) -> GlyphIter<I> where I::Item: Into<CodepointOrGlyphId> {
        GlyphIter {
            font: self,
            itr: itr
        }
    }
    /// A convenience function for laying out glyphs for a string horizontally. It does not take control
    /// characters like line breaks into account, as treatment of these is likely to depend on the application.
    ///
    /// Note that this function does not perform Unicode normalisation. Composite characters (such as ö
    /// constructed from two code points, ¨ and o), will not be normalised to single code points. So if a font
    /// does not contain a glyph for each separate code point, but does contain one for the normalised single
    /// code point (which is common), the desired glyph will not be produced, despite being present in the font.
    /// Deal with this by performing Unicode normalisation on the input string before passing it to `layout`.
    /// The crate [unicode-normalization](http://crates.io/crates/unicode-normalization) is perfect for this
    /// purpose.
    ///
    /// Calling this function is equivalent to a longer sequence of operations involving `glyphs_for`, e.g.
    ///
    /// ```no_run
    /// # use rusttype::*;
    /// # let (scale, start) = (Pixels(0.0), point(0.0, 0.0));
    /// # let font: Font = unimplemented!();
    /// font.layout("Hello World!", scale, start)
    /// # ;
    /// ```
    ///
    /// produces an iterator with behaviour equivalent to the following:
    ///
    /// ```no_run
    /// # use rusttype::*;
    /// # let (scale, start) = (Pixels(0.0), point(0.0, 0.0));
    /// # let font: Font = unimplemented!();
    /// font.glyphs_for("Hello World!".chars())
    ///     .scan((None, 0.0), |&mut (mut last, mut x), g| {
    ///         let g = g.scaled(scale);
    ///         let w = g.h_metrics().advance_width
    ///             + last.map(|last| font.pair_kerning(scale, last, g.id())).unwrap_or(0.0);
    ///         let next = g.positioned(start + vector(x, 0.0));
    ///         last = Some(next.id());
    ///         x += w;
    ///         Some(next)
    ///     })
    /// # ;
    /// ```
    pub fn layout<'b, S: Into<Scale>>(&'b self, s: &'b str, scale: S, start: Point<f32>) -> LayoutIter {
        LayoutIter {
            font: self,
            chars: s.chars(),
            caret: 0.0,
            scale: scale.into(),
            start: start,
            last_glyph: None
        }
    }
    /// Returns additional kerning to apply as well as that given by HMetrics for a particular pair of glyphs.
    pub fn pair_kerning<S, A, B>(&self, scale: S, first: A, second: B) -> f32
        where S: Into<Scale>, A: Into<CodepointOrGlyphId>, B: Into<CodepointOrGlyphId>
    {
        let (first, second) = (self.glyph(first).unwrap(), self.glyph(second).unwrap());
        let scale = scale.into();
        let factor = self.info.scale_for_pixel_height(scale.1) * (scale.0 / scale.1);
        let kern = self.info.get_glyph_kern_advance(first.id().0, second.id().0);
        factor * kern as f32
    }
}
pub struct GlyphIter<'a, I: Iterator> where I::Item: Into<CodepointOrGlyphId> {
    font: &'a Font<'a>,
    itr: I
}
impl<'a, I: Iterator> Iterator for GlyphIter<'a, I> where I::Item: Into<CodepointOrGlyphId> {
    type Item = Glyph<'a>;
    fn next(&mut self) -> Option<Glyph<'a>> {
        self.itr.next().map(|c| self.font.glyph(c).unwrap())
    }
}
pub struct LayoutIter<'a> {
    font: &'a Font<'a>,
    chars: ::std::str::Chars<'a>,
    caret: f32,
    scale: Scale,
    start: Point<f32>,
    last_glyph: Option<GlyphId>
}
impl<'a> Iterator for LayoutIter<'a> {
    type Item = PositionedGlyph<'a>;
    fn next(&mut self) -> Option<PositionedGlyph<'a>> {
        self.chars.next().map(|c| {
            let g = self.font.glyph(c).unwrap().scaled(self.scale);
            if let Some(last) = self.last_glyph {
                self.caret += self.font.pair_kerning(self.scale, last, g.id());
            }
            let g = g.positioned(point(self.start.x + self.caret, self.start.y));
            self.caret += g.h_metrics().advance_width;
            g
        })
    }
}
impl<'a> Glyph<'a> {
    fn new(inner: GlyphInner) -> Glyph {
        Glyph {
            inner: inner
        }
    }
    /// The font to which this glyph belongs. If the glyph is a standalone glyph that owns its resources,
    /// it no longer has a reference to the font which it was created from (using `standalone()`). In which
    /// case, `None` is returned.
    pub fn font(&self) -> Option<&Font<'a>> {
        match self.inner {
            GlyphInner::Proxy(f, _) => Some(f),
            GlyphInner::Shared(_) => None
        }
    }
    /// The glyph identifier for this glyph.
    pub fn id(&self) -> GlyphId {
        match self.inner {
            GlyphInner::Proxy(_, id) => GlyphId(id),
            GlyphInner::Shared(ref data) => GlyphId(data.id),
        }
    }
    /// Augments this glyph with scaling information, making methods that depend on the scale of the glyph
    /// available.
    pub fn scaled<S: Into<Scale>>(self, scale: S) -> SizedGlyph<'a> {
        let scale = scale.into();
        let (scale_x, scale_y) = match self.inner {
            GlyphInner::Proxy(font, _) => {
                let scale_y = font.info.scale_for_pixel_height(scale.1);
                let scale_x = scale_y * scale.0 / scale.1;
                (scale_x, scale_y)
            }
            GlyphInner::Shared(ref data) => {
                let scale_y = data.scale_for_1_pixel * scale.1;
                let scale_x = scale_y * scale.0 / scale.1;
                (scale_x, scale_y)
            }
        };
        SizedGlyph {
            g: self,
            scale: vector(scale_x, scale_y)
        }
    }
    /// Turns a `Glyph<'a>` into a `Glyph<'static>`. This produces a glyph that owns its resources,
    /// extracted from the font. This glyph can outlive the font that it comes from.
    ///
    /// Calling `standalone()` on a standalone glyph shares the resources, and is equivalent to `clone()`.
    pub fn standalone(&self) -> Glyph<'static> {
        match self.inner {
            GlyphInner::Proxy(font, id) => Glyph::new(GlyphInner::Shared(Arc::new(SharedGlyphData {
                id: id,
                scale_for_1_pixel: font.info.scale_for_pixel_height(1.0),
                unit_h_metrics: {
                    let hm = font.info.get_glyph_h_metrics(id);
                    HMetrics {
                        advance_width: hm.advance_width as f32,
                        left_side_bearing: hm.left_side_bearing as f32
                    }
                },
                extents: font.info.get_glyph_box(id).map(|bb| Rect {
                    min: point(bb.x0 as i32, bb.y0 as i32),
                    max: point(bb.x1 as i32, bb.y1 as i32)
                }),
                shape: font.info.get_glyph_shape(id)
            }))),
            GlyphInner::Shared(ref data) => Glyph::new(GlyphInner::Shared(data.clone()))
        }
    }
}
/// Part of a `Contour`, either a `Line` or a `Curve`.
#[derive(Copy, Clone, Debug)]
pub enum Segment {
    Line(Line),
    Curve(Curve)
}
/// A closed loop consisting of a sequence of `Segment`s.
#[derive(Clone, Debug)]
pub struct Contour {
    pub segments: Vec<Segment>
}
impl<'a> SizedGlyph<'a> {
    /// Augments this glyph with positioning information, making methods that depend on the position of the
    /// glyph available.
    pub fn positioned(self, p: Point<f32>) -> PositionedGlyph<'a> {
        let bb = match self.inner {
            GlyphInner::Proxy(font, id) => {
                font.info.get_glyph_bitmap_box_subpixel(id,
                                                        self.scale.x, self.scale.y,
                                                        p.x, p.y)
                    .map(|bb| Rect {
                        min: point(bb.x0, bb.y0),
                        max: point(bb.x1, bb.y1)
                    })
            }
            GlyphInner::Shared(ref data) => {
                data.extents.map(|bb| Rect {
                    min: point((bb.min.x as f32 * self.scale.x).floor() as i32,
                               (bb.min.y as f32 * self.scale.y).floor() as i32),
                    max: point((bb.max.x as f32 * self.scale.x).ceil() as i32,
                               (bb.max.y as f32 * self.scale.y).ceil() as i32)
                })
            }
        };
        PositionedGlyph {
            sg: self,
            position: p,
            bb: bb
        }
    }
    /// Retrieves the "horizontal metrics" of this glyph. See `HMetrics` for more detail.
    pub fn h_metrics(&self) -> HMetrics {
        match self.inner {
            GlyphInner::Proxy(font, id) => {
                let hm = font.info.get_glyph_h_metrics(id);
                HMetrics {
                    advance_width: hm.advance_width as f32 * self.scale.x,
                    left_side_bearing: hm.left_side_bearing as f32 * self.scale.x
                }
            }
            GlyphInner::Shared(ref data) => {
                HMetrics {
                    advance_width: data.unit_h_metrics.advance_width * self.scale.x,
                    left_side_bearing: data.unit_h_metrics.left_side_bearing * self.scale.y
                }
            }
        }
    }
    fn shape_with_offset(&self, offset: Point<f32>) -> Option<Vec<Contour>> {
        use stb_truetype::VertexType;
        use std::mem::replace;
        match self.inner {
            GlyphInner::Proxy(font, id) => font.info.get_glyph_shape(id),
            GlyphInner::Shared(ref data) => data.shape.clone()
        }.map(|shape| {
            let mut result = Vec::new();
            let mut current = Vec::new();
            let mut last = point(0.0, 0.0);
            for v in shape {
                let end = point(v.x as f32 * self.scale.x + offset.x,
                                v.y as f32 * self.scale.y + offset.y);
                match v.vertex_type() {
                    VertexType::MoveTo if result.len() != 0 => {
                        result.push(Contour {
                            segments: replace(&mut current, Vec::new())
                        })
                    }
                    VertexType::LineTo => {
                        current.push(Segment::Line(Line {
                            p: [last, end]
                        }))
                    }
                    VertexType::CurveTo => {
                        let control = point(v.cx as f32 * self.scale.x + offset.x,
                                            v.cy as f32 * self.scale.y + offset.y);
                        current.push(Segment::Curve(Curve {
                            p: [last, control, end]
                        }))
                    }
                    _ => ()
                }
                last = end;
                }
            if current.len() > 0 {
                result.push(Contour {
                    segments: replace(&mut current, Vec::new())
                });
            }
            result
            })
    }
    /// Produces a list of the contours that make up the shape of this glyph. Each contour consists of
    /// a sequence of segments. Each segment is either a straight `Line` or a `Curve`.
    ///
    /// The winding of the produced contours is clockwise for closed shapes, anticlockwise for holes.
    pub fn shape(&self) -> Option<Vec<Contour>> {
        self.shape_with_offset(point(0.0, 0.0))
    }
    /// The bounding box of the shape of this glyph, not to be confused with `pixel_bounding_box`, the
    /// conservative pixel-boundary bounding box. The coordinates are relative to the glyph's origin.
    pub fn exact_bounding_box(&self) -> Option<Rect<f32>> {
        match self.inner {
            GlyphInner::Proxy(font, id) => font.info.get_glyph_box(id).map(|bb| {
                Rect {
                    min: point(bb.x0 as f32 * self.scale.x, bb.y0 as f32 * self.scale.y),
                    max: point(bb.x1 as f32 * self.scale.x, bb.y1 as f32 * self.scale.y)
                }
            }),
            GlyphInner::Shared(ref data) => data.extents.map(|bb| Rect {
                min: point(bb.min.x as f32 * self.scale.x, bb.min.y as f32 * self.scale.y),
                max: point(bb.max.x as f32 * self.scale.x, bb.max.y as f32 * self.scale.y)
            })
        }
    }
    /// Constructs a glyph that owns its data from this glyph. This is similar to `Glyph::standalone`. See
    /// that function for more details.
    pub fn standalone(&self) -> SizedGlyph<'static> {
        SizedGlyph {
            g: self.g.standalone(),
            scale: self.scale
        }
    }
}
impl<'a> ops::Deref for SizedGlyph<'a> {
    type Target = Glyph<'a>;
    fn deref(&self) -> &Glyph<'a> {
        &self.g
    }
}

impl<'a> PositionedGlyph<'a> {
    /// The conservative pixel-boundary bounding box for this glyph. This is the smallest rectangle
    /// aligned to pixel boundaries that encloses the shape of this glyph at this position.
    pub fn pixel_bounding_box(&self) -> Option<Rect<i32>> {
        self.bb
    }
    /// Similar to `shape()`, but with the position of the glyph taken into account.
    pub fn positioned_shape(&self) -> Option<Vec<Contour>> {
        self.shape_with_offset(self.position)
    }
    /// Rasterises this glyph. For each pixel in the rect given by `pixel_bounding_box()`, `o` is called:
    ///
    /// ```ignore
    /// o(x, y, v)
    /// ```
    ///
    /// where `x` and `y` are the coordinates of the pixel relative to the `min` coordinates of the bounding box,
    /// and `v` is the analytically calculated coverage of the pixel by the shape of the glyph.
    /// Calls to `o` proceed in horizontal scanline order, similar to this pseudo-code:
    ///
    /// ```ignore
    /// let bb = glyph.pixel_bounding_box();
    /// for y in 0..bb.height() {
    ///     for x in 0..bb.width() {
    ///         o(x, y, calc_coverage(&glyph, x, y));
    ///     }
    /// }
    /// ```
    pub fn draw<O: FnMut(u32, u32, f32)>(&self, o: O) {
        use geometry::{Line, Curve};
        use stb_truetype::VertexType;
        let shape = match self.inner {
            GlyphInner::Proxy(font, id) => font.info.get_glyph_shape(id).unwrap_or_else(|| Vec::new()),
            GlyphInner::Shared(ref data) => data.shape.clone().unwrap_or_else(|| Vec::new())
        };
        let bb = if let Some(bb) = self.bb.as_ref() {
            bb
        } else {
            return
        };
        let offset = vector(bb.min.x as f32, bb.min.y as f32);
        let mut lines = Vec::new();
        let mut curves = Vec::new();
        let mut last = point(0.0, 0.0);
        for v in shape {
            let end = point(v.x as f32 * self.sg.scale.x + self.position.x,
                            -v.y as f32 * self.sg.scale.y + self.position.y)
                - offset;
            match v.vertex_type() {
                VertexType::LineTo => {
                    lines.push(Line {
                        p: [last, end]
                    })
                }
                VertexType::CurveTo => {
                    let control = point(v.cx as f32 * self.sg.scale.x + self.position.x,
                                        -v.cy as f32 * self.sg.scale.y + self.position.y)
                        - offset;
                    curves.push(Curve {
                        p: [last, control, end]
                    })
                }
                VertexType::MoveTo => {}
            }
            last = end;
        }
        rasterizer::rasterize(&lines, &curves,
                              (bb.max.x - bb.min.x) as u32,
                              (bb.max.y - bb.min.y) as u32,
                              o);
    }
    /// Constructs a glyph that owns its data from this glyph. This is similar to `Glyph::standalone`. See
    /// that function for more details.
    pub fn standalone(&self) -> PositionedGlyph<'static> {
        PositionedGlyph {
            sg: self.sg.standalone(),
            bb: self.bb,
            position: self.position
        }
    }
}
impl<'a> ops::Deref for PositionedGlyph<'a> {
    type Target = SizedGlyph<'a>;
    fn deref(&self) -> &SizedGlyph<'a> {
        &self.sg
    }
}
