use ::{PositionedGlyph, GlyphId, Rect, Scale, Vector};

use std::collections::HashMap;
use support::bst::map::BTreeMap;
use support::bst::Bound::{Included, Unbounded};
use linked_hash_map::LinkedHashMap;

use ndarray::{Ix, OwnedArray, zeros};

#[derive(PartialEq, PartialOrd, Copy, Clone, Debug)]
struct PGlyphSpec {
    font_id: usize,
    glyph_id: GlyphId,
    scale: Scale,
    offset: Vector<f32>
}

impl ::std::cmp::Eq for PGlyphSpec {}

impl ::std::cmp::Ord for PGlyphSpec {
    fn cmp(&self, other: &PGlyphSpec) -> ::std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

struct Row {
    height: u32,
    width: u32,
    glyphs: Vec<(PGlyphSpec, Rect<u32>, OwnedArray<u8, (Ix, Ix)>)>
}

pub struct Cache {
    scale_tolerance: f32,
    position_tolerance: f32,
    width: u32,
    height: u32,
    rows: LinkedHashMap<u32, Row>,
    space_start_for_end: HashMap<u32, u32>,
    space_end_for_start: HashMap<u32, u32>,
    queue: Vec<(usize, PositionedGlyph<'static>)>,
    queue_retry: bool,
    all_glyphs: BTreeMap<PGlyphSpec, (u32, u32)>
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CacheReadErr {
    GlyphNotCached
}
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CacheWriteErr {
    GlyphTooLarge,
    NoRoomForWholeQueue
}
fn normalise_pixel_offset(mut offset: Vector<f32>) -> Vector<f32> {
    if offset.x > 0.5 {
        offset.x -= 1.0;
    } else if offset.x < -0.5 {
        offset.x += 1.0;
    }
    if offset.y > 0.5 {
        offset.y -= 1.0;
    } else if offset.y < -0.5 {
        offset.y += 1.0;
    }
    offset
}
impl Cache {
    pub fn new(width: u32, height: u32,
               scale_tolerance: f32, position_tolerance: f32) -> Cache {
        assert!(scale_tolerance >= 0.0);
        assert!(position_tolerance >= 0.0);
        let scale_tolerance = scale_tolerance.max(0.001);
        let position_tolerance = position_tolerance.max(0.001);
        Cache {
            scale_tolerance: scale_tolerance,
            position_tolerance: position_tolerance,
            width: width,
            height: height,
            rows: LinkedHashMap::new(),
            space_start_for_end: {let mut m = HashMap::new(); m.insert(height, 0); m},
            space_end_for_start: {let mut m = HashMap::new(); m.insert(0, height); m},
            queue: Vec::new(),
            queue_retry: false,
            all_glyphs: BTreeMap::new()

        }
    }
    /// Queue a glyph for caching by the next call to `cache_queued`.
    pub fn queue_glyph(&mut self, font_id: usize, glyph: PositionedGlyph) {
        if glyph.pixel_bounding_box().is_some() {
            self.queue.push((font_id, glyph.standalone()));
        }
    }
    /// Clears the cache. Does not affect the glyph queue.
    pub fn clear(&mut self) {
        self.rows.clear();
        self.space_end_for_start.clear();
        self.space_end_for_start.insert(0, self.height);
        self.space_start_for_end.clear();
        self.space_start_for_end.insert(self.height, 0);
        self.all_glyphs.clear();
    }
    /// Clears the glyph queue.
    pub fn clear_queue(&mut self) {
        self.queue.clear();
    }
    /// Caches the queued glyphs. If this is unsuccessful, the queue is untouched.
    /// Any glyphs cached by previous calls to this function may be removed from the cache to make
    /// room for the newly queued glyphs. Thus if you want to ensure that a glyph is in the cache,
    /// the most recently cached queue must have contained that glyph.
    pub fn cache_queued<F: FnMut(Rect<u32>, &[u8])>(&mut self, mut uploader: F) -> Result<(), CacheWriteErr> {
        use vector;
        use point;
        let mut oldest_in_use_row = None;
        // tallest first gives better packing
        self.queue.sort_by(|x, y|
                           y.1.pixel_bounding_box().unwrap().height().cmp(
                               &x.1.pixel_bounding_box().unwrap().height()));
        let mut queue_success = true;
        'per_glyph: for &(font_id, ref glyph) in &self.queue {
            // Check to see if it's already cached, or a close enough version is:
            // (Note that the search for "close enough" here is conservative - there may be
            // a close enough glyph that isn't found; identical glyphs however will always be found)
            let p = glyph.position();
            let pfract = normalise_pixel_offset(vector(p.x.fract(), p.y.fract()));
            let spec = PGlyphSpec {
                font_id: font_id,
                glyph_id: glyph.id(),
                scale: glyph.scale(),
                offset: pfract
            };
            let lower = self.all_glyphs.range(Unbounded, Included(&spec)).rev().next()
                .and_then(|(l, &(lrow, _))| {
                    if l.font_id == spec.font_id &&
                        l.glyph_id == spec.glyph_id &&
                        (l.scale.x - spec.scale.x).abs() < self.scale_tolerance &&
                        (l.scale.y - spec.scale.y).abs() < self.scale_tolerance &&
                        (spec.offset.x - l.offset.x).abs() < self.position_tolerance &&
                        (spec.offset.y - l.offset.y).abs() < self.position_tolerance
                    {
                        Some((l.scale, l.offset, lrow))
                    } else {
                        None
                    }
                });
            let upper = self.all_glyphs.range(Included(&spec), Unbounded).next()
                .and_then(|(u, &(urow, _))| {
                    if u.font_id == spec.font_id &&
                        u.glyph_id == spec.glyph_id &&
                        (u.scale.x - spec.scale.x).abs() < self.scale_tolerance &&
                        (u.scale.y - spec.scale.y).abs() < self.scale_tolerance &&
                        (spec.offset.x - u.offset.x).abs() < self.position_tolerance &&
                        (spec.offset.y - u.offset.y).abs() < self.position_tolerance
                    {
                        Some((u.scale, u.offset, urow))
                    } else {
                        None
                    }
                });
            match (lower, upper) {
                (None, None) => {} // No match
                (None, Some((_, _, row))) |
                (Some((_, _, row)), None) => {
                    // just one match
                    self.rows.get_refresh(&row);
                    if oldest_in_use_row.is_none() {
                        oldest_in_use_row = Some(row);
                    }
                    continue 'per_glyph;
                }
                (Some((_, _, row1)), Some((_, _, row2))) if row1 == row2 => {
                    // two matches, but the same row
                    self.rows.get_refresh(&row1);
                    if oldest_in_use_row.is_none() {
                        oldest_in_use_row = Some(row1);
                    }
                    continue 'per_glyph;
                }
                (Some((scale1, offset1, row1)), Some((scale2, offset2, row2))) => {
                    // two definitely distinct matches
                    let v1 =
                        ((scale1.x - spec.scale.x) / self.scale_tolerance).abs()
                        + ((scale1.y - spec.scale.y) / self.scale_tolerance).abs()
                        + ((offset1.x - spec.offset.x) / self.position_tolerance).abs()
                        + ((offset1.y - spec.offset.y) / self.position_tolerance).abs();
                    let v2 =
                        ((scale2.x - spec.scale.x) / self.scale_tolerance).abs()
                        + ((scale2.y - spec.scale.y) / self.scale_tolerance).abs()
                        + ((offset2.x - spec.offset.x) / self.position_tolerance).abs()
                        + ((offset2.y - spec.offset.y) / self.position_tolerance).abs();
                    let row = if v1 < v2 { row1 } else { row2 };
                    self.rows.get_refresh(&row);
                    if oldest_in_use_row.is_none() {
                        oldest_in_use_row = Some(row);
                    }
                    continue 'per_glyph;
                }
            }
            // Not cached, so add it:
            let bb = glyph.pixel_bounding_box().unwrap();
            let (width, height) = (bb.width() as u32, bb.height() as u32);
            if width >= self.width || height >= self.height {
                return Result::Err(CacheWriteErr::GlyphTooLarge);
            }
            // find row to put the glyph in, most used rows first
            let mut row_top = None;
            for (top, row) in self.rows.iter().rev() {
                if row.height >= height && self.width - row.width >= width {
                    // found a spot on an existing row
                    row_top = Some(*top);
                    break;
                }
            }

            if row_top.is_none() {
                let mut gap = None;
                // See if there is space for a new row
                for (start, end) in self.space_end_for_start.iter() {
                    if end - start >= height {
                        gap = Some((*start, *end));
                        break;
                    }
                }
                if gap.is_none() {
                    // Remove old rows until room is available
                    while self.rows.len() > 0 {
                        // check that the oldest row isn't also in use
                        if oldest_in_use_row.as_ref().map(|t| self.rows.front().unwrap().0 != t).unwrap_or(true) {
                            // Remove row
                            let (top, row) = self.rows.pop_front().unwrap();
                            for (spec, _, _) in row.glyphs {
                                self.all_glyphs.remove(&spec);
                            }
                            let (mut new_start, mut new_end) = (top, top + row.height);
                            // Update the free space maps
                            if let Some(end) = self.space_end_for_start.remove(&new_end) {
                                new_end = end;
                            }
                            if let Some(start) = self.space_start_for_end.remove(&new_start) {
                                new_start = start;
                            }
                            self.space_start_for_end.insert(new_end, new_start);
                            self.space_end_for_start.insert(new_start, new_end);
                            if new_end - new_start >= height {
                                // The newly formed gap is big enough
                                gap = Some((new_start, new_end));
                                break
                            }
                        } else {
                            // all rows left are in use
                            // try a clean insert of all needed glyphs
                            // if that doesn't work, fail
                            if self.queue_retry { // already trying a clean insert, don't do it again
                                return Err(CacheWriteErr::NoRoomForWholeQueue);
                            } else { // signal that a retry is needed
                                queue_success = false;
                                break 'per_glyph;
                            }
                        }
                    }
                }
                let (gap_start, gap_end) = gap.unwrap();
                // fill space for new row
                let new_space_start = gap_start + height;
                self.space_end_for_start.remove(&gap_start);
                if new_space_start == gap_end {
                    self.space_start_for_end.remove(&gap_end);
                } else {
                    self.space_end_for_start.insert(new_space_start, gap_end);
                    self.space_start_for_end.insert(gap_end, new_space_start);
                }
                // add the row
                self.rows.insert(gap_start, Row {
                    width: 0,
                    height: height,
                    glyphs: Vec::new()
                });
                row_top = Some(gap_start);
            }
            let row_top = row_top.unwrap();
            // calculate the target rect
            let row = self.rows.get_refresh(&row_top).unwrap();
            let rect = Rect {
                min: point(row.width, row_top),
                max: point(row.width + width, row_top + height)
            };
            // draw the glyph into main memory
            let mut pixels = zeros((height as usize, width as usize));
            glyph.draw(|x, y, v| {
                let v = ((v * 255.0) + 0.5).floor().max(0.0).min(255.0) as u8;
                pixels[(y as usize, x as usize)] = v;
            });
            // transfer
            uploader(
                rect,
                &pixels.as_slice().unwrap());
            // add the glyph to the row
            row.glyphs.push((spec, rect, pixels));
            row.width += width;
            if oldest_in_use_row.is_none() {
                oldest_in_use_row = Some(row_top);
            }
            self.all_glyphs.insert(spec, (row_top, row.glyphs.len() as u32 - 1));
        }
        if queue_success {
            self.queue.clear();
            Ok(())
        } else { // clear the cache then try again
            self.clear();
            self.queue_retry = true;
            let result = self.cache_queued(uploader);
            self.queue_retry = false;
            result
        }
    }
    /// Retrieves the texture coordinates of the quad for a glyph in the cache, as well as the pixel-space
    /// coordinates that this region should be drawn at. In the majority of cases these pixel-space coordinates
    /// should be identical to the bounding box of the input glyph. They only differ if the cache has
    /// returned a substitute glyph that is deemed close enough to the requested glyph as specified by the
    /// cache tolerance parameters.
    /// A sucessful result is `Some` if the glyph is not an empty glyph (no shape, and thus no rect to return).
    pub fn rect_for<'a>(&'a self,
                        font_id: usize,
                        glyph: &PositionedGlyph) -> Result<Option<(Rect<f32>, Rect<i32>)>, CacheReadErr> {
        use vector;
        use point;
        let glyph_bb = match glyph.pixel_bounding_box() {
            Some(bb) => bb,
            None => return Ok(None)
        };
        let target_position = glyph.position();
        let target_offset = normalise_pixel_offset(vector(target_position.x.fract(), target_position.y.fract()));
        let target_spec = PGlyphSpec {
            font_id: font_id,
            glyph_id: glyph.id(),
            scale: glyph.scale(),
            offset: target_offset
        };
        let lower = self.all_glyphs.range(Unbounded, Included(&target_spec)).rev().next()
            .and_then(|(l, &(lrow, lindex))| {
                if l.font_id == target_spec.font_id &&
                    l.glyph_id == target_spec.glyph_id &&
                    (l.scale.x - target_spec.scale.x).abs() < self.scale_tolerance &&
                    (l.scale.y - target_spec.scale.y).abs() < self.scale_tolerance &&
                    (target_spec.offset.x - l.offset.x).abs() < self.position_tolerance &&
                    (target_spec.offset.y - l.offset.y).abs() < self.position_tolerance
                {
                    Some((l, lrow, lindex))
                } else {
                    None
                }
            });
        let upper = self.all_glyphs.range(Included(&target_spec), Unbounded).next()
            .and_then(|(u, &(urow, uindex))| {
                if u.font_id == target_spec.font_id &&
                    u.glyph_id == target_spec.glyph_id &&
                    (u.scale.x - target_spec.scale.x).abs() < self.scale_tolerance &&
                    (u.scale.y - target_spec.scale.y).abs() < self.scale_tolerance &&
                    (target_spec.offset.x - u.offset.x).abs() < self.position_tolerance &&
                    (target_spec.offset.y - u.offset.y).abs() < self.position_tolerance
                {
                    Some((u, urow, uindex))
                } else {
                    None
                }
            });
        let (width, height) = (self.width as f32, self.height as f32);
        let (match_spec, row, index) = match (lower, upper) {
            (None, None) => return Err(CacheReadErr::GlyphNotCached),
            (Some(match_), None) |
            (None, Some(match_)) => match_, // one match
            (Some((lmatch_spec, lrow, lindex)), Some((umatch_spec, urow, uindex))) => {
                if lrow == urow && lindex == uindex {
                    // both matches are really the same one, and match the input
                    let tex_rect = self.rows[&lrow].glyphs[lindex as usize].1;
                    let uv_rect = Rect {
                        min: point(tex_rect.min.x as f32 / width, tex_rect.min.y as f32 / height),
                        max: point(tex_rect.max.x as f32 / width, tex_rect.max.y as f32 / height)
                    };
                    return Ok(Some((uv_rect, glyph_bb)))
                } else {
                    // Two close-enough matches. Figure out which is closest.
                    let l_measure =
                        ((lmatch_spec.scale.x - target_spec.scale.x) / self.scale_tolerance).abs()
                        + ((lmatch_spec.scale.y - target_spec.scale.y) / self.scale_tolerance).abs()
                        + ((lmatch_spec.offset.x - target_spec.offset.x) / self.position_tolerance).abs()
                        + ((lmatch_spec.offset.y - target_spec.offset.y) / self.position_tolerance).abs();
                    let u_measure =
                        ((umatch_spec.scale.x - target_spec.scale.x) / self.scale_tolerance).abs()
                        + ((umatch_spec.scale.y - target_spec.scale.y) / self.scale_tolerance).abs()
                        + ((umatch_spec.offset.x - target_spec.offset.x) / self.position_tolerance).abs()
                        + ((umatch_spec.offset.y - target_spec.offset.y) / self.position_tolerance).abs();
                    if l_measure < u_measure {
                        (lmatch_spec, lrow, lindex)
                    } else {
                        (umatch_spec, urow, uindex)
                    }
                }
            }
        };
        let tex_rect = self.rows[&row].glyphs[index as usize].1;
        let uv_rect = Rect {
            min: point(tex_rect.min.x as f32 / width, tex_rect.min.y as f32 / height),
            max: point(tex_rect.max.x as f32 / width, tex_rect.max.y as f32 / height)
        };
        let local_bb = glyph
            .unpositioned().clone()
            .positioned(point(0.0, 0.0) + match_spec.offset).pixel_bounding_box().unwrap();
        let min_from_origin =
            point(local_bb.min.x as f32, local_bb.min.y as f32)
                    - (point(0.0, 0.0) + match_spec.offset);
        let ideal_min = min_from_origin + target_position;
        let min = point(ideal_min.x.round() as i32, ideal_min.y.round() as i32);
        let bb_offset = min - local_bb.min;
        let bb = Rect {
            min: min,
            max: local_bb.max + bb_offset
        };
        Ok(Some((uv_rect, bb)))
    }
}

#[cfg(test)]
#[test]
fn cache_test() {
    use ::FontCollection;
    use ::Scale;
    use ::point;
    let mut cache = Cache::new(32, 32, 0.1, 0.1);
    let font_data = include_bytes!("../examples/Arial Unicode.ttf");
    let font = FontCollection::from_bytes(font_data as &[u8]).into_font().unwrap();
    let strings = [
        ("Hello World!", 15.0),
        ("Hello World!", 14.0),
        ("Hello World!", 10.0),
        ("Hello World!", 15.0),
        ("Hello World!", 14.0),
        ("Hello World!", 10.0)
            ];
    for i in 0..strings.len() {
        println!("Caching {:?}", strings[i]);
        for glyph in font.layout(strings[i].0, Scale { x: strings[i].1, y: strings[i].1 }, point(0.0, 0.0)) {
            cache.queue_glyph(0, glyph);
        }
        cache.cache_queued(|_, _| {}).unwrap();
    }
}
