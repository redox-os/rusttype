//! RustType is a pure Rust alternative to libraries like FreeType.
//!
//! The current capabilities of RustType:
//!
//! * Reading TrueType formatted fonts and font collections. This includes
//!   `*.ttf` as well as a subset of `*.otf` font files.
//! * Retrieving glyph shapes and commonly used properties for a font and its
//!   glyphs.
//! * Laying out glyphs horizontally using horizontal and vertical metrics, and
//!   glyph-pair-specific kerning.
//! * Rasterising glyphs with sub-pixel positioning using an accurate analytical
//!   algorithm (not based on sampling).
//! * Managing a font cache on the GPU with the `gpu_cache` module. This keeps
//!   recently used glyph renderings in a dynamic cache in GPU memory to
//!   minimise texture uploads per-frame. It also allows you keep the draw call
//!   count for text very low, as all glyphs are kept in one GPU texture.
//!
//! Notable things that RustType does not support *yet*:
//!
//! * OpenType formatted fonts that are not just TrueType fonts (OpenType is a
//!   superset of TrueType). Notably there is no support yet for cubic Bezier
//!   curves used in glyphs.
//! * Font hinting.
//! * Ligatures of any kind.
//! * Some less common TrueType sub-formats.
//! * Right-to-left and vertical text layout.
//!
//! # Getting Started
//!
//! To hit the ground running with RustType, look at the `simple.rs` example
//! supplied with the crate. It demonstrates loading a font file, rasterising an
//! arbitrary string, and displaying the result as ASCII art. If you prefer to
//! just look at the documentation, the entry point for loading fonts is
//! `FontCollection`, from which you can access individual fonts, then their
//! glyphs.
//!
//! # Glyphs
//!
//! The glyph API uses wrapper structs to augment a glyph with information such
//! as scaling and positioning, making relevant methods that make use of this
//! information available as appropriate. For example, given a `Glyph` `glyph`
//! obtained directly from a `Font`:
//!
//! ```no_run
//! # use rusttype::*;
//! # let glyph: Glyph<'static> = unimplemented!();
//! // One of the few things you can do with an unsized, positionless glyph is get its id.
//! let id = glyph.id();
//! let glyph = glyph.scaled(Scale::uniform(10.0));
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
//! This crate uses terminology for computerised typography as specified by the
//! Unicode standard. If you are not sure of the differences between a code
//! point, a character, and a glyph, you may want to check the [official Unicode
//! glossary](http://unicode.org/glossary/), or alternatively, here's my take on
//! it from a practical perspective:
//!
//! * A character is what you would conventionally call a single symbol,
//!   independent of its appearance or representation in a particular font.
//!   Examples include `a`, `A`, `ä`, `å`, `1`, `*`, `Ω`, etc.
//! * A Unicode code point is the particular number that the Unicode standard
//!   associates with a particular character. Note however that code points also
//!   exist for things not conventionally thought of as characters by
//!   themselves, but can be combined to form characters, such as diacritics
//!   like accents. These "characters" are known in Unicode as "combining
//!   characters". E.g., a diaeresis (`¨`) has the code point U+0308. If this
//!   code point follows the code point U+0055 (the letter `u`), this sequence
//!   represents the character `ü`. Note that there is also a single codepoint
//!   for `ü`, U+00FC. This means that what visually looks like the same string
//!   can have multiple different Unicode representations. Some fonts will have
//!   glyphs (see below) for one sequence of codepoints, but not another that
//!   has the same meaning. To deal with this problem it is recommended to use
//!   Unicode normalisation, as provided by, for example, the
//!   [unicode-normalization](http://crates.io/crates/unicode-normalization)
//!   crate, to convert to code point sequences that work with the font in
//!   question. Typically a font is more likely to support a single code point
//!   vs. a sequence with the same meaning, so the best normalisation to use is
//!   "canonical recomposition", known as NFC in the normalisation crate.
//! * A glyph is a particular font's shape to draw the character for a
//!   particular Unicode code point. This will have its own identifying number
//!   unique to the font, its ID.
#![allow(
    clippy::cognitive_complexity,
    clippy::doc_markdown,
    clippy::cast_lossless,
    clippy::many_single_char_names
)]
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

mod geometry;
mod rasterizer;

#[cfg(all(feature = "libm-math", not(feature = "std")))]
mod nostd_float;

#[cfg(feature = "gpu_cache")]
pub mod gpu_cache;

pub use crate::geometry::{point, vector, Curve, Line, Point, Rect, Vector};
use approx::relative_eq;
use core::fmt;
use stb_truetype as tt;

#[cfg(not(feature = "has-atomics"))]
use alloc::rc::Rc as Arc;
#[cfg(feature = "has-atomics")]
use alloc::sync::Arc;

#[cfg(not(feature = "std"))]
use alloc::{boxed::Box, vec::Vec};
#[cfg(all(feature = "libm-math", not(feature = "std")))]
use crate::nostd_float::FloatExt;

/// A collection of fonts read straight from a font file's data. The data in the
/// collection is not validated. This structure may or may not own the font
/// data.
///
/// # Lifetime
/// The lifetime reflects the font data lifetime. `FontCollection<'static>`
/// covers most cases ie both dynamically loaded owned data and for referenced
/// compile time font data.
#[derive(Clone, Debug)]
pub struct FontCollection<'a>(SharedBytes<'a>);
/// A single font. This may or may not own the font data.
///
/// # Lifetime
/// The lifetime reflects the font data lifetime. `Font<'static>` covers most
/// cases ie both dynamically loaded owned data and for referenced compile time
/// font data.
///
/// # Example
///
/// ```
/// # use rusttype::{Font, Error};
/// # fn example() -> Result<(), Error> {
/// let font_data: &[u8] = include_bytes!("../dev/fonts/dejavu/DejaVuSansMono.ttf");
/// let font: Font<'static> = Font::from_bytes(font_data)?;
///
/// let owned_font_data: Vec<u8> = font_data.to_vec();
/// let from_owned_font: Font<'static> = Font::from_bytes(owned_font_data)?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct Font<'a> {
    info: tt::FontInfo<SharedBytes<'a>>,
}

impl fmt::Debug for Font<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Font")
    }
}

/// `SharedBytes` handles the lifetime of font data used in RustType. The data
/// is either a shared reference to externally owned data, or managed by
/// reference counting. `SharedBytes` can be conveniently used with `From` and
/// `Into`, and dereferences to the contained bytes.
///
/// # Lifetime
/// The lifetime reflects the font data lifetime. `SharedBytes<'static>` covers
/// most cases ie both dynamically loaded owned data and for referenced compile
/// time font data.
#[derive(Clone, Debug)]
pub enum SharedBytes<'a> {
    ByRef(&'a [u8]),
    ByArc(Arc<[u8]>),
}

impl<'a> core::ops::Deref for SharedBytes<'a> {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        match *self {
            SharedBytes::ByRef(bytes) => bytes,
            SharedBytes::ByArc(ref bytes) => &**bytes,
        }
    }
}
/// ```
/// # use rusttype::SharedBytes;
/// let bytes: &[u8] = &[0u8, 1, 2, 3];
/// let shared: SharedBytes = bytes.into();
/// assert_eq!(&*shared, bytes);
/// ```
impl<'a> From<&'a [u8]> for SharedBytes<'a> {
    fn from(bytes: &'a [u8]) -> SharedBytes<'a> {
        SharedBytes::ByRef(bytes)
    }
}
/// ```
/// # use rusttype::SharedBytes;
/// # use std::sync::Arc;
/// let bytes: Arc<[u8]> = vec![0u8, 1, 2, 3].into();
/// let shared: SharedBytes = Arc::clone(&bytes).into();
/// assert_eq!(&*shared, &*bytes);
/// ```
impl From<Arc<[u8]>> for SharedBytes<'static> {
    fn from(bytes: Arc<[u8]>) -> SharedBytes<'static> {
        SharedBytes::ByArc(bytes)
    }
}
/// ```
/// # use rusttype::SharedBytes;
/// let bytes: Box<[u8]> = vec![0u8, 1, 2, 3].into();
/// let shared: SharedBytes = bytes.into();
/// assert_eq!(&*shared, &[0u8, 1, 2, 3]);
/// ```
impl From<Box<[u8]>> for SharedBytes<'static> {
    fn from(bytes: Box<[u8]>) -> SharedBytes<'static> {
        SharedBytes::ByArc(bytes.into())
    }
}
/// ```
/// # use rusttype::SharedBytes;
/// let bytes = vec![0u8, 1, 2, 3];
/// let shared: SharedBytes = bytes.into();
/// assert_eq!(&*shared, &[0u8, 1, 2, 3]);
/// ```
impl From<Vec<u8>> for SharedBytes<'static> {
    fn from(bytes: Vec<u8>) -> SharedBytes<'static> {
        SharedBytes::ByArc(bytes.into())
    }
}
/// ```
/// # use rusttype::SharedBytes;
/// let bytes = vec![0u8, 1, 2, 3];
/// let shared: SharedBytes = (&bytes).into();
/// assert_eq!(&*shared, &bytes as &[u8]);
/// ```
impl<'a, T: AsRef<[u8]>> From<&'a T> for SharedBytes<'a> {
    fn from(bytes: &'a T) -> SharedBytes<'a> {
        SharedBytes::ByRef(bytes.as_ref())
    }
}

/// Represents a Unicode code point.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Codepoint(pub u32);
/// Represents a glyph identifier for a particular font. This identifier will
/// not necessarily correspond to the correct glyph in a font other than the
/// one that it was obtained from.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GlyphId(pub u32);
/// A single glyph of a font. this may either be a thin wrapper referring to the
/// font and the glyph id, or it may be a standalone glyph that owns the data
/// needed by it.
///
/// A `Glyph` does not have an inherent scale or position associated with it. To
/// augment a glyph with a size, give it a scale using `scaled`. You can then
/// position it using `positioned`.
#[derive(Clone)]
pub struct Glyph<'a> {
    inner: GlyphInner<'a>,
}

impl fmt::Debug for Glyph<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Glyph").field("id", &self.id().0).finish()
    }
}

#[derive(Clone)]
enum GlyphInner<'a> {
    Proxy(Font<'a>, u32),
    Shared(Arc<SharedGlyphData>),
}

#[derive(Debug)]
pub struct SharedGlyphData {
    pub id: u32,
    pub extents: Option<Rect<i32>>,
    pub scale_for_1_pixel: f32,
    pub unit_h_metrics: HMetrics,
    pub shape: Option<Vec<tt::Vertex>>,
}
/// The "horizontal metrics" of a glyph. This is useful for calculating the
/// horizontal offset of a glyph from the previous one in a string when laying a
/// string out horizontally.
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
pub struct HMetrics {
    /// The horizontal offset that the origin of the next glyph should be from
    /// the origin of this glyph.
    pub advance_width: f32,
    /// The horizontal offset between the origin of this glyph and the leftmost
    /// edge/point of the glyph.
    pub left_side_bearing: f32,
}
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
/// The "vertical metrics" of a font at a particular scale. This is useful for
/// calculating the amount of vertical space to give a line of text, and for
/// computing the vertical offset between successive lines.
pub struct VMetrics {
    /// The highest point that any glyph in the font extends to above the
    /// baseline. Typically positive.
    pub ascent: f32,
    /// The lowest point that any glyph in the font extends to below the
    /// baseline. Typically negative.
    pub descent: f32,
    /// The gap to leave between the descent of one line and the ascent of the
    /// next. This is of course only a guideline given by the font's designers.
    pub line_gap: f32,
}

impl From<tt::VMetrics> for VMetrics {
    fn from(vm: tt::VMetrics) -> Self {
        Self {
            ascent: vm.ascent as f32,
            descent: vm.descent as f32,
            line_gap: vm.line_gap as f32,
        }
    }
}

impl core::ops::Mul<f32> for VMetrics {
    type Output = VMetrics;

    fn mul(self, rhs: f32) -> Self {
        Self {
            ascent: self.ascent * rhs,
            descent: self.descent * rhs,
            line_gap: self.line_gap * rhs,
        }
    }
}

/// A glyph augmented with scaling information. You can query such a glyph for
/// information that depends on the scale of the glyph.
#[derive(Clone)]
pub struct ScaledGlyph<'a> {
    g: Glyph<'a>,
    api_scale: Scale,
    scale: Vector<f32>,
}

impl fmt::Debug for ScaledGlyph<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ScaledGlyph")
            .field("id", &self.id().0)
            .field("scale", &self.api_scale)
            .finish()
    }
}

/// A glyph augmented with positioning and scaling information. You can query
/// such a glyph for information that depends on the scale and position of the
/// glyph.
#[derive(Clone)]
pub struct PositionedGlyph<'a> {
    sg: ScaledGlyph<'a>,
    position: Point<f32>,
    bb: Option<Rect<i32>>,
}

impl fmt::Debug for PositionedGlyph<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PositionedGlyph")
            .field("id", &self.id().0)
            .field("scale", &self.scale())
            .field("position", &self.position)
            .finish()
    }
}

/// Defines the size of a rendered face of a font, in pixels, horizontally and
/// vertically. A vertical scale of `y` pixels means that the distance betwen
/// the ascent and descent lines (see `VMetrics`) of the face will be `y`
/// pixels. If `x` and `y` are equal the scaling is uniform. Non-uniform scaling
/// by a factor *f* in the horizontal direction is achieved by setting `x` equal
/// to *f* times `y`.
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug)]
pub struct Scale {
    /// Horizontal scale, in pixels.
    pub x: f32,
    /// Vertical scale, in pixels.
    pub y: f32,
}

impl Scale {
    /// Uniform scaling, equivalent to `Scale { x: s, y: s }`.
    #[inline]
    pub fn uniform(s: f32) -> Scale {
        Scale { x: s, y: s }
    }
}
/// A trait for types that can be converted into a `GlyphId`, in the context of
/// a specific font.
///
/// Many `rusttype` functions that operate on characters accept values of any
/// type that implements `IntoGlyphId`. Such types include `char`, `Codepoint`,
/// and obviously `GlyphId` itself.
pub trait IntoGlyphId {
    /// Convert `self` into a `GlyphId`, consulting the index map of `font` if
    /// necessary.
    fn into_glyph_id(self, _: &Font<'_>) -> GlyphId;
}
impl IntoGlyphId for char {
    fn into_glyph_id(self, font: &Font<'_>) -> GlyphId {
        GlyphId(font.info.find_glyph_index(self as u32))
    }
}
impl IntoGlyphId for Codepoint {
    fn into_glyph_id(self, font: &Font<'_>) -> GlyphId {
        GlyphId(font.info.find_glyph_index(self.0))
    }
}
impl IntoGlyphId for GlyphId {
    #[inline]
    fn into_glyph_id(self, _font: &Font<'_>) -> GlyphId {
        self
    }
}
impl<'a> FontCollection<'a> {
    /// Constructs a font collection from an array of bytes, typically loaded
    /// from a font file, which may be a single font or a TrueType Collection
    /// holding a number of fonts. This array may be owned (e.g. `Vec<u8>`), or
    /// borrowed (`&[u8]`). As long as `From<T>` is implemented for `Bytes` for
    /// some type `T`, `T` can be used as input.
    ///
    /// This returns an error if `bytes` does not seem to be font data in a
    /// format we recognize.
    pub fn from_bytes<B: Into<SharedBytes<'a>>>(bytes: B) -> Result<FontCollection<'a>, Error> {
        let bytes = bytes.into();
        // We should use tt::is_collection once it lands in stb_truetype-rs:
        // https://github.com/redox-os/stb_truetype-rs/pull/15
        if !tt::is_font(&bytes) && &bytes[0..4] != b"ttcf" {
            return Err(Error::UnrecognizedFormat);
        }

        Ok(FontCollection(bytes))
    }
    /// If this `FontCollection` holds a single font, or a TrueType Collection
    /// containing only one font, return that as a `Font`. The `FontCollection`
    /// is consumed.
    ///
    /// If this `FontCollection` holds multiple fonts, return a
    /// `CollectionContainsMultipleFonts` error.
    ///
    /// If an error occurs, the `FontCollection` is lost, since this function
    /// takes ownership of it, and the error values don't give it back. If that
    /// is a problem, use the `font_at` or `into_fonts` methods instead, which
    /// borrow the `FontCollection` rather than taking ownership of it.
    pub fn into_font(self) -> Result<Font<'a>, Error> {
        let offset = if tt::is_font(&self.0) {
            0
        } else if tt::get_font_offset_for_index(&self.0, 1).is_some() {
            return Err(Error::CollectionContainsMultipleFonts);
        } else {
            // We now know that either a) `self.0` is a collection with only one
            // font, or b) `get_font_offset_for_index` found data it couldn't
            // recognize. Request the first font's offset, distinguishing
            // those two cases.
            match tt::get_font_offset_for_index(&self.0, 0) {
                None => return Err(Error::IllFormed),
                Some(offset) => offset,
            }
        };
        let info = tt::FontInfo::new(self.0, offset as usize).ok_or(Error::IllFormed)?;
        Ok(Font { info })
    }
    /// Gets the font at index `i` in the font collection, if it exists and is
    /// valid. The produced font borrows the font data that is either borrowed
    /// or owned by this font collection.
    pub fn font_at(&self, i: usize) -> Result<Font<'a>, Error> {
        let offset = tt::get_font_offset_for_index(&self.0, i as i32)
            .ok_or(Error::CollectionIndexOutOfBounds)?;
        let info = tt::FontInfo::new(self.0.clone(), offset as usize).ok_or(Error::IllFormed)?;
        Ok(Font { info })
    }
    /// Converts `self` into an `Iterator` yielding each `Font` that exists
    /// within the collection.
    pub fn into_fonts(self) -> IntoFontsIter<'a> {
        IntoFontsIter {
            collection: self,
            next_index: 0,
        }
    }
}
pub struct IntoFontsIter<'a> {
    next_index: usize,
    collection: FontCollection<'a>,
}
impl<'a> Iterator for IntoFontsIter<'a> {
    type Item = Result<Font<'a>, Error>;
    fn next(&mut self) -> Option<Self::Item> {
        let result = self.collection.font_at(self.next_index);
        if let Err(Error::CollectionIndexOutOfBounds) = result {
            return None;
        }
        self.next_index += 1;
        Some(result)
    }
}
impl<'a> Font<'a> {
    /// Constructs a font from an array of bytes, this is a shortcut for
    /// `FontCollection::from_bytes` for collections comprised of a single font.
    pub fn from_bytes<B: Into<SharedBytes<'a>>>(bytes: B) -> Result<Font<'a>, Error> {
        FontCollection::from_bytes(bytes).and_then(|c| c.into_font())
    }

    /// The "vertical metrics" for this font at a given scale. These metrics are
    /// shared by all of the glyphs in the font. See `VMetrics` for more detail.
    pub fn v_metrics(&self, scale: Scale) -> VMetrics {
        let vm = self.info.get_v_metrics();
        let scale = self.info.scale_for_pixel_height(scale.y);
        VMetrics::from(vm) * scale
    }

    /// Get the unscaled VMetrics for this font, shared by all glyphs.
    /// See `VMetrics` for more detail.
    pub fn v_metrics_unscaled(&self) -> VMetrics {
        VMetrics::from(self.info.get_v_metrics())
    }

    /// Returns the units per EM square of this font
    pub fn units_per_em(&self) -> u16 {
        self.info.units_per_em()
    }

    /// The number of glyphs present in this font. Glyph identifiers for this
    /// font will always be in the range `0..self.glyph_count()`
    pub fn glyph_count(&self) -> usize {
        self.info.get_num_glyphs() as usize
    }

    /// Returns the corresponding glyph for a Unicode code point or a glyph id
    /// for this font.
    ///
    /// If `id` is a `GlyphId`, it must be valid for this font; otherwise, this
    /// function panics. `GlyphId`s should always be produced by looking up some
    /// other sort of designator (like a Unicode code point) in a font, and
    /// should only be used to index the font they were produced for.
    ///
    /// Note that code points without corresponding glyphs in this font map to
    /// the ".notdef" glyph, glyph 0.
    pub fn glyph<C: IntoGlyphId>(&self, id: C) -> Glyph<'a> {
        let gid = id.into_glyph_id(self);
        assert!((gid.0 as usize) < self.glyph_count());
        // font clone either a reference clone, or arc clone
        Glyph::new(GlyphInner::Proxy(self.clone(), gid.0))
    }
    /// A convenience function.
    ///
    /// Returns an iterator that produces the glyphs corresponding to the code
    /// points or glyph ids produced by the given iterator `itr`.
    ///
    /// This is equivalent in behaviour to `itr.map(|c| font.glyph(c))`.
    pub fn glyphs_for<I: Iterator>(&self, itr: I) -> GlyphIter<'a, '_, I>
    where
        I::Item: IntoGlyphId,
    {
        GlyphIter { font: self, itr }
    }
    /// Returns an iterator over the names for this font.
    pub fn font_name_strings(&self) -> tt::FontNameIter<'_, SharedBytes<'a>> {
        self.info.get_font_name_strings()
    }
    /// A convenience function for laying out glyphs for a string horizontally.
    /// It does not take control characters like line breaks into account, as
    /// treatment of these is likely to depend on the application.
    ///
    /// Note that this function does not perform Unicode normalisation.
    /// Composite characters (such as ö constructed from two code points, ¨ and
    /// o), will not be normalised to single code points. So if a font does not
    /// contain a glyph for each separate code point, but does contain one for
    /// the normalised single code point (which is common), the desired glyph
    /// will not be produced, despite being present in the font. Deal with this
    /// by performing Unicode normalisation on the input string before passing
    /// it to `layout`. The crate
    /// [unicode-normalization](http://crates.io/crates/unicode-normalization)
    /// is perfect for this purpose.
    ///
    /// Calling this function is equivalent to a longer sequence of operations
    /// involving `glyphs_for`, e.g.
    ///
    /// ```no_run
    /// # use rusttype::*;
    /// # let (scale, start) = (Scale::uniform(0.0), point(0.0, 0.0));
    /// # let font: Font = unimplemented!();
    /// font.layout("Hello World!", scale, start)
    /// # ;
    /// ```
    ///
    /// produces an iterator with behaviour equivalent to the following:
    ///
    /// ```no_run
    /// # use rusttype::*;
    /// # let (scale, start) = (Scale::uniform(0.0), point(0.0, 0.0));
    /// # let font: Font = unimplemented!();
    /// font.glyphs_for("Hello World!".chars())
    ///     .scan((None, 0.0), |&mut (mut last, mut x), g| {
    ///         let g = g.scaled(scale);
    ///         if let Some(last) = last {
    ///             x += font.pair_kerning(scale, last, g.id());
    ///         }
    ///         let w = g.h_metrics().advance_width;
    ///         let next = g.positioned(start + vector(x, 0.0));
    ///         last = Some(next.id());
    ///         x += w;
    ///         Some(next)
    ///     })
    /// # ;
    /// ```
    pub fn layout<'b>(&'b self, s: &'b str, scale: Scale, start: Point<f32>) -> LayoutIter<'a, 'b> {
        LayoutIter {
            font: self,
            chars: s.chars(),
            caret: 0.0,
            scale,
            start,
            last_glyph: None,
        }
    }
    /// Returns additional kerning to apply as well as that given by HMetrics
    /// for a particular pair of glyphs.
    pub fn pair_kerning<A, B>(&self, scale: Scale, first: A, second: B) -> f32
    where
        A: IntoGlyphId,
        B: IntoGlyphId,
    {
        let first_id = first.into_glyph_id(self);
        let second_id = second.into_glyph_id(self);
        let factor = self.info.scale_for_pixel_height(scale.y) * (scale.x / scale.y);
        let kern = self.info.get_glyph_kern_advance(first_id.0, second_id.0);
        factor * kern as f32
    }
}
#[derive(Clone)]
pub struct GlyphIter<'a, 'b, I: Iterator>
where
    I::Item: IntoGlyphId,
{
    font: &'b Font<'a>,
    itr: I,
}
impl<'a, 'b, I: Iterator> Iterator for GlyphIter<'a, 'b, I>
where
    I::Item: IntoGlyphId,
{
    type Item = Glyph<'a>;
    fn next(&mut self) -> Option<Glyph<'a>> {
        self.itr.next().map(|c| self.font.glyph(c))
    }
}
#[derive(Clone)]
pub struct LayoutIter<'a, 'b> {
    font: &'b Font<'a>,
    chars: core::str::Chars<'b>,
    caret: f32,
    scale: Scale,
    start: Point<f32>,
    last_glyph: Option<GlyphId>,
}
impl<'a, 'b> Iterator for LayoutIter<'a, 'b> {
    type Item = PositionedGlyph<'a>;
    fn next(&mut self) -> Option<PositionedGlyph<'a>> {
        self.chars.next().map(|c| {
            let g = self.font.glyph(c).scaled(self.scale);
            if let Some(last) = self.last_glyph {
                self.caret += self.font.pair_kerning(self.scale, last, g.id());
            }
            let g = g.positioned(point(self.start.x + self.caret, self.start.y));
            self.caret += g.sg.h_metrics().advance_width;
            self.last_glyph = Some(g.id());
            g
        })
    }
}
impl<'a> Glyph<'a> {
    fn new(inner: GlyphInner<'a>) -> Glyph<'a> {
        Glyph { inner }
    }
    /// The font to which this glyph belongs. If the glyph is a standalone glyph
    /// that owns its resources, it no longer has a reference to the font which
    /// it was created from (using `standalone()`). In which case, `None` is
    /// returned.
    pub fn font(&self) -> Option<&Font<'a>> {
        match self.inner {
            GlyphInner::Proxy(ref f, _) => Some(f),
            GlyphInner::Shared(_) => None,
        }
    }
    /// The glyph identifier for this glyph.
    pub fn id(&self) -> GlyphId {
        match self.inner {
            GlyphInner::Proxy(_, id) => GlyphId(id),
            GlyphInner::Shared(ref data) => GlyphId(data.id),
        }
    }
    /// Augments this glyph with scaling information, making methods that depend
    /// on the scale of the glyph available.
    pub fn scaled(self, scale: Scale) -> ScaledGlyph<'a> {
        let (scale_x, scale_y) = match self.inner {
            GlyphInner::Proxy(ref font, _) => {
                let scale_y = font.info.scale_for_pixel_height(scale.y);
                let scale_x = scale_y * scale.x / scale.y;
                (scale_x, scale_y)
            }
            GlyphInner::Shared(ref data) => {
                let scale_y = data.scale_for_1_pixel * scale.y;
                let scale_x = scale_y * scale.x / scale.y;
                (scale_x, scale_y)
            }
        };
        ScaledGlyph {
            g: self,
            api_scale: scale,
            scale: vector(scale_x, scale_y),
        }
    }
    /// Turns a `Glyph<'a>` into a `Glyph<'static>`. This produces a glyph that
    /// owns its resources, extracted from the font. This glyph can outlive the
    /// font that it comes from.
    ///
    /// Calling `standalone()` on a standalone glyph shares the resources, and
    /// is equivalent to `clone()`.
    pub fn standalone(&self) -> Glyph<'static> {
        match self.inner {
            GlyphInner::Proxy(ref font, id) => {
                Glyph::new(GlyphInner::Shared(Arc::new(SharedGlyphData {
                    id,
                    scale_for_1_pixel: font.info.scale_for_pixel_height(1.0),
                    unit_h_metrics: {
                        let hm = font.info.get_glyph_h_metrics(id);
                        HMetrics {
                            advance_width: hm.advance_width as f32,
                            left_side_bearing: hm.left_side_bearing as f32,
                        }
                    },
                    extents: font.info.get_glyph_box(id).map(|bb| Rect {
                        min: point(bb.x0 as i32, -(bb.y1 as i32)),
                        max: point(bb.x1 as i32, -(bb.y0 as i32)),
                    }),
                    shape: font.info.get_glyph_shape(id),
                })))
            }
            GlyphInner::Shared(ref data) => Glyph::new(GlyphInner::Shared(data.clone())),
        }
    }
    /// Get the data from this glyph (such as width, extents, vertices, etc.).
    /// Only possible if the glyph is a shared glyph.
    pub fn get_data(&self) -> Option<Arc<SharedGlyphData>> {
        match self.inner {
            GlyphInner::Proxy(..) => None,
            GlyphInner::Shared(ref s) => Some(s.clone()),
        }
    }
}
/// Part of a `Contour`, either a `Line` or a `Curve`.
#[derive(Copy, Clone, Debug)]
pub enum Segment {
    Line(Line),
    Curve(Curve),
}
/// A closed loop consisting of a sequence of `Segment`s.
#[derive(Clone, Debug)]
pub struct Contour {
    pub segments: Vec<Segment>,
}
impl<'a> ScaledGlyph<'a> {
    /// The glyph identifier for this glyph.
    pub fn id(&self) -> GlyphId {
        self.g.id()
    }
    /// The font to which this glyph belongs. If the glyph is a standalone glyph
    /// that owns its resources, it no longer has a reference to the font which
    /// it was created from (using `standalone()`). In which case, `None` is
    /// returned.
    pub fn font(&self) -> Option<&Font<'a>> {
        self.g.font()
    }
    /// A reference to this glyph without the scaling
    pub fn into_unscaled(self) -> Glyph<'a> {
        self.g
    }
    /// Removes the scaling from this glyph
    pub fn unscaled(&self) -> &Glyph<'a> {
        &self.g
    }
    /// Augments this glyph with positioning information, making methods that
    /// depend on the position of the glyph available.
    pub fn positioned(self, p: Point<f32>) -> PositionedGlyph<'a> {
        let bb = self.pixel_bounds_at(p);
        PositionedGlyph {
            sg: self,
            position: p,
            bb,
        }
    }
    pub fn scale(&self) -> Scale {
        self.api_scale
    }
    /// Retrieves the "horizontal metrics" of this glyph. See `HMetrics` for
    /// more detail.
    pub fn h_metrics(&self) -> HMetrics {
        match self.g.inner {
            GlyphInner::Proxy(ref font, id) => {
                let hm = font.info.get_glyph_h_metrics(id);
                HMetrics {
                    advance_width: hm.advance_width as f32 * self.scale.x,
                    left_side_bearing: hm.left_side_bearing as f32 * self.scale.x,
                }
            }
            GlyphInner::Shared(ref data) => HMetrics {
                advance_width: data.unit_h_metrics.advance_width * self.scale.x,
                left_side_bearing: data.unit_h_metrics.left_side_bearing * self.scale.y,
            },
        }
    }
    fn shape_with_offset(&self, offset: Point<f32>) -> Option<Vec<Contour>> {
        use core::mem::replace;
        use stb_truetype::VertexType;
        match self.g.inner {
            GlyphInner::Proxy(ref font, id) => font.info.get_glyph_shape(id),
            GlyphInner::Shared(ref data) => data.shape.clone(),
        }
        .map(|shape| {
            let mut result = Vec::new();
            let mut current = Vec::new();
            let mut last = point(0.0, 0.0);
            for v in shape {
                let end = point(
                    v.x as f32 * self.scale.x + offset.x,
                    v.y as f32 * self.scale.y + offset.y,
                );
                match v.vertex_type() {
                    VertexType::MoveTo if !current.is_empty() => result.push(Contour {
                        segments: replace(&mut current, Vec::new()),
                    }),
                    VertexType::LineTo => current.push(Segment::Line(Line { p: [last, end] })),
                    VertexType::CurveTo => {
                        let control = point(
                            v.cx as f32 * self.scale.x + offset.x,
                            v.cy as f32 * self.scale.y + offset.y,
                        );
                        current.push(Segment::Curve(Curve {
                            p: [last, control, end],
                        }))
                    }
                    _ => (),
                }
                last = end;
            }
            if !current.is_empty() {
                result.push(Contour {
                    segments: replace(&mut current, Vec::new()),
                });
            }
            result
        })
    }
    /// Produces a list of the contours that make up the shape of this glyph.
    /// Each contour consists of a sequence of segments. Each segment is either
    /// a straight `Line` or a `Curve`.
    ///
    /// The winding of the produced contours is clockwise for closed shapes,
    /// anticlockwise for holes.
    pub fn shape(&self) -> Option<Vec<Contour>> {
        self.shape_with_offset(point(0.0, 0.0))
    }
    /// The bounding box of the shape of this glyph, not to be confused with
    /// `pixel_bounding_box`, the conservative pixel-boundary bounding box. The
    /// coordinates are relative to the glyph's origin.
    pub fn exact_bounding_box(&self) -> Option<Rect<f32>> {
        match self.g.inner {
            GlyphInner::Proxy(ref font, id) => font.info.get_glyph_box(id).map(|bb| Rect {
                min: point(bb.x0 as f32 * self.scale.x, -bb.y1 as f32 * self.scale.y),
                max: point(bb.x1 as f32 * self.scale.x, -bb.y0 as f32 * self.scale.y),
            }),
            GlyphInner::Shared(ref data) => data.extents.map(|bb| Rect {
                min: point(
                    bb.min.x as f32 * self.scale.x,
                    bb.min.y as f32 * self.scale.y,
                ),
                max: point(
                    bb.max.x as f32 * self.scale.x,
                    bb.max.y as f32 * self.scale.y,
                ),
            }),
        }
    }
    /// Constructs a glyph that owns its data from this glyph. This is similar
    /// to `Glyph::standalone`. See that function for more details.
    pub fn standalone(&self) -> ScaledGlyph<'static> {
        ScaledGlyph {
            g: self.g.standalone(),
            api_scale: self.api_scale,
            scale: self.scale,
        }
    }

    #[inline]
    fn pixel_bounds_at(&self, p: Point<f32>) -> Option<Rect<i32>> {
        // Use subpixel fraction in floor/ceil rounding to elimate rounding error
        // from identical subpixel positions
        let (x_trunc, x_fract) = (p.x.trunc() as i32, p.x.fract());
        let (y_trunc, y_fract) = (p.y.trunc() as i32, p.y.fract());

        match self.g.inner {
            GlyphInner::Proxy(ref font, id) => font
                .info
                .get_glyph_bitmap_box_subpixel(id, self.scale.x, self.scale.y, x_fract, y_fract)
                .map(|bb| Rect {
                    min: point(x_trunc + bb.x0, y_trunc + bb.y0),
                    max: point(x_trunc + bb.x1, y_trunc + bb.y1),
                }),
            GlyphInner::Shared(ref data) => data.extents.map(|bb| Rect {
                min: point(
                    (bb.min.x as f32 * self.scale.x + x_fract).floor() as i32 + x_trunc,
                    (bb.min.y as f32 * self.scale.y + y_fract).floor() as i32 + y_trunc,
                ),
                max: point(
                    (bb.max.x as f32 * self.scale.x + x_fract).ceil() as i32 + x_trunc,
                    (bb.max.y as f32 * self.scale.y + y_fract).ceil() as i32 + y_trunc,
                ),
            }),
        }
    }
}

impl<'a> PositionedGlyph<'a> {
    /// The glyph identifier for this glyph.
    pub fn id(&self) -> GlyphId {
        self.sg.id()
    }
    /// The font to which this glyph belongs. If the glyph is a standalone glyph
    /// that owns its resources, it no longer has a reference to the font which
    /// it was created from (using `standalone()`). In which case, `None` is
    /// returned.
    pub fn font(&self) -> Option<&Font<'a>> {
        self.sg.font()
    }
    /// A reference to this glyph without positioning
    pub fn unpositioned(&self) -> &ScaledGlyph<'a> {
        &self.sg
    }
    /// Removes the positioning from this glyph
    pub fn into_unpositioned(self) -> ScaledGlyph<'a> {
        self.sg
    }
    /// The conservative pixel-boundary bounding box for this glyph. This is the
    /// smallest rectangle aligned to pixel boundaries that encloses the shape
    /// of this glyph at this position. Note that the origin of the glyph, at
    /// pixel-space coordinates (0, 0), is at the top left of the bounding box.
    pub fn pixel_bounding_box(&self) -> Option<Rect<i32>> {
        self.bb
    }
    /// Similar to `ScaledGlyph::shape()`, but with the position of the glyph
    /// taken into account.
    pub fn shape(&self) -> Option<Vec<Contour>> {
        self.sg.shape_with_offset(self.position)
    }
    pub fn scale(&self) -> Scale {
        self.sg.api_scale
    }
    pub fn position(&self) -> Point<f32> {
        self.position
    }
    /// Rasterises this glyph. For each pixel in the rect given by
    /// `pixel_bounding_box()`, `o` is called:
    ///
    /// ```ignore
    /// o(x, y, v)
    /// ```
    ///
    /// where `x` and `y` are the coordinates of the pixel relative to the `min`
    /// coordinates of the bounding box, and `v` is the analytically calculated
    /// coverage of the pixel by the shape of the glyph. Calls to `o` proceed in
    /// horizontal scanline order, similar to this pseudo-code:
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
        use stb_truetype::VertexType;
        let shape = match self.sg.g.inner {
            GlyphInner::Proxy(ref font, id) => {
                font.info.get_glyph_shape(id).unwrap_or_else(Vec::new)
            }
            GlyphInner::Shared(ref data) => data.shape.clone().unwrap_or_else(Vec::new),
        };
        let bb = if let Some(bb) = self.bb.as_ref() {
            bb
        } else {
            return;
        };
        let offset = vector(bb.min.x as f32, bb.min.y as f32);
        let mut lines = Vec::new();
        let mut curves = Vec::new();
        let mut last = point(0.0, 0.0);
        for v in shape {
            let end = point(
                v.x as f32 * self.sg.scale.x + self.position.x,
                -v.y as f32 * self.sg.scale.y + self.position.y,
            ) - offset;
            match v.vertex_type() {
                VertexType::LineTo => lines.push(Line { p: [last, end] }),
                VertexType::CurveTo => {
                    let control = point(
                        v.cx as f32 * self.sg.scale.x + self.position.x,
                        -v.cy as f32 * self.sg.scale.y + self.position.y,
                    ) - offset;
                    curves.push(Curve {
                        p: [last, control, end],
                    })
                }
                VertexType::MoveTo => {}
            }
            last = end;
        }
        rasterizer::rasterize(
            &lines,
            &curves,
            (bb.max.x - bb.min.x) as u32,
            (bb.max.y - bb.min.y) as u32,
            o,
        );
    }
    /// Constructs a glyph that owns its data from this glyph. This is similar
    /// to `Glyph::standalone`. See that function for more details.
    pub fn standalone(&self) -> PositionedGlyph<'static> {
        PositionedGlyph {
            sg: self.sg.standalone(),
            bb: self.bb,
            position: self.position,
        }
    }

    /// Resets positioning information and recalculates the pixel bounding box
    pub fn set_position(&mut self, p: Point<f32>) {
        let p_diff = p - self.position;
        if relative_eq!(p_diff.x.fract(), 0.0) && relative_eq!(p_diff.y.fract(), 0.0) {
            if let Some(bb) = self.bb.as_mut() {
                let rounded_diff = vector(p_diff.x.round() as i32, p_diff.y.round() as i32);
                bb.min = bb.min + rounded_diff;
                bb.max = bb.max + rounded_diff;
            }
        } else {
            self.bb = self.sg.pixel_bounds_at(p);
        }
        self.position = p;
    }
}

/// The type for errors returned by rusttype.
#[derive(Debug)]
pub enum Error {
    /// Font data presented to rusttype is not in a format that the library
    /// recognizes.
    UnrecognizedFormat,

    /// Font data presented to rusttype was ill-formed (lacking necessary
    /// tables, for example).
    IllFormed,

    /// The caller tried to access the `i`'th font from a `FontCollection`, but
    /// the collection doesn't contain that many fonts.
    CollectionIndexOutOfBounds,

    /// The caller tried to convert a `FontCollection` into a font via
    /// `into_font`, but the `FontCollection` contains more than one font.
    CollectionContainsMultipleFonts,
}

impl Error {
    fn description(&self) -> &str {
        use self::Error::*;
        match *self {
            UnrecognizedFormat => "Font data in unrecognized format",
            IllFormed => "Font data is ill-formed",
            CollectionIndexOutOfBounds => "Font collection has no font at the given index",
            CollectionContainsMultipleFonts => {
                "Attempted to convert collection into a font, \
                 but collection contais more than one font"
            }
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> core::result::Result<(), fmt::Error> {
        f.write_str(self.description())
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {
    fn description(&self) -> &str {
        self.description()
    }
}

#[cfg(feature = "std")]
impl std::convert::From<Error> for std::io::Error {
    fn from(error: Error) -> Self {
        std::io::Error::new(std::io::ErrorKind::Other, error)
    }
}
