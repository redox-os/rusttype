# RustType
[![crates.io](https://img.shields.io/crates/v/rusttype.svg)](https://crates.io/crates/rusttype)
[![docs.rs](https://docs.rs/rusttype/badge.svg)](https://docs.rs/rusttype)

RustType is a pure Rust alternative to libraries like FreeType.

The current capabilities of RustType:

* Reading TrueType formatted fonts and font collections. This includes `*.ttf`
  as well as a subset of `*.otf` font files.
* Retrieving glyph shapes and commonly used properties for a font and its glyphs.
* Laying out glyphs horizontally using horizontal and vertical metrics, and
  glyph-pair-specific kerning.
* Rasterising glyphs with sub-pixel positioning using an accurate analytical
  algorithm (not based on sampling).
* Managing a font cache on the GPU with the `gpu_cache` module. This keeps
  recently used glyph renderings
  in a dynamic cache in GPU memory to minimise texture uploads per-frame. It
  also allows you keep the draw call count for text very low, as all glyphs are
  kept in one GPU texture.

Notable things that RustType does not support *yet*:

* OpenType formatted fonts that are not just TrueType fonts (OpenType is a
  superset of TrueType). Notably there is no support yet for cubic Bezier curves
  used in glyphs.
* Font hinting.
* Ligatures of any kind
* Some less common TrueType sub-formats.
* Right-to-left and vertical text layout.

## Getting Started

To hit the ground running with RustType, look at the `simple.rs` example
supplied with the crate. It demonstrates loading a font file, rasterising an
arbitrary string, and displaying the result as ASCII art. If you prefer to just
look at the documentation, the entry point for loading fonts is
`FontCollection`, from which you can access individual fonts, then their glyphs.

## Future Plans

The initial motivation for the project was to provide easy-to-use font rendering for games.
There are numerous avenues for improving RustType. Ideas:

* Some form of hinting for improved legibility at small font sizes.
* Replacing the dependency on
  [stb_truetype-rs](https://gitlab.redox-os.org/redox-os/stb_truetype-rs)
  (a translation of [stb_truetype.h](https://github.com/nothings/stb/blob/master/stb_truetype.h)),
  with OpenType font loading written in idiomatic Rust.
* Add support for cubic curves in OpenType fonts.
* Extract the rasterisation code into a separate vector graphics rendering crate.
* Support for some common forms of ligatures.
* And, eventually, support for embedded right-to-left Unicode text.

If you think you could help with achieving any of these goals, feel free to open
a tracking issue for discussing them.

## Minimum supported rust compiler
This crate is maintained with [latest stable rust](https://gist.github.com/alexheretic/d1e98d8433b602e57f5d0a9637927e0c).

## License

Licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
   http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or
   http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

### See Also

- [glyph_brush](https://github.com/alexheretic/glyph-brush) - can cache vertex generation & provides more complex layouts.
