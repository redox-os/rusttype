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

pub use rusttype_next::*;
