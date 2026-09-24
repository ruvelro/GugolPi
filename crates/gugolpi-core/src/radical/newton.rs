//! Newton–Raphson para la raíz cuadrada, paso a paso como wPrime.

use crate::score::{RADICAL_REFINE_ITERATIONS, RADICAL_TOLERANCE_EXPONENT};

/// Raíz cuadrada de `k` por Newton, replicando el criterio de parada de wPrime.
///
/// Arranca en `k / 2` e itera hasta que el signo del paso `(x² − k) / (2x)` cambia respecto
/// al paso anterior; después aplica [`RADICAL_REFINE_ITERATIONS`] iteraciones más. No usa
/// la instrucción `sqrt`.
///
/// En coma flotante el paso puede quedarse positivo y menor que medio ulp de `x`, con lo que
/// `x` deja de cambiar y el signo nunca se invierte; por eso el bucle también termina cuando
/// el paso es cero o `x` no cambia. Sin esa salida el cálculo no acabaría nunca.
#[inline]
// Las comparaciones exactas son el criterio de parada: paso nulo o `x` sin cambio.
#[allow(clippy::float_cmp)]
pub fn newton_sqrt(k: f64) -> f64 {
    let mut x = k * 0.5;
    let mut previous_positive: Option<bool> = None;
    loop {
        let step = (x * x - k) / (2.0 * x);
        let next = x - step;
        if step == 0.0 || next == x {
            break;
        }
        let positive = step > 0.0;
        x = next;
        if previous_positive.is_some_and(|prev| prev != positive) {
            break;
        }
        previous_positive = Some(positive);
    }
    for _ in 0..RADICAL_REFINE_ITERATIONS {
        x -= (x * x - k) / (2.0 * x);
    }
    x
}

/// Calcula la raíz y comprueba `|x² − k| ≤ k · 2^RADICAL_TOLERANCE_EXPONENT`.
#[inline]
pub(crate) fn sqrt_and_check(k: f64) -> (f64, bool) {
    let x = newton_sqrt(k);
    let tolerance = k * 2_f64.powi(RADICAL_TOLERANCE_EXPONENT);
    (x, (x * x - k).abs() <= tolerance)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_values_are_exact() {
        for k in 1..=10_000_u64 {
            let (x, ok) = sqrt_and_check(k as f64);
            assert!(ok, "k = {k} falla la tolerancia");
            let reference = (k as f64).sqrt();
            assert!(
                (x - reference).abs() <= reference * 1e-15,
                "k = {k}: {x} vs {reference}"
            );
        }
    }

    #[test]
    fn perfect_squares_are_exact() {
        for r in 1..=5_000_u64 {
            let k = (r * r) as f64;
            assert_eq!(newton_sqrt(k), r as f64, "raíz de {k}");
        }
    }

    #[test]
    fn every_value_up_to_a_million_terminates_and_verifies() {
        // Cubre los k cuyo paso de Newton se queda por debajo de medio ulp (antes colgaba).
        let (errors, _) = (1..=1_000_000_u64).fold((0_u64, 0_u64), |(errors, sum), k| {
            let (x, ok) = sqrt_and_check(k as f64);
            (errors + u64::from(!ok), sum.wrapping_add(x.to_bits()))
        });
        assert_eq!(errors, 0);
    }

    #[test]
    fn large_values_pass_tolerance() {
        for k in [
            32_000_000_u64,
            1_024_000_000,
            4_096_000_000,
            u64::from(u32::MAX),
        ] {
            let (_, ok) = sqrt_and_check(k as f64);
            assert!(ok, "k = {k}");
        }
    }
}
