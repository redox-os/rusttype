use crate::{Point, Vector};
use ab_glyph_rasterizer::{point as ab_point, Point as AbPoint, Rasterizer};

pub(crate) struct OutlineRasterizer {
    pub(crate) rasterizer: Rasterizer,
    last: AbPoint,
    last_move: Option<AbPoint>,
    position: AbPoint,
    scale: Vector<f32>,
}

impl OutlineRasterizer {
    pub(crate) fn new(position: Point<f32>, scale: Vector<f32>, width: usize, height: usize) -> Self {
        Self {
            rasterizer: Rasterizer::new(width, height),
            last: ab_point(0.0, 0.0),
            last_move: None,
            position: ab_point(position.x, position.y),
            scale,
        }
    }
}

impl ttf_parser::OutlineBuilder for OutlineRasterizer {
    fn move_to(&mut self, x: f32, y: f32) {
        self.last = AbPoint {
            x: x as f32 * self.scale.x + self.position.x,
            y: -y as f32 * self.scale.y + self.position.y,
        };
        self.last_move = Some(self.last);
    }

    fn line_to(&mut self, x1: f32, y1: f32) {
        let p1 = AbPoint {
            x: x1 as f32 * self.scale.x + self.position.x,
            y: -y1 as f32 * self.scale.y + self.position.y,
        };

        self.rasterizer.draw_line(self.last, p1);
        self.last = p1;
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32) {
        let p1 = AbPoint {
            x: x1 as f32 * self.scale.x + self.position.x,
            y: -y1 as f32 * self.scale.y + self.position.y,
        };
        let p2 = AbPoint {
            x: x2 as f32 * self.scale.x + self.position.x,
            y: -y2 as f32 * self.scale.y + self.position.y,
        };

        self.rasterizer.draw_quad(self.last, p1, p2);
        self.last = p2;
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x3: f32, y3: f32) {
        let p1 = AbPoint {
            x: x1 as f32 * self.scale.x + self.position.x,
            y: -y1 as f32 * self.scale.y + self.position.y,
        };
        let p2 = AbPoint {
            x: x2 as f32 * self.scale.x + self.position.x,
            y: -y2 as f32 * self.scale.y + self.position.y,
        };
        let p3 = AbPoint {
            x: x3 as f32 * self.scale.x + self.position.x,
            y: -y3 as f32 * self.scale.y + self.position.y,
        };

        self.rasterizer.draw_cubic(self.last, p1, p2, p3);
        self.last = p3;
    }

    fn close(&mut self) {
        if let Some(m) = self.last_move {
            self.rasterizer.draw_line(self.last, m);
        }
    }
}
