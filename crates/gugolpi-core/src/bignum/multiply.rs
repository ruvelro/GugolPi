//! Multiplicación de [`Fixed`] por convolución FFT en `f64`, con control del error de redondeo.
//!
//! Dos números de `n` limbs se multiplican empaquetando uno en la parte real y otro en la
//! imaginaria de una sola FFT compleja (truco "dos por uno"); el cuadrado usa una sola
//! transformada directa. Los limbs se convierten a dígitos balanceados en `[-5000, 5000)` antes
//! de transformar, para que los coeficientes se cancelen y el error de redondeo quede muy por
//! debajo del límite. Tras la inversa, cada coeficiente debe ser casi entero: la distancia al
//! entero más cercano es el error de redondeo, y su máximo se conserva para la verificación
//! (SuperPi lo llamaba "not exact in round").

use super::complex::Complex;
use super::fft::FftPlan;
use super::fixed::{BASE, Fixed};
use crate::error::{Error, Result};

/// Multiplicador con plan de FFT y búfer reutilizables.
///
/// Guarda un único plan, el del mayor tamaño visto: su tabla de twiddles sirve para todos los
/// tamaños menores, así que la memoria de twiddles es la de una sola transformada.
#[derive(Debug, Default)]
pub struct Multiplier {
    plan: Option<FftPlan>,
    buf: Vec<Complex>,
    max_error: f64,
}

impl Multiplier {
    /// Multiplicador vacío; los planes se crean bajo demanda.
    pub fn new() -> Self {
        Self::default()
    }

    /// Mayor error de redondeo visto en cualquier coeficiente desde la creación.
    pub fn max_error(&self) -> f64 {
        self.max_error
    }

    /// Producto truncado a `len` limbs. Los operandos se truncan a `len` limbs antes de
    /// multiplicar.
    pub fn mul(&mut self, a: &Fixed, b: &Fixed, len: usize) -> Result<Fixed> {
        self.product(a, Some(b), len)
    }

    /// Cuadrado truncado a `len` limbs.
    pub fn square(&mut self, a: &Fixed, len: usize) -> Result<Fixed> {
        self.product(a, None, len)
    }

    fn product(&mut self, a: &Fixed, b: Option<&Fixed>, len: usize) -> Result<Fixed> {
        let len = len.max(1);
        let a_op = Operand::balanced(a.limbs(), len);
        let b_op = match b {
            Some(b) => Operand::balanced(b.limbs(), len),
            None => a_op.clone(),
        };
        if a_op.digits.is_empty() || b_op.digits.is_empty() {
            return Ok(Fixed::zero(len));
        }
        let conv_len = a_op.digits.len() + b_op.digits.len() - 1;
        let n = conv_len.next_power_of_two().max(2);

        let Self {
            plan,
            buf,
            max_error,
        } = self;
        if plan.as_ref().is_none_or(|p| p.len() < n) {
            *plan = Some(FftPlan::new(n));
        }
        let Some(plan) = plan.as_ref() else {
            return Err(Error::Internal("sin plan de FFT".to_owned()));
        };

        buf.clear();
        buf.resize(n, Complex::default());
        let packed = a_op.digits.len().max(b_op.digits.len());
        for (i, slot) in buf.iter_mut().enumerate().take(packed) {
            let re = a_op.digits.get(i).copied().map_or(0.0, f64::from);
            let im = if b.is_some() {
                b_op.digits.get(i).copied().map_or(0.0, f64::from)
            } else {
                0.0
            };
            *slot = Complex::new(re, im);
        }
        plan.forward(buf);
        if b.is_some() {
            pointwise_two_for_one(buf);
        } else {
            for x in buf.iter_mut() {
                *x = *x * *x;
            }
        }
        plan.inverse(buf);

        // Redondeo, error y acarreo con signo desde el limb menos significativo.
        let base = i64::from(BASE);
        let mut out = vec![0_u32; conv_len + 1];
        let mut carry = 0_i64;
        let mut worst = 0.0_f64;
        for k in (0..conv_len).rev() {
            let c = buf[k].re;
            let rounded = c.round();
            worst = worst.max((c - rounded).abs());
            let value = rounded as i64 + carry;
            out[k + 1] = value.rem_euclid(base) as u32;
            carry = value.div_euclid(base);
        }
        *max_error = max_error.max(worst);
        if !(0..base).contains(&carry) {
            return Err(Error::Internal(
                "desbordamiento en la multiplicación".to_owned(),
            ));
        }
        out[0] = carry as u32;

        // `out[m]` pesa BASE^-(offset_a + offset_b - 1 + m).
        let shift = a_op.offset + b_op.offset - 1;
        let mut limbs = vec![0_u32; len];
        for (m, &value) in out.iter().enumerate() {
            let j = shift + m.cast_signed();
            if j < 0 {
                if value != 0 {
                    return Err(Error::Internal(
                        "la parte entera del producto no cabe en un limb".to_owned(),
                    ));
                }
            } else if (j as usize) < len {
                limbs[j as usize] = value;
            }
        }
        Ok(Fixed::from_limbs(limbs))
    }
}

/// Operando en dígitos balanceados: cada dígito está en `[-BASE/2, BASE/2)` y el valor es
/// `Σ digits[i] · BASE^-(offset + i)`. Con dígitos balanceados los coeficientes de la convolución
/// se cancelan en vez de acumularse, y el error de redondeo de la FFT baja varios órdenes de
/// magnitud respecto a los dígitos en `[0, BASE)`.
#[derive(Clone, Debug)]
struct Operand {
    digits: Vec<i32>,
    offset: isize,
}

impl Operand {
    /// Trunca a `len` limbs, balancea y elimina ceros iniciales y finales.
    fn balanced(limbs: &[u32], len: usize) -> Self {
        let slice = &limbs[..limbs.len().min(len)];
        let half = (BASE / 2).cast_signed();
        let base = BASE.cast_signed();
        // `extended[0]` recoge el acarreo que sale por arriba; pesa BASE^1.
        let mut extended = vec![0_i32; slice.len() + 1];
        let mut carry = 0_i32;
        for i in (0..slice.len()).rev() {
            let mut d = slice[i].cast_signed() + carry;
            if d >= half {
                d -= base;
                carry = 1;
            } else {
                carry = 0;
            }
            extended[i + 1] = d;
        }
        extended[0] = carry;
        let leading = extended.iter().take_while(|&&d| d == 0).count();
        if leading == extended.len() {
            return Self {
                digits: Vec::new(),
                offset: 0,
            };
        }
        let trailing = extended.iter().rev().take_while(|&&d| d == 0).count();
        let digits = extended[leading..extended.len() - trailing].to_vec();
        Self {
            digits,
            offset: leading.cast_signed() - 1,
        }
    }
}

/// Separa los espectros de las dos secuencias reales empaquetadas y deja su producto en `buf`.
fn pointwise_two_for_one(buf: &mut [Complex]) {
    let n = buf.len();
    for k in 0..=n / 2 {
        let j = (n - k) % n;
        let xk = buf[k];
        let xj = buf[j];
        let a = (xk + xj.conj()).scale(0.5);
        let b = (xk - xj.conj()).mul_neg_i().scale(0.5);
        let c = a * b;
        buf[k] = c;
        buf[j] = c.conj();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schoolbook(a: &[u32], b: &[u32]) -> Vec<u32> {
        // Producto exacto de dos números de punto fijo con la misma convención de limbs.
        let mut acc = vec![0_u128; a.len() + b.len()];
        for (i, &x) in a.iter().enumerate() {
            for (j, &y) in b.iter().enumerate() {
                acc[i + j + 1] += u128::from(x) * u128::from(y);
            }
        }
        let mut carry = 0_u128;
        for v in acc.iter_mut().rev() {
            let cur = *v + carry;
            *v = cur % u128::from(BASE);
            carry = cur / u128::from(BASE);
        }
        acc.iter().map(|&v| v as u32).collect()
    }

    fn pseudo_random_limbs(len: usize, seed: u64, first: u32) -> Vec<u32> {
        let mut state = seed;
        let mut limbs: Vec<u32> = (0..len)
            .map(|_| {
                state = state
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1);
                ((state >> 33) % u64::from(BASE)) as u32
            })
            .collect();
        limbs[0] = first;
        limbs
    }

    #[test]
    fn matches_schoolbook_product() {
        let mut m = Multiplier::new();
        for &len in &[2_usize, 3, 7, 16, 100, 1000] {
            let a = pseudo_random_limbs(len, 1, 3);
            let b = pseudo_random_limbs(len, 2, 1);
            let want = schoolbook(&a, &b);
            let got = m
                .mul(&Fixed::from_limbs(a), &Fixed::from_limbs(b), len)
                .expect("mul");
            assert_eq!(got.limbs(), &want[1..=len], "len = {len}");
        }
        assert!(m.max_error() < 1e-3);
    }

    #[test]
    fn square_matches_schoolbook() {
        let mut m = Multiplier::new();
        for &len in &[1_usize, 5, 64, 513] {
            let a = pseudo_random_limbs(len, 9, 2);
            let want = schoolbook(&a, &a);
            let got = m.square(&Fixed::from_limbs(a), len).expect("square");
            assert_eq!(got.limbs(), &want[1..=len], "len = {len}");
        }
    }

    #[test]
    fn leading_zeros_keep_weights() {
        let mut m = Multiplier::new();
        // 0.0001 × 0.0001 = 0.00000001 → limb índice 2.
        let a = Fixed::from_limbs(vec![0, 1, 0, 0]);
        let got = m.mul(&a, &a, 4).expect("mul");
        assert_eq!(got.limbs(), &[0, 0, 1, 0]);
        // 2 × 0.5 = 1.
        let two = Fixed::from_u32(2, 3);
        let half = Fixed::from_limbs(vec![0, 5000, 0]);
        assert_eq!(m.mul(&two, &half, 3).expect("mul").limbs(), &[1, 0, 0]);
    }

    #[test]
    fn balanced_operand_preserves_value() {
        // 1.9999 → dígitos balanceados [2, -1] con offset 0.
        let op = Operand::balanced(&[1, 9999], 2);
        assert_eq!(op.digits, vec![2, -1]);
        assert_eq!(op.offset, 0);
        // 0.0000 5000 → [0, 0, 5000] → balanceado [0, 1, -5000] → [1, -5000] con offset 1.
        let op = Operand::balanced(&[0, 0, 5000], 3);
        assert_eq!(op.digits, vec![1, -5000]);
        assert_eq!(op.offset, 1);
        assert!(Operand::balanced(&[0, 0, 0], 3).digits.is_empty());
    }

    #[test]
    fn integer_overflow_is_an_error() {
        let mut m = Multiplier::new();
        let big = Fixed::from_u32(9999, 2);
        assert!(m.mul(&big, &big, 2).is_err());
    }

    #[test]
    fn longer_operands_are_truncated_to_len() {
        let mut m = Multiplier::new();
        let a = Fixed::from_limbs(vec![1, 5000, 1234, 5678]);
        let got = m.mul(&a, &a, 2).expect("mul");
        // Sólo se usan [1, 5000]: 1.5² = 2.25.
        assert_eq!(got.limbs(), &[2, 2500]);
    }
}
