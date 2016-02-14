# RustType

[![Build Status](https://travis-ci.org/dylanede/rusttype.svg?branch=master)](https://travis-ci.org/dylanede/rusttype)

RustType is a pure Rust alternative to libraries like FreeType.

The current capabilities of RustType:

* Reading TrueType formatted fonts and font collections. This includes `*.ttf`
  as well as a subset of `*.otf` font files.
* Retrieving glyph shapes and commonly used properties for a font and its glyphs.
* Laying out glyphs horizontally using horizontal and vertical metrics, and
  glyph-pair-specific kerning.
* Rasterising glyphs with sub-pixel positioning using an accurate analytical
  algorithm (not based on sampling).

Notable things that RustType does not support *yet*:

* OpenType formatted fonts that are not just TrueType fonts (OpenType is a
  superset of TrueType). Notably there is no support yet for cubic Bezier curves
  used in glyphs.
* Ligatures of any kind
* Some less common TrueType sub-formats.
* Right-to-left and vertical text layout.

## Getting Started

Add the following to your Cargo.toml:

```toml
[dependencies]
rusttype = "0.1.2"
```

To hit the ground running with RustType, look at the `simple.rs` example
supplied with the crate. It demonstrates loading a font file, rasterising an
arbitrary string, and displaying the result as ASCII art. If you prefer to just
look at the documentation, the entry point for loading fonts is
`FontCollection`, from which you can access individual fonts, then their glyphs.

## [Documentation](https://dylanede.github.io/rusttype)

## Future Plans

The current state of RustType is only the beginning. There are numerous avenues
for improving it. My main motivation for this project is to provide easy-to-use
font rendering for games. Thus I will be constructing a GPU font caching library
based on RustType. Once I get the time to go back and improve RustType itself,
the main improvements I am most interested in are:

* Replacing the dependency on my other library,
  [stb_truetype-rs](https://github.com/dylanede/stb_truetype-rs)
  (a direct translation of [stb_truetype.h](https://github.com/nothings/stb/blob/master/stb_truetype.h)),
  with OpenType font loading written in idiomatic Rust.
* Add support for cubic curves in OpenType fonts.
* Extract the rasterisation code into a separate vector graphics rendering crate.
* Support for some common forms of ligatures.
* And, eventually, support for embedded right-to-left Unicode text.

If you think you could help with achieving any of these goals, feel free to open
a tracking issue for discussing them.

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
