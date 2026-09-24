//! FFT compleja iterativa (Cooley–Tukey radix-2, decimación en tiempo) con tabla de twiddles.
//!
//! El plan se construye una vez por tamaño y se reutiliza; la transformada es in situ.

use super::complex::Complex;

/// Plan de FFT para un tamaño potencia de dos.
#[derive(Clone, Debug)]
pub struct FftPlan {
    n: usize,
    /// Twiddles por etapa, contiguos: la etapa de longitud `L` ocupa `[L/2 - 1, L - 1)` y su
    /// entrada `k` es `exp(-2πi k / L)`. En total `n - 1` entradas. Así cada etapa lee su tabla
    /// secuencialmente, también cuando se transforma un tamaño menor que `n`.
    twiddles: Vec<Complex>,
}

impl FftPlan {
    /// Crea un plan para `n` puntos. `n` debe ser potencia de dos y ≥ 2; si no, se redondea
    /// hacia arriba a la siguiente potencia de dos (mínimo 2).
    pub fn new(n: usize) -> Self {
        let n = n.max(2).next_power_of_two();
        let mut twiddles = Vec::with_capacity(n - 1);
        let mut len = 2;
        while len <= n {
            for k in 0..len / 2 {
                // Ángulo calculado en f64 directamente sobre el índice: error ≈ 1 ulp.
                let angle = -2.0 * std::f64::consts::PI * (k as f64) / (len as f64);
                let (sin, cos) = angle.sin_cos();
                twiddles.push(Complex::new(cos, sin));
            }
            len <<= 1;
        }
        Self { n, twiddles }
    }

    /// Número de puntos.
    pub fn len(&self) -> usize {
        self.n
    }

    /// `true` nunca: un plan tiene al menos 2 puntos. Existe por simetría con `len`.
    pub fn is_empty(&self) -> bool {
        false
    }

    /// Transformada directa in situ de `data`, cuya longitud debe ser una potencia de dos no
    /// mayor que `self.len()`: un plan sirve para todos los tamaños menores. Con una longitud
    /// que no sea potencia de dos sólo se transforma el mayor prefijo que lo sea.
    pub fn forward(&self, data: &mut [Complex]) {
        let n = prefix_power_of_two(data.len()).min(self.n);
        let data = &mut data[..n];
        bit_reverse_permute(data);

        // Etapa 1 (len = 2): twiddle = 1.
        for pair in data.as_chunks_mut::<2>().0 {
            let (u, v) = (pair[0], pair[1]);
            pair[0] = u + v;
            pair[1] = u - v;
        }
        // Etapa 2 (len = 4): twiddles 1 y -i.
        if n >= 4 {
            for quad in data.as_chunks_mut::<4>().0 {
                let (a, b, c, d) = (quad[0], quad[1], quad[2], quad[3]);
                let d_rot = d.mul_neg_i();
                quad[0] = a + c;
                quad[2] = a - c;
                quad[1] = b + d_rot;
                quad[3] = b - d_rot;
            }
        }
        // Etapas restantes, cada una con su tabla contigua.
        let mut len = 8;
        while len <= n {
            let half = len / 2;
            let table = &self.twiddles[half - 1..len - 1];
            for block in data.chunks_exact_mut(len) {
                let (lo, hi) = block.split_at_mut(half);
                for ((l, h), &w) in lo.iter_mut().zip(hi.iter_mut()).zip(table) {
                    let u = *l;
                    let v = *h * w;
                    *l = u + v;
                    *h = u - v;
                }
            }
            len <<= 1;
        }
    }

    /// Transformada inversa in situ, con el factor `1/n` incluido.
    pub fn inverse(&self, data: &mut [Complex]) {
        for x in data.iter_mut() {
            *x = x.conj();
        }
        self.forward(data);
        let scale = 1.0 / (data.len() as f64);
        for x in data.iter_mut() {
            *x = x.conj().scale(scale);
        }
    }
}

/// Mayor potencia de dos que no supera `len` (0 para 0).
fn prefix_power_of_two(len: usize) -> usize {
    if len == 0 {
        0
    } else {
        1 << (usize::BITS - 1 - len.leading_zeros())
    }
}

/// Permutación por inversión de bits, in situ.
fn bit_reverse_permute(data: &mut [Complex]) {
    let n = data.len();
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            data.swap(i, j);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn naive_dft(input: &[Complex]) -> Vec<Complex> {
        let n = input.len();
        (0..n)
            .map(|k| {
                let mut acc = Complex::default();
                for (j, x) in input.iter().enumerate() {
                    let angle = -2.0 * std::f64::consts::PI * ((j * k) as f64) / (n as f64);
                    let (s, c) = angle.sin_cos();
                    acc = acc + *x * Complex::new(c, s);
                }
                acc
            })
            .collect()
    }

    fn pseudo_random(n: usize, seed: u64) -> Vec<Complex> {
        let mut state = seed;
        (0..n)
            .map(|_| {
                state = state
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1);
                let re = ((state >> 33) % 10_000) as f64;
                state = state
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1);
                let im = ((state >> 33) % 10_000) as f64;
                Complex::new(re, im)
            })
            .collect()
    }

    #[test]
    fn matches_naive_dft() {
        for &n in &[2_usize, 4, 8, 16, 64, 256] {
            let input = pseudo_random(n, 42 + n as u64);
            let expected = naive_dft(&input);
            let plan = FftPlan::new(n);
            let mut data = input.clone();
            plan.forward(&mut data);
            for (got, want) in data.iter().zip(&expected) {
                let err = ((got.re - want.re).powi(2) + (got.im - want.im).powi(2)).sqrt();
                assert!(err < 1e-6 * (n as f64), "n = {n}: {got:?} vs {want:?}");
            }
        }
    }

    #[test]
    fn inverse_round_trips() {
        let n = 1024;
        let input = pseudo_random(n, 7);
        let plan = FftPlan::new(n);
        let mut data = input.clone();
        plan.forward(&mut data);
        plan.inverse(&mut data);
        for (got, want) in data.iter().zip(&input) {
            assert!((got.re - want.re).abs() < 1e-7 && (got.im - want.im).abs() < 1e-7);
        }
    }

    #[test]
    fn larger_plan_transforms_smaller_inputs() {
        let big = FftPlan::new(256);
        for &n in &[2_usize, 16, 64, 256] {
            let input = pseudo_random(n, 99);
            let mut with_big = input.clone();
            big.forward(&mut with_big);
            let mut with_own = input.clone();
            FftPlan::new(n).forward(&mut with_own);
            for (a, b) in with_big.iter().zip(&with_own) {
                assert!(
                    (a.re - b.re).abs() < 1e-9 && (a.im - b.im).abs() < 1e-9,
                    "n = {n}"
                );
            }
        }
        assert_eq!(prefix_power_of_two(0), 0);
        assert_eq!(prefix_power_of_two(1), 1);
        assert_eq!(prefix_power_of_two(1023), 512);
    }

    #[test]
    fn plan_rounds_up_to_power_of_two() {
        assert_eq!(FftPlan::new(1).len(), 2);
        assert_eq!(FftPlan::new(5).len(), 8);
        assert_eq!(FftPlan::new(1024).len(), 1024);
    }
}
