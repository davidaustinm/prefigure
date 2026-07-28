//! Port of prefig/core/CTM.py: the current transformation matrix.
//!
//! A 2-D affine transform in homogeneous coordinates, stored as the 2x3 matrix
//! [[a,b,c],[d,e,f]], plus a 4x4 matrix and eye point for 3-D projection and
//! optional log scaling of either axis.

use crate::core::utilities::{float2longstr, float2str, pt2str};

pub type Mat2x3 = [[f64; 3]; 2];
pub type Mat4 = [[f64; 4]; 4];

pub fn identity() -> Mat2x3 {
    [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]
}

pub fn identity_3d() -> Mat4 {
    let mut m = [[0.0; 4]; 4];
    for (i, row) in m.iter_mut().enumerate() {
        row[i] = 1.0;
    }
    m
}

pub fn translation(x: f64, y: f64) -> Mat2x3 {
    [[1.0, 0.0, x], [0.0, 1.0, y]]
}

pub fn scaling(sx: f64, sy: f64) -> Mat2x3 {
    [[sx, 0.0, 0.0], [0.0, sy, 0.0]]
}

pub fn rotation(theta: f64, degrees: bool) -> Mat2x3 {
    let theta = if degrees { theta.to_radians() } else { theta };
    let (s, c) = theta.sin_cos();
    [[c, -s, 0.0], [s, c, 0.0]]
}

pub fn build_matrix(m: [[f64; 2]; 2]) -> Mat2x3 {
    [[m[0][0], m[0][1], 0.0], [m[1][0], m[1][1], 0.0]]
}

/// CTM.concat: compose m with n (apply n first).
pub fn concat(m: Mat2x3, n: Mat2x3) -> Mat2x3 {
    let cols = [
        [n[0][0], n[1][0], 0.0],
        [n[0][1], n[1][1], 0.0],
        [n[0][2], n[1][2], 1.0],
    ];
    let dot = |row: [f64; 3], col: [f64; 3]| row[0] * col[0] + row[1] * col[1] + row[2] * col[2];
    [
        [dot(m[0], cols[0]), dot(m[0], cols[1]), dot(m[0], cols[2])],
        [dot(m[1], cols[0]), dot(m[1], cols[1]), dot(m[1], cols[2])],
    ]
}

fn mat4_mul(a: &Mat4, b: &Mat4) -> Mat4 {
    let mut out = [[0.0; 4]; 4];
    for i in 0..4 {
        for j in 0..4 {
            out[i][j] = (0..4).map(|k| a[i][k] * b[k][j]).sum();
        }
    }
    out
}

// SVG transform strings

pub fn translatestr(x: f64, y: f64) -> String {
    format!("translate({})", pt2str([x, y], ","))
}

pub fn scalestr(x: f64, y: f64) -> String {
    format!("scale({},{})", crate::value::py_str(x), crate::value::py_str(y))
}

pub fn rotatestr(theta: f64) -> String {
    format!("rotate({})", float2str(-theta))
}

pub fn matrixstr(m: [[f64; 2]; 2]) -> String {
    let parts = [m[0][0], -m[1][0], -m[0][1], m[1][1], 0.0, 0.0];
    let joined: Vec<String> = parts.iter().map(|&p| float2longstr(p)).collect();
    format!("matrix({})", joined.join(","))
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AxisScale {
    Linear,
    Log,
}

impl AxisScale {
    fn apply(self, x: f64) -> f64 {
        match self {
            AxisScale::Linear => x,
            AxisScale::Log => x.log10(),
        }
    }

    fn invert(self, x: f64) -> f64 {
        match self {
            AxisScale::Linear => x,
            AxisScale::Log => 10f64.powf(x),
        }
    }
}

#[derive(Clone)]
pub struct CTM {
    pub ctm: Mat2x3,
    pub inverse: Mat2x3,
    pub ctm_3d: Mat4,
    stack: Vec<(Mat2x3, Mat2x3, Mat4)>,
    pub scale_x: AxisScale,
    pub scale_y: AxisScale,
    pub eye: [f64; 2],
}

impl Default for CTM {
    fn default() -> Self {
        Self::new()
    }
}

impl CTM {
    pub fn new() -> Self {
        CTM {
            ctm: identity(),
            inverse: identity(),
            ctm_3d: identity_3d(),
            stack: Vec::new(),
            scale_x: AxisScale::Linear,
            scale_y: AxisScale::Linear,
            eye: [0.0, 0.0],
        }
    }

    pub fn push(&mut self) {
        self.stack.push((self.ctm, self.inverse, self.ctm_3d));
    }

    pub fn pop(&mut self) {
        match self.stack.pop() {
            Some((ctm, inverse, ctm_3d)) => {
                self.ctm = ctm;
                self.inverse = inverse;
                self.ctm_3d = ctm_3d;
            }
            None => log::error!("Attempt to restore an empty transform"),
        }
    }

    pub fn set_log_x(&mut self) {
        self.scale_x = AxisScale::Log;
    }

    pub fn set_log_y(&mut self) {
        self.scale_y = AxisScale::Log;
    }

    pub fn set_eye(&mut self, eye: &[f64]) {
        if eye[0].abs() < 1e-8 {
            log::error!("The first coordinate of the eye's position must be nonzero");
            return;
        }
        self.eye = [eye[1] / eye[0], eye[2] / eye[0]];
    }

    pub fn translate(&mut self, x: f64, y: f64) {
        self.ctm = concat(self.ctm, translation(x, y));
        self.inverse = concat(translation(-x, -y), self.inverse);
    }

    pub fn translate3d(&mut self, x: f64, y: f64, z: f64) {
        let m = [
            [1.0, 0.0, 0.0, y],
            [0.0, 1.0, 0.0, z],
            [0.0, 0.0, 1.0, x],
            [0.0, 0.0, 0.0, 1.0],
        ];
        self.ctm_3d = mat4_mul(&self.ctm_3d, &m);
    }

    pub fn scale(&mut self, x: f64, y: f64) {
        self.ctm = concat(self.ctm, scaling(x, y));
        self.inverse = concat(scaling(1.0 / x, 1.0 / y), self.inverse);
    }

    pub fn scale3d(&mut self, sx: f64, sy: f64, sz: f64) {
        let m = [
            [sy, 0.0, 0.0, 0.0],
            [0.0, sz, 0.0, 0.0],
            [0.0, 0.0, sx, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        self.ctm_3d = mat4_mul(&self.ctm_3d, &m);
    }

    pub fn rotate(&mut self, theta: f64, degrees: bool) {
        self.ctm = concat(self.ctm, rotation(theta, degrees));
        self.inverse = concat(rotation(-theta, degrees), self.inverse);
    }

    pub fn apply_matrix(&mut self, m: [[f64; 2]; 2]) {
        self.ctm = concat(self.ctm, build_matrix(m));
        let det = m[0][0] * m[1][1] - m[0][1] * m[1][0];
        let inv = [
            [m[1][1] / det, -m[0][1] / det],
            [-m[1][0] / det, m[0][0] / det],
        ];
        self.inverse = concat(build_matrix(inv), self.inverse);
    }

    pub fn transform(&self, p: [f64; 2]) -> [f64; 2] {
        let p = [self.scale_x.apply(p[0]), self.scale_y.apply(p[1]), 1.0];
        [
            self.ctm[0][0] * p[0] + self.ctm[0][1] * p[1] + self.ctm[0][2],
            self.ctm[1][0] * p[0] + self.ctm[1][1] * p[1] + self.ctm[1][2],
        ]
    }

    pub fn inverse_transform(&self, p: [f64; 2]) -> [f64; 2] {
        let p = [p[0], p[1], 1.0];
        let x = self.inverse[0][0] * p[0] + self.inverse[0][1] * p[1] + self.inverse[0][2];
        let y = self.inverse[1][0] * p[0] + self.inverse[1][1] * p[1] + self.inverse[1][2];
        [self.scale_x.invert(x), self.scale_y.invert(y)]
    }

    /// Project a 3-D point (given as [x,y,z,1] after coordinate permutation)
    /// to the screen.
    pub fn project_to_screen(&self, p: [f64; 4]) -> [f64; 2] {
        // permute the coordinates and make homogeneous
        let p = [p[1], p[2], p[0], 1.0];
        let mut out = [0.0; 4];
        for (i, row) in self.ctm_3d.iter().enumerate() {
            out[i] = (0..4).map(|k| row[k] * p[k]).sum();
        }
        [
            out[0] - self.eye[0] * out[2],
            out[1] - self.eye[1] * out[2],
        ]
    }
}
