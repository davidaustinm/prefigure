//! Port of prefig/core/implicit.py: implicit curves via quadtree subdivision
//! and Newton's method.

use crate::core::diagram::Diagram;
use crate::core::math_utilities::midpoint;
use crate::core::utilities::{self as util, pt2str};
use crate::evaluator::interp_call;
use crate::value::Value;
use crate::xml::{self, El};

type Point = [f64; 2];

struct LevelSet<'a> {
    f: &'a Value,
    k: f64,
}

impl LevelSet<'_> {
    fn value(&self, p: Point, diagram: &mut Diagram) -> f64 {
        interp_call(
            self.f,
            &[Value::Num(p[0]), Value::Num(p[1])],
            &mut diagram.ctx,
        )
        .ok()
        .and_then(|v| v.as_num().ok())
        .map(|v| v - self.k)
        .unwrap_or(f64::NAN)
    }
}

struct QuadTree {
    corners: [Point; 4],
    depth: i64,
}

impl QuadTree {
    fn subdivide(&self) -> Vec<QuadTree> {
        let c = &self.corners;
        let bottom = midpoint(c[0], c[1]);
        let left = midpoint(c[0], c[3]);
        let right = midpoint(c[1], c[2]);
        let top = midpoint(c[2], c[3]);
        let mid = midpoint(bottom, top);
        vec![
            QuadTree {
                corners: [c[0], bottom, mid, left],
                depth: self.depth - 1,
            },
            QuadTree {
                corners: [bottom, c[1], right, mid],
                depth: self.depth - 1,
            },
            QuadTree {
                corners: [left, mid, top, c[3]],
                depth: self.depth - 1,
            },
            QuadTree {
                corners: [mid, right, c[2], top],
                depth: self.depth - 1,
            },
        ]
    }

    fn intersects(&self, g: &LevelSet, diagram: &mut Diagram) -> bool {
        let mut sign = g.value(self.corners[3], diagram);
        for i in 0..4 {
            let nextsign = g.value(self.corners[i], diagram);
            if sign * nextsign <= 0.0 {
                return true;
            }
            sign = nextsign;
        }
        false
    }

    fn findzero(&self, p1: Point, p2: Point, g: &LevelSet, diagram: &mut Diagram) -> Point {
        let dx = p2[0] - p1[0];
        let dy = p2[1] - p1[1];
        let change = 0.00001;
        let (dx, dy, dt) = if dx != 0.0 {
            let dx = change * dx.abs() / dx;
            (dx, 0.0, dx)
        } else {
            let dy = change * dy.abs() / dy;
            (0.0, dy, dy)
        };
        let mut p = p1;
        let mut diff = 1.0f64;
        let mut n = 0;
        while diff.abs() > 0.000001 && n < 50 {
            let f = g.value(p, diagram);
            if f == 0.0 {
                break;
            }
            let df = (g.value([p[0] + dx, p[1] + dy], diagram) - f) / dt;
            diff = f / df;
            p = if dx != 0.0 {
                [p[0] - diff, p[1]]
            } else {
                [p[0], p[1] - diff]
            };
            n += 1;
        }
        p
    }

    fn segments(&self, g: &LevelSet, diagram: &mut Diagram) -> Vec<(Point, Point)> {
        let mut corner = self.corners[3];
        let mut sign = g.value(corner, diagram);
        let mut segments = Vec::new();
        let mut last_zero: Option<Point> = None;
        for i in 0..4 {
            let nextcorner = self.corners[i];
            let nextsign = g.value(nextcorner, diagram);
            if sign == 0.0 && nextsign == 0.0 {
                segments.push((corner, nextcorner));
            } else if sign * nextsign <= 0.0 {
                match last_zero {
                    None => last_zero = Some(self.findzero(corner, nextcorner, g, diagram)),
                    Some(lz) => {
                        let this_zero = self.findzero(corner, nextcorner, g, diagram);
                        segments.push((lz, this_zero));
                        last_zero = Some(this_zero);
                    }
                }
            }
            corner = nextcorner;
            sign = nextsign;
        }
        segments
    }
}

pub fn implicit_curve(
    element: &El,
    diagram: &mut Diagram,
    parent: &El,
    outline_group: Option<&El>,
) {
    if diagram.output_format() == "tactile" {
        element.borrow_mut().set("stroke", "black");
    } else {
        util::set_attr(element, "stroke", "black", &mut diagram.ctx);
    }
    util::set_attr(element, "thickness", "2", &mut diagram.ctx);

    let bbox = diagram.bbox();
    let function_attr = element.borrow().get("function").unwrap_or_default();
    let Ok(f) = diagram.ctx.valid_eval(&function_attr) else {
        log::error!("Error in <implicit-curve> retrieving function={function_attr}");
        return;
    };

    let k = {
        let value_attr = element.borrow().get("value");
        let k_attr = value_attr.unwrap_or_else(|| element.borrow().get_or("k", "0"));
        match diagram.ctx.valid_eval(&k_attr).and_then(|v| v.as_num()) {
            Ok(k) => k,
            Err(_) => {
                log::error!("Error in <implicit-curve> retrieving k");
                return;
            }
        }
    };
    let eval_int = |diagram: &mut Diagram, attr: String, default: i64| -> i64 {
        diagram
            .ctx
            .valid_eval(&attr)
            .ok()
            .and_then(|v| v.as_num().ok())
            .map(|n| n as i64)
            .unwrap_or(default)
    };
    let depth_attr = element.borrow().get_or("depth", "8");
    let depth = eval_int(diagram, depth_attr, 8);
    let initial_attr = element.borrow().get_or("initial-depth", "4");
    let initial_depth = eval_int(diagram, initial_attr, 4);

    let levelset = LevelSet { f: &f, k };

    // build the quadtree and collect segments
    let root = QuadTree {
        corners: [
            [bbox[0], bbox[1]],
            [bbox[2], bbox[1]],
            [bbox[2], bbox[3]],
            [bbox[0], bbox[3]],
        ],
        depth,
    };
    let mut tree = vec![root];
    for _ in 0..initial_depth {
        let mut newtree = Vec::new();
        for node in &tree {
            newtree.extend(node.subdivide());
        }
        tree = newtree;
    }
    let mut segments: Vec<(Point, Point)> = Vec::new();
    let mut queue = std::collections::VecDeque::from(tree);
    while let Some(node) = queue.pop_front() {
        if node.depth == 0 {
            segments.extend(node.segments(&levelset, diagram));
        } else if node.intersects(&levelset, diagram) {
            queue.extend(node.subdivide());
        }
    }

    let mut cmds = Vec::new();
    for (s0, s1) in &segments {
        let p0 = diagram.transform(*s0);
        let p1 = diagram.transform(*s1);
        cmds.push(format!("M {}", pt2str(p0, " ")));
        cmds.push(format!("L {}", pt2str(p1, " ")));
    }

    let path = xml::new_element("path");
    let id = element.borrow().get("id");
    diagram.add_id(&path, id.as_deref());
    diagram.register_svg_element(element, &path);
    path.borrow_mut().set("d", &cmds.join(" "));

    util::add_attr(&path, util::get_1d_attr(element, &mut diagram.ctx));

    if let Some(outline_group) = outline_group {
        diagram.add_outline(element, &path, outline_group, None, None);
        finish_outline(element, diagram, parent);
    } else if element.borrow().get_or("outline", "no") == "yes"
        || diagram.output_format() == "tactile"
    {
        diagram.add_outline(element, &path, parent, None, None);
        finish_outline(element, diagram, parent);
    } else {
        xml::append(parent, &path);
    }
}

fn finish_outline(element: &El, diagram: &mut Diagram, parent: &El) {
    let stroke = element.borrow().get("stroke");
    let thickness = element.borrow().get("thickness");
    let fill = element.borrow().get_or("fill", "none");
    diagram.finish_outline(element, stroke, thickness, &fill, parent, None);
}
