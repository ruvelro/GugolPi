//! Número complejo mínimo en `f64` para la FFT.

use std::ops::{Add, Mul, Sub};

/// Complejo de doble precisión.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Complex {
    /// Parte real.
    pub re: f64,
    /// Parte imaginaria.
    pub im: f64,
}

impl Complex {
    /// Construye `re + i·im`.
    #[inline]
    pub const fn new(re: f64, im: f64) -> Self {
        Self { re, im }
    }

    /// Conjugado.
    #[inline]
    pub fn conj(self) -> Self {
        Self {
            re: self.re,
            im: -self.im,
        }
    }

    /// Producto por un real.
    #[inline]
    pub fn scale(self, k: f64) -> Self {
        Self {
            re: self.re * k,
            im: self.im * k,
        }
    }

    /// Multiplicación por `-i` (rotación de −90°).
    #[inline]
    pub fn mul_neg_i(self) -> Self {
        Self {
            re: self.im,
            im: -self.re,
        }
    }
}

impl Add for Complex {
    type Output = Self;
    #[inline]
    fn add(self, o: Self) -> Self {
        Self {
            re: self.re + o.re,
            im: self.im + o.im,
        }
    }
}

impl Sub for Complex {
    type Output = Self;
    #[inline]
    fn sub(self, o: Self) -> Self {
        Self {
            re: self.re - o.re,
            im: self.im - o.im,
        }
    }
}

impl Mul for Complex {
    type Output = Self;
    #[inline]
    fn mul(self, o: Self) -> Self {
        Self {
            re: self.re * o.re - self.im * o.im,
            im: self.re * o.im + self.im * o.re,
        }
    }
}
