use std::ops;

/// A point in 2-dimensional space, with each dimension of type `N`.
///
/// Legal operations on points are addition and subtraction by vectors, and subtraction between points, to give
/// a vector representing the offset between the two points. Combined with the legal operations on vectors,
/// meaningful manipulations of vectors and points can be performed.
///
/// For example, to interpolate between two points by a factor `t`:
///
/// ```
/// # use rusttype::*;
/// # let t = 0.5; let p0 = point(0.0, 0.0); let p1 = point(0.0, 0.0);
/// let interpolated_point = p0 + (p1 - p0) * t;
/// ```
#[derive(Copy, Clone, Debug)]
pub struct Point<N> {
    pub x: N,
    pub y: N
}

/// A vector in 2-dimensional space, with each dimension of type `N`.
///
/// Legal operations on vectors are addition and subtraction by vectors, addition by points (to give points),
/// and multiplication and division by scalars.
#[derive(Copy, Clone, Debug)]
pub struct Vector<N> {
    pub x: N,
    pub y: N
}
/// A convenience function for generating `Point`s.
pub fn point<N>(x: N, y: N) -> Point<N> {
    Point { x: x, y: y }
}
/// A convenience function for generating `Vector`s.
pub fn vector<N>(x: N, y: N) -> Vector<N> {
    Vector { x: x, y: y }
}

impl<N: ops::Sub<Output=N>> ops::Sub for Point<N> {
    type Output = Vector<N>;
    fn sub(self, rhs: Point<N>) -> Vector<N> {
        vector(self.x - rhs.x, self.y - rhs.y)
    }
}

impl<N: ops::Add<Output=N>> ops::Add for Vector<N> {
    type Output = Vector<N>;
    fn add(self, rhs: Vector<N>) -> Vector<N> {
        vector(self.x + rhs.x, self.y + rhs.y)
    }
}

impl<N: ops::Sub<Output=N>> ops::Sub for Vector<N> {
    type Output = Vector<N>;
    fn sub(self, rhs: Vector<N>) -> Vector<N> {
        vector(self.x - rhs.x, self.y - rhs.y)
    }
}

impl ops::Mul<f32> for Vector<f32> {
    type Output = Vector<f32>;
    fn mul(self, rhs: f32) -> Vector<f32> {
        vector(self.x * rhs, self.y * rhs)
    }
}

impl ops::Mul<Vector<f32>> for f32 {
    type Output = Vector<f32>;
    fn mul(self, rhs: Vector<f32>) -> Vector<f32> {
        vector(self * rhs.x, self * rhs.y)
    }
}

impl ops::Mul<f64> for Vector<f64> {
    type Output = Vector<f64>;
    fn mul(self, rhs: f64) -> Vector<f64> {
        vector(self.x * rhs, self.y * rhs)
    }
}

impl ops::Mul<Vector<f64>> for f64 {
    type Output = Vector<f64>;
    fn mul(self, rhs: Vector<f64>) -> Vector<f64> {
        vector(self * rhs.x, self * rhs.y)
    }
}

impl<N: ops::Add<Output=N>> ops::Add<Vector<N>> for Point<N> {
    type Output = Point<N>;
    fn add(self, rhs: Vector<N>) -> Point<N> {
        point(self.x + rhs.x, self.y + rhs.y)
    }
}

impl<N: ops::Sub<Output=N>> ops::Sub<Vector<N>> for Point<N> {
    type Output = Point<N>;
    fn sub(self, rhs: Vector<N>) -> Point<N> {
        point(self.x - rhs.x, self.y - rhs.y)
    }
}

impl<N: ops::Add<Output=N>> ops::Add<Point<N>> for Vector<N> {
    type Output = Point<N>;
    fn add(self, rhs: Point<N>) -> Point<N> {
        point(self.x + rhs.x, self.y + rhs.y)
    }
}

/// A straight line between two points, `p[0]` and `p[1]`
#[derive(Copy, Clone, Debug)]
pub struct Line {
    pub p: [Point<f32>; 2]
}
/// A quadratic Bezier curve, starting at `p[0]`, ending at `p[2]`, with control point `p[1]`.
#[derive(Copy, Clone, Debug)]
pub struct Curve {
    pub p: [Point<f32>; 3]
}
/// A rectangle, with top-left corner at `min`, and bottom-right corner at `max`.
#[derive(Copy, Clone, Debug)]
pub struct Rect<N> {
    pub min: Point<N>,
    pub max: Point<N>
}
impl<N: ops::Sub<Output=N> + Copy> Rect<N> {
    pub fn width(&self) -> N {
        self.max.x - self.min.x
    }
    pub fn height(&self) -> N {
        self.max.y - self.min.y
    }
}

pub trait BoundingBox<N> {
    fn bounding_box(&self) -> Rect<N> {
        let (min_x, max_x) = self.x_bounds();
        let (min_y, max_y) = self.y_bounds();
        Rect {
            min: point(min_x, min_y),
            max: point(max_x, max_y)
        }
    }
    fn x_bounds(&self) -> (N, N);
    fn y_bounds(&self) -> (N, N);
}

impl BoundingBox<f32> for Line {
    fn x_bounds(&self) -> (f32, f32) {
        let p = &self.p;
        if p[0].x < p[1].x {
            (p[0].x, p[1].x)
        } else {
            (p[1].x, p[0].x)
        }
    }
    fn y_bounds(&self) -> (f32, f32) {
        let p = &self.p;
        if p[0].y < p[1].y {
            (p[0].y, p[1].y)
        } else {
            (p[1].y, p[0].y)
        }
    }
}

impl BoundingBox<f32> for Curve {
    fn x_bounds(&self) -> (f32, f32) {
        let p = &self.p;
        if p[0].x <= p[1].x && p[1].x <= p[2].x {
            (p[0].x, p[2].x)
        } else if p[0].x >= p[1].x && p[1].x >= p[2].x {
            (p[2].x, p[0].x)
        } else {
            let t = (p[0].x - p[1].x) / (p[0].x - 2.0 * p[1].x + p[2].x);
            let _1mt = 1.0 - t;
            let inflection = _1mt*_1mt*p[0].x + 2.0*_1mt*t*p[1].x + t*t*p[2].x;
            if p[1].x < p[0].x {
                (inflection, p[0].x.max(p[2].x))
            } else {
                (p[0].x.min(p[2].x), inflection)
            }
        }
    }

    fn y_bounds(&self) -> (f32, f32) {
        let p = &self.p;
        if p[0].y <= p[1].y && p[1].y <= p[2].y {
            (p[0].y, p[2].y)
        } else if p[0].y >= p[1].y && p[1].y >= p[2].y {
            (p[2].y, p[0].y)
        } else {
            let t = (p[0].y - p[1].y) / (p[0].y - 2.0 * p[1].y + p[2].y);
            let _1mt = 1.0 - t;
            let inflection = _1mt*_1mt*p[0].y + 2.0*_1mt*t*p[1].y + t*t*p[2].y;
            if p[1].y < p[0].y {
                (inflection, p[0].y.max(p[2].y))
            } else {
                (p[0].y.min(p[2].y), inflection)
            }
        }
    }
}

pub trait Cut: Sized {
    fn cut_to(self, t: f32) -> Self;
    fn cut_from(self, t: f32) -> Self;
    fn cut_from_to(self, t0: f32, t1: f32) -> Self {
        self.cut_from(t0).cut_to((t1-t0)/(1.0-t0))
    }
}

impl Cut for Curve {
    fn cut_to(self, t: f32) -> Curve {
        let p = self.p;
        let a = p[0] + t * (p[1] - p[0]);
        let b = p[1] + t * (p[2] - p[1]);
        let c = a + t * (b-a);
        Curve {
            p: [p[0], a, c]
        }
    }
    fn cut_from(self, t: f32) -> Curve {
        let p = self.p;
        let a = p[0] + t * (p[1] - p[0]);
        let b = p[1] + t * (p[2] - p[1]);
        let c = a + t * (b-a);
        Curve {
            p: [c, b, p[2]]
        }
    }
}

impl Cut for Line {
    fn cut_to(self, t: f32) -> Line {
        let p = self.p;
        Line {
            p: [p[0], p[0] + t * (p[1] - p[0])]
        }
    }
    fn cut_from(self, t: f32) -> Line {
        let p = self.p;
        Line {
            p:[p[0] + t * (p[1] - p[0]), p[1]]
        }
    }
    fn cut_from_to(self, t0: f32, t1: f32) -> Line {
        let p = self.p;
        let v = p[1] - p[0];
        Line {
            p: [p[0] + t0 * v, p[0] + t1 * v]
        }
    }
}

/// The real valued solutions to a real quadratic equation.
#[derive(Copy, Clone, Debug)]
pub enum RealQuadraticSolution {
    /// Two zero-crossing solutions
    Two(f32, f32),
    /// One zero-crossing solution (equation is a straight line)
    One(f32),
    /// One zero-touching solution
    Touch(f32),
    /// No solutions
    None,
    /// All real numbers are solutions since a == b == c == 0.0
    All
}

impl RealQuadraticSolution {
    /// If there are two solutions, this function ensures that they are in order (first < second)
    pub fn in_order(self) -> RealQuadraticSolution {
        use self::RealQuadraticSolution::*;
        match self {
            Two(x, y) => if x < y { Two(x, y) } else { Two(y, x) },
            other => other
        }
    }
}

/// Solve a real quadratic equation, giving all real solutions, if any.
pub fn solve_quadratic_real(a: f32, b: f32, c: f32) -> RealQuadraticSolution {
    let discriminant = b*b - 4.0 * a * c;
    if discriminant > 0.0 {
        let sqrt_d = discriminant.sqrt();
        let common = -b + if b >= 0.0 {
            -sqrt_d
        } else {
            sqrt_d
        };
        let x1 = 2.0 * c / common;
        if a == 0.0 {
            RealQuadraticSolution::One(x1)
        } else {
            let x2 = common / (2.0 * a);
            RealQuadraticSolution::Two(x1, x2)
        }
    } else if discriminant < 0.0 {
        RealQuadraticSolution::None
    } else { // discriminant == 0.0
        if b == 0.0 {
            if a == 0.0 {
                if c == 0.0 {
                    RealQuadraticSolution::All
                } else {
                    RealQuadraticSolution::None
                }
            } else {
                RealQuadraticSolution::Touch(0.0)
            }
        } else {
            RealQuadraticSolution::Touch(2.0 * c / -b)
        }
    }
}

#[test]
fn quadratic_test() {
    solve_quadratic_real(-0.0000001, -2.0, 10.0);
}
