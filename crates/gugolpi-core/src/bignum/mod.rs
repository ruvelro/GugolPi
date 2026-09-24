//! Aritmética de precisión arbitraria propia, sin GMP.
//!
//! Los números son de punto fijo en base 10^4 ([`Fixed`]); la multiplicación es una convolución
//! por FFT compleja en `f64` ([`Multiplier`]), como en SuperPi; raíz cuadrada e inverso van por
//! Newton con precisión creciente ([`newton`]).
//!
//! Este módulo no sabe qué es un benchmark: sólo calcula.

pub mod complex;
pub mod fft;
pub mod fixed;
pub mod multiply;
pub mod newton;

pub use complex::Complex;
pub use fft::FftPlan;
pub use fixed::{BASE, Fixed, LIMB_DIGITS};
pub use multiply::Multiplier;
