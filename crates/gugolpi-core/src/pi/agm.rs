//! El algoritmo de Gauss–Legendre (media aritmético-geométrica) para Pi.

use std::time::Instant;

use crate::benchmark::CancelToken;
use crate::bignum::{Fixed, Multiplier, newton};
use crate::error::{Error, Result};
use crate::progress::{Progress, ProgressEvent};
use crate::result::LoopTiming;
use crate::score::{PI_GUARD_LIMBS, pi_loops};

/// Resultado del cálculo de Pi.
#[derive(Clone, Debug)]
pub struct PiComputation {
    /// Los `digits` primeros decimales (sin el `3.`).
    pub decimals: String,
    /// Tiempo de cada loop.
    pub loops: Vec<LoopTiming>,
    /// Segundos de cálculo (sin la conversión a texto).
    pub compute_seconds: f64,
    /// Mayor error de redondeo de la FFT en todo el cálculo.
    pub max_fft_error: f64,
}

/// Calcula `digits` decimales de Pi. Emite un [`ProgressEvent::Loop`] por iteración.
pub fn compute(digits: u64, progress: &Progress, cancel: &CancelToken) -> Result<PiComputation> {
    let digits_usize = usize::try_from(digits)
        .map_err(|_| Error::InvalidConfig("tamaño de Pi fuera de rango".to_owned()))?;
    let len = digits_usize.div_ceil(4) + 1 + PI_GUARD_LIMBS;
    let loops = pi_loops(digits);
    let mut m = Multiplier::new();
    let start = Instant::now();

    let mut a = Fixed::from_u32(1, len);
    let mut b = newton::sqrt(&mut m, &Fixed::from_limbs(vec![0, 5000]).resized(len), len)?;
    let mut t = Fixed::from_limbs(vec![0, 2500]).resized(len);
    let mut p: u32 = 1;
    let mut loop_timings = Vec::with_capacity(loops as usize);
    let mut cumulative = 0.0;

    for index in 1..=loops {
        if cancel.is_cancelled() {
            return Err(Error::Cancelled);
        }
        let loop_start = Instant::now();
        let a_next = a.add(&b).half();
        let ab = m.mul(&a, &b, len)?;
        let b_next = newton::sqrt(&mut m, &ab, len)?;
        let (d, _) = a.abs_diff(&a_next);
        let d_sq = m.square(&d, len)?;
        t = t.abs_diff(&d_sq.mul_small(p)).0;
        p = p.saturating_mul(2);
        a = a_next;
        b = b_next;
        let seconds = loop_start.elapsed().as_secs_f64();
        cumulative += seconds;
        loop_timings.push(LoopTiming {
            index,
            seconds,
            cumulative_seconds: cumulative,
        });
        progress.send(ProgressEvent::Loop {
            index,
            seconds,
            cumulative_seconds: cumulative,
        });
    }

    // π ≈ (a + b)² / (4t)
    let sum = a.add(&b);
    let numerator = m.square(&sum, len)?;
    let denominator = t.mul_small(4);
    let inverse = newton::inv(&mut m, &denominator, len)?;
    let pi = m.mul(&numerator, &inverse, len)?;
    let compute_seconds = start.elapsed().as_secs_f64();
    if pi.integer_part() != 3 {
        return Err(Error::Verification(format!(
            "la parte entera de Pi salió {} en vez de 3",
            pi.integer_part()
        )));
    }
    Ok(PiComputation {
        decimals: pi.fraction_digits(digits_usize),
        loops: loop_timings,
        compute_seconds,
        max_fft_error: m.max_error(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const PI_100: &str = "1415926535897932384626433832795028841971693993751058209749445923078164062862089986280348253421170679";

    #[test]
    fn first_digits_are_right_for_small_sizes() {
        for &digits in &[16_u64, 100, 1000, 4096] {
            let got = compute(digits, &Progress::none(), &CancelToken::new()).expect("pi");
            let checked = (digits as usize).min(PI_100.len());
            assert_eq!(
                &got.decimals[..checked],
                &PI_100[..checked],
                "digits = {digits}"
            );
            assert_eq!(got.decimals.len(), digits as usize);
            assert!(got.max_fft_error < 0.1);
        }
    }

    #[test]
    fn loop_count_matches_superpi() {
        assert_eq!(pi_loops(1 << 20), 19);
        assert_eq!(pi_loops(1 << 25), 24);
        assert_eq!(pi_loops(1 << 14), 13);
    }
}
