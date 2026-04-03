use std::ops::{Add, Sub, Mul, Neg};

#[derive(Clone, Copy, Debug, Default)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Self { Self { x, y, z } }
    pub fn zero() -> Self { Self::new(0.0, 0.0, 0.0) }

    pub fn dot(self, rhs: Self) -> f32 {
        self.x * rhs.x + self.y * rhs.y + self.z * rhs.z
    }

    pub fn cross(self, rhs: Self) -> Self {
        Self::new(
            self.y * rhs.z - self.z * rhs.y,
            self.z * rhs.x - self.x * rhs.z,
            self.x * rhs.y - self.y * rhs.x,
        )
    }

    pub fn length(self) -> f32 {
        self.dot(self).sqrt()
    }

    pub fn normalize(self) -> Self {
        let len = self.length();
        if len < 1e-8 { return Self::zero(); }
        Self::new(self.x / len, self.y / len, self.z / len)
    }

    pub fn scale(self, s: f32) -> Self {
        Self::new(self.x * s, self.y * s, self.z * s)
    }
}

impl Add for Vec3 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl Sub for Vec3 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl Neg for Vec3 {
    type Output = Self;
    fn neg(self) -> Self { Self::new(-self.x, -self.y, -self.z) }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Vec4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Vec4 {
    pub fn new(x: f32, y: f32, z: f32, w: f32) -> Self { Self { x, y, z, w } }

    pub fn from_vec3(v: Vec3, w: f32) -> Self {
        Self::new(v.x, v.y, v.z, w)
    }

    pub fn xyz(self) -> Vec3 {
        Vec3::new(self.x, self.y, self.z)
    }

    pub fn perspective_divide(self) -> Vec3 {
        if self.w.abs() < 1e-8 {
            return Vec3::new(self.x, self.y, self.z);
        }
        Vec3::new(self.x / self.w, self.y / self.w, self.z / self.w)
    }
}

/// Row-major 4x4 matrix. Transform: v_out = M * v (v is column vector)
#[derive(Clone, Copy, Debug)]
pub struct Mat4 {
    pub m: [[f32; 4]; 4],
}

impl Mat4 {
    pub fn identity() -> Self {
        Self {
            m: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    pub fn rotation_x(angle: f32) -> Self {
        let (s, c) = angle.sin_cos();
        let mut m = Self::identity();
        m.m[1][1] = c;  m.m[1][2] = -s;
        m.m[2][1] = s;  m.m[2][2] = c;
        m
    }

    pub fn rotation_y(angle: f32) -> Self {
        let (s, c) = angle.sin_cos();
        let mut m = Self::identity();
        m.m[0][0] = c;  m.m[0][2] = s;
        m.m[2][0] = -s; m.m[2][2] = c;
        m
    }

    pub fn rotation_z(angle: f32) -> Self {
        let (s, c) = angle.sin_cos();
        let mut m = Self::identity();
        m.m[0][0] = c;  m.m[0][1] = -s;
        m.m[1][0] = s;  m.m[1][1] = c;
        m
    }

    pub fn scale(sx: f32, sy: f32, sz: f32) -> Self {
        let mut m = Self::identity();
        m.m[0][0] = sx;
        m.m[1][1] = sy;
        m.m[2][2] = sz;
        m
    }

    pub fn translation(tx: f32, ty: f32, tz: f32) -> Self {
        let mut m = Self::identity();
        m.m[0][3] = tx;
        m.m[1][3] = ty;
        m.m[2][3] = tz;
        m
    }

    /// Perspective projection. fov_y in radians.
    pub fn perspective(fov_y: f32, aspect: f32, near: f32, far: f32) -> Self {
        let f = 1.0 / (fov_y / 2.0).tan();
        let mut m = [[0.0f32; 4]; 4];
        m[0][0] = f / aspect;
        m[1][1] = f;
        m[2][2] = (far + near) / (near - far);
        m[2][3] = (2.0 * far * near) / (near - far);
        m[3][2] = -1.0;
        Self { m }
    }

    /// Look-at view matrix.
    pub fn look_at(eye: Vec3, center: Vec3, up: Vec3) -> Self {
        let f = (center - eye).normalize();
        let r = f.cross(up).normalize();
        let u = r.cross(f);

        let mut m = [[0.0f32; 4]; 4];
        m[0][0] = r.x;  m[0][1] = r.y;  m[0][2] = r.z;  m[0][3] = -r.dot(eye);
        m[1][0] = u.x;  m[1][1] = u.y;  m[1][2] = u.z;  m[1][3] = -u.dot(eye);
        m[2][0] = -f.x; m[2][1] = -f.y; m[2][2] = -f.z; m[2][3] = f.dot(eye);
        m[3][3] = 1.0;
        Self { m }
    }

    pub fn mul_vec4(&self, v: Vec4) -> Vec4 {
        let row = |i: usize| {
            self.m[i][0] * v.x + self.m[i][1] * v.y + self.m[i][2] * v.z + self.m[i][3] * v.w
        };
        Vec4::new(row(0), row(1), row(2), row(3))
    }

    pub fn mul_vec3_dir(&self, v: Vec3) -> Vec3 {
        // Transforms a direction (w=0)
        let row = |i: usize| {
            self.m[i][0] * v.x + self.m[i][1] * v.y + self.m[i][2] * v.z
        };
        Vec3::new(row(0), row(1), row(2))
    }
}

impl Mul for Mat4 {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        let mut result = [[0.0f32; 4]; 4];
        for i in 0..4 {
            for j in 0..4 {
                for k in 0..4 {
                    result[i][j] += self.m[i][k] * rhs.m[k][j];
                }
            }
        }
        Self { m: result }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub fn new(x: f32, y: f32) -> Self { Self { x, y } }
}
