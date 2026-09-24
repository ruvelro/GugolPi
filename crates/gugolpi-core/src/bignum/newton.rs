//! Raíz cuadrada e inverso por Newton con precisión creciente.
//!
//! Cada paso de Newton dobla los limbs correctos, así que la recursión trabaja primero a la
//! mitad de precisión y termina con un único paso a precisión completa: el coste total es
//! poco más que el del último paso.

use std::cmp::Ordering;

use super::fixed::Fixed;
use super::multiply::Multiplier;
use crate::error::Result;

/// Precisión (en limbs) a partir de la cual se recurre a `f64` como aproximación inicial.
const BASE_CASE_LIMBS: usize = 3;

/// `1/√x` con `len` limbs. `x` debe ser positivo y menor que `BASE`.
pub fn inv_sqrt(m: &mut Multiplier, x: &Fixed, len: usize) -> Result<Fixed> {
    let len = len.max(1);
    if len <= BASE_CASE_LIMBS {
        let approx = 1.0 / x.to_f64().max(f64::MIN_POSITIVE).sqrt();
        return Ok(Fixed::from_f64(approx, len));
    }
    let half = len / 2 + 1;
    let r0 = inv_sqrt(m, x, half)?.resized(len);
    // r1 = r0 + r0 · (1 − x·r0²) / 2
    let r0_sq = m.square(&r0, len)?;
    let y = m.mul(x, &r0_sq, len)?;
    let one = Fixed::from_u32(1, len);
    let (e, ord) = one.abs_diff(&y);
    let corr = m.mul(&r0, &e, len)?.half();
    Ok(match ord {
        Ordering::Greater => r0.add(&corr),
        Ordering::Equal => r0,
        Ordering::Less => r0.abs_diff(&corr).0,
    })
}

/// `√x` con `len` limbs por el esquema de Karp–Markstein: inverso de la raíz a media precisión y
/// una corrección final `s + r·(x − s²)/2` a precisión completa.
pub fn sqrt(m: &mut Multiplier, x: &Fixed, len: usize) -> Result<Fixed> {
    let len = len.max(1);
    if x.is_zero() {
        return Ok(Fixed::zero(len));
    }
    let half = len / 2 + 2;
    let r = inv_sqrt(m, x, half)?.resized(len);
    let s = m.mul(x, &r, len)?;
    let s_sq = m.square(&s, len)?;
    let x_len = x.resized(len);
    let (d, ord) = x_len.abs_diff(&s_sq);
    let corr = m.mul(&r, &d, len)?.half();
    Ok(match ord {
        Ordering::Greater => s.add(&corr),
        Ordering::Equal => s,
        Ordering::Less => s.abs_diff(&corr).0,
    })
}

/// `1/x` con `len` limbs. `x` debe ser positivo y menor que `BASE`.
pub fn inv(m: &mut Multiplier, x: &Fixed, len: usize) -> Result<Fixed> {
    let len = len.max(1);
    if len <= BASE_CASE_LIMBS {
        let approx = 1.0 / x.to_f64().max(f64::MIN_POSITIVE);
        return Ok(Fixed::from_f64(approx, len));
    }
    let half = len / 2 + 1;
    let y0 = inv(m, x, half)?.resized(len);
    // y1 = y0 + y0 · (1 − x·y0)
    let xy = m.mul(x, &y0, len)?;
    let one = Fixed::from_u32(1, len);
    let (e, ord) = one.abs_diff(&xy);
    let corr = m.mul(&y0, &e, len)?;
    Ok(match ord {
        Ordering::Greater => y0.add(&corr),
        Ordering::Equal => y0,
        Ordering::Less => y0.abs_diff(&corr).0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SQRT_HALF: &str = "70710678118654752440084436210484903928483593768847403658833986899536623923105351942519376716382078636750692311545614851246241802792536860632206074854996791570661133296375279637789997525057639103028573505477998580298513726729843100736425870932044459930477616461524215435716072541988130181399762570399484362669827316590441482031030762917619752737287514387998086491778761016";

    #[test]
    fn sqrt_of_half_matches_reference() {
        let mut m = Multiplier::new();
        for &len in &[2_usize, 3, 4, 5, 8, 17, 40, 90] {
            let half = Fixed::from_limbs(vec![0, 5000]).resized(len);
            let got = sqrt(&mut m, &half, len).expect("sqrt");
            let digits = 4 * (len - 1);
            let checked = digits.saturating_sub(8).min(SQRT_HALF.len());
            assert_eq!(
                &got.fraction_digits(checked),
                &SQRT_HALF[..checked],
                "len = {len}"
            );
        }
    }

    #[test]
    fn inverse_of_three() {
        let mut m = Multiplier::new();
        let three = Fixed::from_u32(3, 50);
        let got = inv(&mut m, &three, 50).expect("inv");
        assert_eq!(got.integer_part(), 0);
        assert_eq!(got.fraction_digits(180), "3".repeat(180));
    }

    #[test]
    fn inv_sqrt_of_two() {
        let mut m = Multiplier::new();
        let two = Fixed::from_u32(2, 30);
        let got = inv_sqrt(&mut m, &two, 30).expect("inv_sqrt");
        assert_eq!(got.fraction_digits(100), &SQRT_HALF[..100]);
    }
}
