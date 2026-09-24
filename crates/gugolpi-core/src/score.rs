//! Constantes que afectan a la puntuación.
//!
//! Cualquier cambio en este módulo que mueva los tiempos exige subir
//! [`SCORE_VERSION`] y anotarlo en el changelog. Los resultados sólo son
//! comparables entre sí cuando comparten `score_version`.

/// Versión de puntuación. Sube cuando cambia la carga de trabajo de cualquier módulo.
pub const SCORE_VERSION: u32 = 1;

/// Módulo Radical: iteraciones de Newton adicionales tras el cambio de signo.
pub const RADICAL_REFINE_ITERATIONS: u32 = 4;

/// Módulo Radical: tolerancia relativa de la verificación `x² = k`, como exponente de 2.
pub const RADICAL_TOLERANCE_EXPONENT: i32 = -50;

/// Módulo Radical: números por bloque de trabajo en el reparto dinámico.
pub const RADICAL_CHUNK: u64 = 1_000_000;

/// Módulo Pi: limbs de guarda (4 dígitos cada uno) añadidos a la precisión pedida.
pub const PI_GUARD_LIMBS: usize = 8;

/// Módulo Pi: error de redondeo máximo admitido en un coeficiente de la FFT. Por encima, la
/// multiplicación no es fiable y el run se invalida.
pub const FFT_ROUNDING_LIMIT: f64 = 0.25;

/// Módulo Pi: número de iteraciones de Gauss–Legendre para `digits` decimales.
///
/// Tras `k` iteraciones el error es ≈ 10^(−π·2^(k+1)/ln 10), luego basta con
/// `k ≥ log2(digits) − 1,448`. Da 19 loops para 1M y 24 para 32M, como SuperPi.
pub fn pi_loops(digits: u64) -> u32 {
    let digits = digits.max(4) as f64;
    (digits.log2() - 1.448).ceil().max(1.0) as u32
}

/// Módulo Zeta: bytes por segmento de criba (cada byte cubre 30 números). 32 KB caben en la
/// caché L1 de datos de cualquier CPU actual.
pub const ZETA_SEGMENT_BYTES: usize = 32 * 1024;

/// Módulo Zeta: segmentos consecutivos que toma cada hilo de la cola dinámica en cada turno.
pub const ZETA_CHUNK_SEGMENTS: u64 = 16;
