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
