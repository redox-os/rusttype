use ::geometry::*;
use arrayvec;
trait SliceUp: Sized {
    type PerSlice: Iterator<Item=Self>;
    type Out: Iterator<Item=Self::PerSlice>;
    fn slice_up_x(&self, planes: PlaneSet) -> Self::Out;
    fn slice_up_y(&self, planes: PlaneSet) -> Self::Out;
}

type LineIter = ::std::option::IntoIter<Line>;

#[derive(Debug)]
struct LineSliceIter {
    l: Line,
    m: f32,
    c: f32,
    planes: PlaneSet,
    i: usize
}

impl Iterator for LineSliceIter {
    type Item = LineIter;
    fn next(&mut self) -> Option<LineIter> {
        if self.i >= self.planes.count {
            return None
        }
        if self.m == 0.0 {
            self.i += 1;
            return Some(Some(self.l).into_iter())
        }
        let lower = self.i as f32;
        let upper = lower + 1.0;
        let lower_d = self.planes.start + self.planes.step * lower;
        let upper_d = self.planes.start + self.planes.step * upper;
        let mut lower_t = (lower_d - self.c) / self.m;
        let mut upper_t = (upper_d - self.c) / self.m;
        lower_t = lower_t.max(0.0).min(1.0);
        upper_t = upper_t.max(0.0).min(1.0);
        if self.m < 0.0 {
            ::std::mem::swap(&mut lower_t, &mut upper_t);
        }
        self.i += 1;
        if lower_t != upper_t {
            let p = &self.l.p;
            let v = p[1] - p[0];
            Some(Some(Line {
                p: [p[0] + v * lower_t,
                    p[0] + v * upper_t]
            }).into_iter())
        } else {
            Some(None.into_iter())
        }
    }
}

impl SliceUp for Line {
    type PerSlice = LineIter;
    type Out = LineSliceIter;
    fn slice_up_x(&self, planes: PlaneSet) -> LineSliceIter {
        let p = &self.p;
        LineSliceIter {
            l: *self,
            planes: planes,
            i: 0,
            m: p[1].x - p[0].x,
            c: p[0].x
        }
    }
    fn slice_up_y(&self, planes: PlaneSet) -> LineSliceIter {
        let p = &self.p;
        LineSliceIter {
            l: *self,
            planes: planes,
            i: 0,
            m: p[1].y - p[0].y,
            c: p[0].y
        }
    }
}

type CurveIter = arrayvec::IntoIter<[Curve; 2]>;

struct CurveSliceIter {
    curve: Curve,
    planes: PlaneSet,
    i: usize,
    a: f32,
    b: f32,
    c_shift: f32
}

impl Iterator for CurveSliceIter {
    type Item = CurveIter;
    fn next(&mut self) -> Option<Self::Item> {
        use arrayvec::ArrayVec;
        use geometry::RealQuadraticSolution as RQS;
        use geometry::solve_quadratic_real as solve;
        use geometry::Cut;
        if self.i >= self.planes.count {
            return None
        }
        let lower = self.i as f32;
        self.i += 1;
        let upper = lower + self.planes.step;
        let lower_d = self.planes.start + self.planes.step * lower;
        let upper_d = self.planes.start + self.planes.step * upper;
        let l_sol = solve(self.a, self.b, self.c_shift - lower_d);
        let u_sol = solve(self.a, self.b, self.c_shift - upper_d);
        let mut result = ArrayVec::<[Curve; 2]>::new();
        match (l_sol.in_order(), u_sol.in_order()) {
            (RQS::Two(a, b), RQS::Two(c, d)) => {
                // Two pieces
                let (a, b, c, d) = if self.a > 0.0 {
                    (c, a, b, d)
                } else {
                    (a, c, d, b)
                };
                let (a, b, c, d) = (a.min(1.0).max(0.0),
                                    b.min(1.0).max(0.0),
                                    c.min(1.0).max(0.0),
                                    d.min(1.0).max(0.0));
                if a != b {
                    result.push(self.curve.cut_from_to(a, b));
                }
                if c != d {
                    result.push(self.curve.cut_from_to(c, d));
                }
            }
            (RQS::Two(a, b), RQS::None) |
            (RQS::Two(a, b), RQS::Touch(_)) |
            (RQS::None, RQS::Two(a, b)) |
            (RQS::Touch(_), RQS::Two(a, b)) |
            (RQS::One(a), RQS::One(b)) => {
                // One piece
                let (a, b) = if a > b { (b, a) } else { (a, b) };
                let a = a.min(1.0).max(0.0);
                let b = b.min(1.0).max(0.0);
                if a != b {
                    result.push(self.curve.cut_from_to(a, b));
                }
            }
            (RQS::All, RQS::None) |
            (RQS::None, RQS::All) => {
                // coincident with one plane
                result.push(self.curve);
            }
            (RQS::None, RQS::None) => if self.a == 0.0 && self.b == 0.0
                && self.c_shift >= lower_d && self.c_shift <= upper_d
            {
                // parallel to planes, inbetween
                result.push(self.curve);
            },
            _ => unreachable!() // impossible
        }
        //println!("{:?}", result);
        Some(result.into_iter())
    }
}

#[derive(Debug)]
struct PlaneSet {
    start: f32,
    step: f32,
    count: usize
}

impl SliceUp for Curve {
    type PerSlice = CurveIter;
    type Out = CurveSliceIter;
    fn slice_up_x(&self, planes: PlaneSet) -> CurveSliceIter {
        let p = &self.p;
        CurveSliceIter {
            curve: *self,
            planes: planes,
            i: 0,
            a: p[0].x - 2.0 * p[1].x + p[2].x,
            b: 2.0 * (p[1].x - p[0].x),
            c_shift: p[0].x
        }
    }
    fn slice_up_y(&self, planes: PlaneSet) -> CurveSliceIter {
        let p = &self.p;
        CurveSliceIter {
            curve: *self,
            planes: planes,
            i: 0,
            a: p[0].y - 2.0 * p[1].y + p[2].y,
            b: 2.0 * (p[1].y - p[0].y),
            c_shift: p[0].y
        }
    }
}

pub fn rasterize<O: FnMut(u32, u32, f32)>(lines: &[Line], curves: &[Curve],
                                          width: u32, height: u32,
                                          mut output: O) {
    use ::std::collections::HashMap;
    let mut lines: Vec<_> = lines.iter().map(|&l| (l, l.bounding_box())).collect();
    lines[..].sort_by(|&(_, ref a), &(_, ref b)| a.min.y.partial_cmp(&b.min.y).unwrap());
    let mut curves: Vec<_> = curves.iter().map(|&c| (c, c.bounding_box())).collect();
    curves[..].sort_by(|&(_, ref a), &(_, ref b)| a.min.y.partial_cmp(&b.min.y).unwrap());
    let mut y = 0;
    let mut next_line = 0; let mut next_curve = 0;
    let mut active_lines_y  = HashMap::new(); let mut active_curves_y = HashMap::new();
    let mut active_lines_x  = HashMap::new(); let mut active_curves_x = HashMap::new();
    let mut scanline_lines = Vec::new();
    let mut lines_to_remove = Vec::new();
    let mut scanline_curves = Vec::new();
    let mut curves_to_remove = Vec::new();
    while y < height && (next_line != lines.len() || next_curve != curves.len()
        || active_lines_y.len() > 0 || active_curves_y.len() > 0)
    {
        let lower = y as f32;
        let upper = (y + 1) as f32;
        // Add newly active segments
        for &(ref line, ref bb) in lines[next_line..].iter().take_while(|p| p.1.min.y < upper) {
            let planes = PlaneSet {
                start: lower,
                step: 1.0,
                count: (bb.max.y.ceil() - lower).max(1.0) as usize
            };
            active_lines_y.insert(next_line, line.slice_up_y(planes));
            next_line += 1;
        }
        for &(ref curve, ref bb) in curves[next_curve..].iter().take_while(|p| p.1.min.y < upper) {
            let planes = PlaneSet {
                start: lower,
                step: 1.0,
                count: (bb.max.y.ceil() - lower).max(1.0) as usize
            };
            active_curves_y.insert(next_curve, curve.slice_up_y(planes));
            next_curve += 1;
        }
        // get y sliced segments for this scanline
        scanline_lines.clear();
        scanline_curves.clear();
        for (k, itr) in active_lines_y.iter_mut() {
            if let Some(itr) = itr.next() {
                for line in itr {
                    scanline_lines.push((line, line.x_bounds()))
                }
            } else {
                lines_to_remove.push(*k);
            }
        }
        for (k, itr) in active_curves_y.iter_mut() {
            if let Some(itr) = itr.next() {
                for curve in itr {
                    scanline_curves.push((curve, curve.x_bounds()))
                }
            } else {
                curves_to_remove.push(*k);
            }
        }
        // remove deactivated segments
        for k in lines_to_remove.drain(..) {
            active_lines_y.remove(&k);
        }
        for k in curves_to_remove.drain(..) {
            active_curves_y.remove(&k);
        }
        // sort scanline for traversal
        scanline_lines.sort_by(|a, b| (a.1).0.partial_cmp(&(b.1).0).unwrap());
        scanline_curves.sort_by(|a, b| (a.1).0.partial_cmp(&(b.1).0).unwrap());
        // Iterate through x, slice scanline segments into each cell. Evaluate, accumulate and output.
        {
            let mut next_line = 0; let mut next_curve = 0;
            let mut x = 0;
            let mut acc = 0.0;
            active_lines_x.clear();
            active_curves_x.clear();
            while x < width && (next_line != scanline_lines.len() || next_curve != scanline_curves.len()
                || active_lines_x.len() > 0 || active_curves_x.len() > 0)
            {
                let offset = vector(x as f32, y as f32);
                let lower = x as f32;
                let upper = (x+1) as f32;
                //add newly active segments
                for &(ref line, (_, ref max)) in scanline_lines[next_line..].iter()
                    .take_while(|p| (p.1).0 < upper)
                {
                    let planes = PlaneSet {
                        start: lower,
                        step: 1.0,
                        count: (max.ceil() - lower).max(1.0) as usize
                    };
                    active_lines_x.insert(next_line, line.slice_up_x(planes));
                    next_line += 1;
                }
                for &(ref curve, (_, ref max)) in scanline_curves[next_curve..].iter()
                    .take_while(|p| (p.1).0 < upper)
                {
                    let planes = PlaneSet {
                        start: lower,
                        step: 1.0,
                        count: (max.ceil() - lower).max(1.0) as usize
                    };
                    active_curves_x.insert(next_curve, curve.slice_up_x(planes));
                    next_curve += 1;
                }
                //process x sliced segments for this pixel
                let mut pixel_value = acc;
                let mut pixel_acc = 0.0;
                for (k, itr) in active_lines_x.iter_mut() {
                    if let Some(itr) = itr.next() {
                        for mut line in itr {
                            let p = &mut line.p;
                            p[0] = p[0] - offset;
                            p[1] = p[1] - offset;

                            let a = p[0].y - p[1].y;
                            let v = (1.0 - (p[0].x + p[1].x) * 0.5) * a;
                            pixel_value += v;
                            pixel_acc += a;
                        }
                    } else {
                        lines_to_remove.push(*k);
                    }
                }
                for (k, itr) in active_curves_x.iter_mut() {
                    if let Some(itr) = itr.next() {
                        for mut curve in itr {
                            let p = &mut curve.p;
                            p[0] = p[0] - offset;
                            p[1] = p[1] - offset;
                            p[2] = p[2] - offset;
                            let a = p[0].y - p[2].y;
                            let b = p[0].y - p[1].y;
                            let c = p[1].y - p[2].y;
                            let v = (b*(6.0 - 3.0*p[0].x - 2.0*p[1].x -     p[2].x) +
                                     c*(6.0 -     p[0].x - 2.0*p[1].x - 3.0*p[2].x)) / 6.0;
                            pixel_value += v;
                            pixel_acc += a;
                        }
                    } else {
                        curves_to_remove.push(*k);
                    }
                }
                //output
                output(x, y, pixel_value);
                acc += pixel_acc;
                // remove deactivated segments
                for k in lines_to_remove.drain(..) {
                    active_lines_x.remove(&k);
                }
                for k in curves_to_remove.drain(..) {
                    active_curves_x.remove(&k);
                }
                x += 1;
            }
            // fill remaining pixels
            for x in x..width {
                output(x, y, acc);
            }
        }
        y += 1;
    }
    // fill remaining scanlines with 0.0
    for y in y..height {
        for x in 0..width {
            output(x, y, 0.0);
        }
    }
}
