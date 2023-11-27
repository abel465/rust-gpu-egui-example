use core::convert::From;
use core::ops::*;
use spirv_std::glam::Vec2;

#[derive(Copy, Clone)]
pub struct Complex(Vec2);

impl Complex {
    pub fn new(x: f32, y: f32) -> Self {
        Complex::from(Vec2::new(x, y))
    }
    pub const ZERO: Complex = Complex(Vec2::ZERO);
}

impl From<Vec2> for Complex {
    fn from(value: Vec2) -> Self {
        Complex(value)
    }
}

impl Deref for Complex {
    type Target = Vec2;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Complex {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Add for Complex {
    type Output = Self;
    fn add(self, other: Self) -> Self::Output {
        Complex::new(self.x + other.x, self.y + other.y)
    }
}

impl Sub for Complex {
    type Output = Self;
    fn sub(self, other: Self) -> Self::Output {
        Complex::new(self.x - other.x, self.y - other.y)
    }
}

impl Mul for Complex {
    type Output = Self;
    fn mul(self, other: Self) -> Self::Output {
        Complex::new(
            self.x * other.x - self.y * other.y,
            self.x * other.y + self.y * other.x,
        )
    }
}

impl Mul<f32> for Complex {
    type Output = Self;
    fn mul(self, other: f32) -> Self::Output {
        Complex::new(self.x * other, self.y * other)
    }
}

impl Mul<Complex> for f32 {
    type Output = Complex;
    fn mul(self, other: Complex) -> Self::Output {
        Complex::new(self * other.x, self * other.y)
    }
}

impl Div<f32> for Complex {
    type Output = Self;
    fn div(self, other: f32) -> Self::Output {
        Complex::new(self.x / other, self.y / other)
    }
}
