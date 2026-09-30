use std::ops::{Add, AddAssign, Div, Mul, Neg, Sub};

#[derive(Clone, Copy, Debug, Default)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
    pub fn dot(self, b: Self) -> f32 {
        self.x * b.x + self.y * b.y + self.z * b.z
    }
    pub fn cross(self, b: Self) -> Self {
        Self::new(
            self.y * b.z - self.z * b.y,
            self.z * b.x - self.x * b.z,
            self.x * b.y - self.y * b.x,
        )
    }
    pub fn length(self) -> f32 {
        self.dot(self).sqrt()
    }
    pub fn normalized(self) -> Self {
        let l = self.length();
        if l > 0.0 {
            self / l
        } else {
            self
        }
    }
    pub fn hadamard(self, b: Self) -> Self {
        Self::new(self.x * b.x, self.y * b.y, self.z * b.z)
    }
    pub fn clamp(self, lo: f32, hi: f32) -> Self {
        Self::new(
            self.x.clamp(lo, hi),
            self.y.clamp(lo, hi),
            self.z.clamp(lo, hi),
        )
    }
    pub fn reflect(self, n: Self) -> Self {
        self - n * (2.0 * self.dot(n))
    }
    pub fn refract(self, n: Self, eta: f32) -> Option<Self> {
        let cosi = (-self).dot(n).clamp(-1.0, 1.0);
        let k = 1.0 - eta * eta * (1.0 - cosi * cosi);
        (k >= 0.0).then(|| self * eta + n * (eta * cosi - k.sqrt()))
    }
    pub fn lerp(self, b: Self, t: f32) -> Self {
        self * (1.0 - t) + b * t
    }
}
impl Add for Vec3 {
    type Output = Self;
    fn add(self, b: Self) -> Self {
        Self::new(self.x + b.x, self.y + b.y, self.z + b.z)
    }
}
impl AddAssign for Vec3 {
    fn add_assign(&mut self, b: Self) {
        *self = *self + b
    }
}
impl Sub for Vec3 {
    type Output = Self;
    fn sub(self, b: Self) -> Self {
        Self::new(self.x - b.x, self.y - b.y, self.z - b.z)
    }
}
impl Mul<f32> for Vec3 {
    type Output = Self;
    fn mul(self, s: f32) -> Self {
        Self::new(self.x * s, self.y * s, self.z * s)
    }
}
impl Div<f32> for Vec3 {
    type Output = Self;
    fn div(self, s: f32) -> Self {
        self * (1.0 / s)
    }
}
impl Neg for Vec3 {
    type Output = Self;
    fn neg(self) -> Self {
        self * -1.0
    }
}

pub type Color = Vec3;
pub const WHITE: Color = Vec3::new(1.0, 1.0, 1.0);
pub fn hash(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846ca68b);
    x ^ (x >> 16)
}
pub fn noise3(x: i32, y: i32, z: i32) -> f32 {
    hash(
        (x as u32).wrapping_mul(73856093)
            ^ (y as u32).wrapping_mul(19349663)
            ^ (z as u32).wrapping_mul(83492791),
    ) as f32
        / u32::MAX as f32
}
pub fn smoothstep(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}
pub fn value_noise(x: f32, z: f32) -> f32 {
    let xi = x.floor() as i32;
    let zi = z.floor() as i32;
    let tx = smoothstep(x - x.floor());
    let tz = smoothstep(z - z.floor());
    let a = noise3(xi, 0, zi);
    let b = noise3(xi + 1, 0, zi);
    let c = noise3(xi, 0, zi + 1);
    let d = noise3(xi + 1, 0, zi + 1);
    (a + (b - a) * tx) + ((c + (d - c) * tx) - (a + (b - a) * tx)) * tz
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reflection_and_normalization_are_correct() {
        let r = Vec3::new(1.0, -1.0, 0.0)
            .normalized()
            .reflect(Vec3::new(0.0, 1.0, 0.0));
        assert!(r.y > 0.0 && (r.length() - 1.0).abs() < 1e-5);
    }
    #[test]
    fn procedural_noise_is_deterministic() {
        assert_eq!(noise3(3, 7, 11), noise3(3, 7, 11));
    }
}
