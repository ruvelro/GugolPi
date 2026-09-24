//! Criba de Eratóstenes segmentada con rueda módulo 30.
//!
//! Cada byte del segmento representa 30 números consecutivos; el bit `j` es el número
//! `30·byte + WHEEL[j]` con `WHEEL = [1, 7, 11, 13, 17, 19, 23, 29]`, los residuos coprimos con
//! 2, 3 y 5. Los múltiplos de cada primo de criba avanzan con un ciclo de 8 incrementos de byte y
//! 8 máscaras precalculados por primo, de modo que el bucle interior no divide nunca.

use crate::score::ZETA_SEGMENT_BYTES;

/// Residuos módulo 30 coprimos con 2, 3 y 5.
pub const WHEEL: [u64; 8] = [1, 7, 11, 13, 17, 19, 23, 29];

/// Números cubiertos por un segmento.
pub const SEGMENT_NUMBERS: u64 = ZETA_SEGMENT_BYTES as u64 * 30;

/// Índice en [`WHEEL`] de cada residuo módulo 30 (255 si no es coprimo).
const WHEEL_INDEX: [u8; 30] = {
    let mut table = [255_u8; 30];
    let mut i = 0;
    while i < 8 {
        table[WHEEL[i] as usize] = i as u8;
        i += 1;
    }
    table
};

/// Totales de un tramo: recuento y suma de primos módulo 2^64.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SieveTotals {
    /// Primos encontrados.
    pub count: u64,
    /// Suma de esos primos módulo 2^64.
    pub sum_mod_2_64: u64,
}

impl SieveTotals {
    /// Suma de dos tramos disjuntos.
    pub fn merge(self, other: Self) -> Self {
        Self {
            count: self.count + other.count,
            sum_mod_2_64: self.sum_mod_2_64.wrapping_add(other.sum_mod_2_64),
        }
    }
}

/// Un primo de criba con su ciclo de 8 incrementos de byte y 8 máscaras.
#[derive(Clone, Debug)]
struct SievingPrime {
    prime: u64,
    /// Incremento de byte al pasar del multiplicador `WHEEL[j]` al siguiente.
    byte_step: [u32; 8],
    /// Máscara (bit a cero) del múltiplo con multiplicador `WHEEL[j]`.
    mask: [u8; 8],
}

impl SievingPrime {
    fn new(prime: u64) -> Self {
        let residue_index = WHEEL_INDEX[(prime % 30) as usize] as usize;
        let mut byte_step = [0_u32; 8];
        let mut mask = [0_u8; 8];
        for j in 0..8 {
            let product = prime * WHEEL[j];
            let next = prime * (WHEEL[(j + 1) % 8] + if j == 7 { 30 } else { 0 });
            byte_step[j] = ((next / 30) - (product / 30)) as u32;
            let bit = WHEEL_INDEX[(product % 30) as usize];
            mask[j] = !(1_u8 << bit);
        }
        debug_assert!(residue_index < 8);
        Self {
            prime,
            byte_step,
            mask,
        }
    }
}

/// Estado por hilo: posición del próximo múltiplo de cada primo dentro del tramo actual.
#[derive(Clone, Debug)]
pub struct Workspace {
    segment: Vec<u8>,
    /// Por primo: (byte del próximo múltiplo relativo al inicio del tramo, índice de rueda).
    cursors: Vec<(u64, u8)>,
}

/// Criba para un límite fijo: primos de criba hasta √N precalculados.
#[derive(Clone, Debug)]
pub struct Sieve {
    limit: u64,
    primes: Vec<SievingPrime>,
}

impl Sieve {
    /// Prepara la criba para contar primos en `[0, limit]`.
    pub fn new(limit: u64) -> Self {
        let root = (limit as f64).sqrt() as u64 + 1;
        let primes = small_primes(root)
            .into_iter()
            .filter(|&p| p >= 7)
            .map(SievingPrime::new)
            .collect();
        Self { limit, primes }
    }

    /// Límite superior (inclusive).
    pub fn limit(&self) -> u64 {
        self.limit
    }

    /// Espacio de trabajo para un hilo.
    pub fn workspace(&self) -> Workspace {
        Workspace {
            segment: vec![0xFF; ZETA_SEGMENT_BYTES],
            cursors: vec![(0, 0); self.primes.len()],
        }
    }

    /// Criba el tramo `[low, high]` (ambos inclusive, `low` múltiplo de 30 salvo el 0) y
    /// devuelve recuento y suma de los primos que contiene.
    pub fn sieve_range(&self, ws: &mut Workspace, low: u64, high: u64) -> SieveTotals {
        let mut totals = SieveTotals::default();
        if low == 0 {
            // 2, 3 y 5 no están en la rueda.
            for p in [2_u64, 3, 5] {
                if p <= high {
                    totals.count += 1;
                    totals.sum_mod_2_64 = totals.sum_mod_2_64.wrapping_add(p);
                }
            }
        }
        let low_byte = low / 30;
        let high_byte = high / 30;
        self.position_cursors(ws, low_byte);

        let mut byte = low_byte;
        while byte <= high_byte {
            let seg_len = ((high_byte - byte + 1).min(ZETA_SEGMENT_BYTES as u64)) as usize;
            let segment = &mut ws.segment[..seg_len];
            segment.fill(0xFF);
            for (prime, cursor) in self.primes.iter().zip(ws.cursors.iter_mut()) {
                let (mut pos, mut j) = (cursor.0, cursor.1 as usize);
                while pos < seg_len as u64 {
                    segment[pos as usize] &= prime.mask[j];
                    pos += u64::from(prime.byte_step[j]);
                    j = (j + 1) & 7;
                }
                *cursor = (pos - seg_len as u64, j as u8);
            }
            if byte == 0 {
                segment[0] &= !1; // el 1 no es primo
            }
            let last_number = high.min((byte + seg_len as u64) * 30 - 1);
            totals = totals.merge(count_segment(segment, byte, last_number));
            byte += seg_len as u64;
        }
        totals
    }

    /// Coloca cada primo en su primer múltiplo `≥ max(p², 30·low_byte)` con multiplicador en
    /// la rueda, expresado como byte relativo a `low_byte`.
    fn position_cursors(&self, ws: &mut Workspace, low_byte: u64) {
        let low = low_byte * 30;
        for (prime, cursor) in self.primes.iter().zip(ws.cursors.iter_mut()) {
            let p = prime.prime;
            let mut q = (low.div_ceil(p)).max(p);
            // Avanzar q hasta un residuo de la rueda.
            let mut j = WHEEL_INDEX[(q % 30) as usize];
            while j == 255 {
                q += 1;
                j = WHEEL_INDEX[(q % 30) as usize];
            }
            let multiple = p * q;
            *cursor = (multiple / 30 - low_byte, j);
        }
    }
}

/// Cuenta y suma los primos marcados en `segment`, cuyo primer byte representa `30·first_byte`,
/// ignorando los números mayores que `last_number`.
fn count_segment(segment: &[u8], first_byte: u64, last_number: u64) -> SieveTotals {
    let mut totals = SieveTotals::default();
    for (i, &byte) in segment.iter().enumerate() {
        let base = (first_byte + i as u64) * 30;
        if base + 29 <= last_number {
            totals.count += u64::from(byte.count_ones());
            let mut bits = byte;
            while bits != 0 {
                let j = bits.trailing_zeros() as usize;
                totals.sum_mod_2_64 = totals.sum_mod_2_64.wrapping_add(base + WHEEL[j]);
                bits &= bits - 1;
            }
        } else {
            let mut bits = byte;
            while bits != 0 {
                let j = bits.trailing_zeros() as usize;
                let n = base + WHEEL[j];
                if n <= last_number {
                    totals.count += 1;
                    totals.sum_mod_2_64 = totals.sum_mod_2_64.wrapping_add(n);
                }
                bits &= bits - 1;
            }
        }
    }
    totals
}

/// Primos hasta `limit` (inclusive) con una criba simple.
fn small_primes(limit: u64) -> Vec<u64> {
    let limit = limit as usize;
    let mut is_prime = vec![true; limit + 1];
    let mut primes = Vec::new();
    for i in 2..=limit {
        if is_prime[i] {
            primes.push(i as u64);
            let mut j = i * i;
            while j <= limit {
                is_prime[j] = false;
                j += i;
            }
        }
    }
    primes
}

/// Cuenta y suma los primos hasta `limit` en un solo hilo (útil para tests y tamaños libres).
pub fn count_primes(limit: u64) -> SieveTotals {
    let sieve = Sieve::new(limit);
    let mut ws = sieve.workspace();
    sieve.sieve_range(&mut ws, 0, limit)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn naive(limit: u64) -> SieveTotals {
        let primes = small_primes(limit);
        SieveTotals {
            count: primes.len() as u64,
            sum_mod_2_64: primes.iter().fold(0_u64, |acc, &p| acc.wrapping_add(p)),
        }
    }

    #[test]
    fn matches_naive_sieve_at_awkward_limits() {
        for limit in [
            0_u64, 1, 2, 3, 5, 6, 7, 29, 30, 31, 100, 997, 1_000, 30_000, 65_537, 1_000_000,
            2_000_003,
        ] {
            assert_eq!(count_primes(limit), naive(limit), "limit = {limit}");
        }
    }

    #[test]
    fn known_pi_values() {
        assert_eq!(count_primes(1_000).count, 168);
        assert_eq!(count_primes(100_000).count, 9_592);
        assert_eq!(count_primes(100_000).sum_mod_2_64, 454_396_537);
        assert_eq!(count_primes(10_000_000).count, 664_579);
    }

    #[test]
    fn ranges_compose() {
        // Los tramos empiezan en múltiplos de 30 y el estado de los cursores se recoloca en
        // cada tramo, así que cualquier partición da el mismo total.
        let sieve = Sieve::new(3_000_000);
        let mut ws = sieve.workspace();
        let split = 999_990;
        let a = sieve.sieve_range(&mut ws, 0, split - 1);
        let b = sieve.sieve_range(&mut ws, split, 3_000_000);
        assert_eq!(a.merge(b), naive(3_000_000));
        let c = sieve.sieve_range(&mut ws, 2_999_970, 3_000_000);
        let d = sieve.sieve_range(&mut ws, 0, 2_999_969);
        assert_eq!(c.merge(d), naive(3_000_000));
    }
}
