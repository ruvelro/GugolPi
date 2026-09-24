//! Resultado de un run: serializable, autocontenido y sellado con un hash de integridad.

use std::fmt::Write as _;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::config::{Module, RunConfig};
use crate::score::SCORE_VERSION;
use crate::sysinfo::SystemInfo;

/// Versión del esquema del fichero de resultado. Sube sólo con cambios incompatibles.
pub const SCHEMA_VERSION: u32 = 1;

/// Resultado completo de un run.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RunResult {
    /// Versión del esquema de este fichero.
    pub schema_version: u32,
    /// Qué build produjo el resultado.
    pub tool: ToolInfo,
    /// Máquina en la que corrió.
    pub system: SystemInfo,
    /// Qué se ejecutó.
    pub test: TestInfo,
    /// Tiempos medidos.
    pub timing: Timing,
    /// Verificación matemática.
    pub verification: Verification,
    /// `true` si el run puntúa oficialmente (tamaño oficial, sin opciones que alteren la carga,
    /// verificación correcta).
    pub official: bool,
    /// SHA-256 del JSON canónico (todo menos este campo). `None` hasta que se sella.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub integrity: Option<String>,
}

impl RunResult {
    /// Calcula y guarda el hash de integridad.
    pub fn seal(&mut self) {
        self.integrity = None;
        self.integrity = Some(self.canonical_digest());
    }

    /// `true` si el hash guardado coincide con el contenido.
    pub fn integrity_ok(&self) -> bool {
        match &self.integrity {
            None => false,
            Some(stored) => {
                let mut unsealed = self.clone();
                unsealed.integrity = None;
                *stored == unsealed.canonical_digest()
            }
        }
    }

    fn canonical_digest(&self) -> String {
        // serde_json serializa los campos en orden de declaración, sin espacios: eso es el
        // formato canónico. Un fallo de serialización es imposible para estos tipos.
        let bytes = serde_json::to_vec(self).unwrap_or_default();
        let digest = Sha256::digest(&bytes);
        digest
            .iter()
            .fold(String::with_capacity(64), |mut acc, byte| {
                let _ = write!(acc, "{byte:02x}");
                acc
            })
    }
}

/// Build que produjo el resultado.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolInfo {
    /// Siempre `gugolpi`.
    pub name: String,
    /// Versión semántica del crate.
    pub version: String,
    /// Versión de puntuación ([`SCORE_VERSION`]).
    pub score_version: u32,
    /// Commit de git de la build, o `unknown`.
    pub commit: String,
    /// Nivel SIMD detectado en tiempo de ejecución.
    pub simd_level: String,
}

impl ToolInfo {
    /// Información de esta build.
    pub fn current() -> Self {
        Self {
            name: "gugolpi".to_owned(),
            version: env!("CARGO_PKG_VERSION").to_owned(),
            score_version: SCORE_VERSION,
            commit: env!("GUGOLPI_COMMIT").to_owned(),
            simd_level: crate::sysinfo::simd_level().to_owned(),
        }
    }
}

/// Qué se ejecutó.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestInfo {
    /// Configuración completa del run.
    #[serde(flatten)]
    pub config: RunConfig,
    /// Hilos realmente usados.
    pub threads_used: usize,
}

/// Tiempos medidos. Sólo se cronometra el cálculo, nunca la E/S.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Timing {
    /// Instante de inicio en UTC, RFC 3339.
    pub started_utc: String,
    /// Tiempo total de cálculo en segundos.
    pub total_seconds: f64,
    /// Tiempo por loop (sólo Pi), en segundos.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub loops: Vec<LoopTiming>,
    /// Tiempo de trabajo de cada hilo, en segundos.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub per_thread_seconds: Vec<f64>,
    /// Unidades procesadas por segundo.
    pub throughput: f64,
    /// Unidad del throughput (`digits/s`, `numbers/s`).
    pub throughput_unit: String,
}

impl Timing {
    /// Tiempo del hilo más lento, el más rápido y la media, si hay datos por hilo.
    pub fn thread_spread(&self) -> Option<(f64, f64, f64)> {
        if self.per_thread_seconds.is_empty() {
            return None;
        }
        let max = self
            .per_thread_seconds
            .iter()
            .copied()
            .fold(f64::MIN, f64::max);
        let min = self
            .per_thread_seconds
            .iter()
            .copied()
            .fold(f64::MAX, f64::min);
        let mean =
            self.per_thread_seconds.iter().sum::<f64>() / self.per_thread_seconds.len() as f64;
        Some((min, max, mean))
    }
}

/// Un loop de Pi, como la línea "Loop n" de SuperPi.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LoopTiming {
    /// Índice del loop, empezando en 1.
    pub index: u32,
    /// Duración del loop en segundos.
    pub seconds: f64,
    /// Tiempo acumulado en segundos.
    pub cumulative_seconds: f64,
}

/// Estado de la verificación matemática.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VerificationStatus {
    /// El resultado coincide con la referencia.
    Passed,
    /// El resultado no coincide: CPU inestable o bug.
    Failed,
}

/// Verificación matemática del run.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Verification {
    /// Estado global.
    pub status: VerificationStatus,
    /// Valores que fallaron (Radical) o discrepancias contadas.
    pub errors: u64,
    /// Hash de dígitos (Pi), checksum (Radical, Zeta) en hexadecimal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub digest: Option<String>,
    /// Error máximo de redondeo de la FFT (sólo Pi).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_fft_error: Option<f64>,
    /// Explicación legible cuando falla.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl Verification {
    /// `true` si la verificación pasó.
    pub fn passed(&self) -> bool {
        self.status == VerificationStatus::Passed
    }
}

/// Construye un resultado a partir de sus partes, calcula `official` y lo sella.
pub(crate) fn build_sealed(
    config: &RunConfig,
    system: &SystemInfo,
    threads_used: usize,
    timing: Timing,
    verification: Verification,
) -> RunResult {
    let official = config.is_official() && verification.passed();
    let mut result = RunResult {
        schema_version: SCHEMA_VERSION,
        tool: ToolInfo::current(),
        system: system.clone(),
        test: TestInfo {
            config: config.clone(),
            threads_used,
        },
        timing,
        verification,
        official,
        integrity: None,
    };
    result.seal();
    result
}

/// Unidad de throughput de cada módulo.
pub(crate) fn throughput_unit(module: Module) -> &'static str {
    match module {
        Module::Pi => "digits/s",
        Module::Radical | Module::Zeta => "numbers/s",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Module;

    fn sample() -> RunResult {
        let config = RunConfig::new(Module::Radical, "32M").expect("size");
        let timing = Timing {
            started_utc: "2026-09-24T00:00:00Z".to_owned(),
            total_seconds: 1.5,
            loops: Vec::new(),
            per_thread_seconds: vec![1.4, 1.5],
            throughput: 1.0,
            throughput_unit: "numbers/s".to_owned(),
        };
        let verification = Verification {
            status: VerificationStatus::Passed,
            errors: 0,
            digest: Some("00".to_owned()),
            max_fft_error: None,
            detail: None,
        };
        build_sealed(&config, &SystemInfo::placeholder(), 2, timing, verification)
    }

    #[test]
    fn sealed_result_verifies_and_round_trips() {
        let result = sample();
        assert!(result.integrity_ok());
        let json = serde_json::to_string_pretty(&result).expect("json");
        let back: RunResult = serde_json::from_str(&json).expect("parse");
        assert_eq!(back, result);
        assert!(back.integrity_ok());
    }

    #[test]
    fn tampering_breaks_integrity() {
        let mut result = sample();
        result.timing.total_seconds = 0.1;
        assert!(!result.integrity_ok());
    }

    #[test]
    fn official_requires_passing_verification() {
        let mut result = sample();
        assert!(result.official);
        result.verification.status = VerificationStatus::Failed;
        let config = result.test.config.clone();
        let rebuilt = build_sealed(
            &config,
            &result.system,
            2,
            result.timing.clone(),
            result.verification.clone(),
        );
        assert!(!rebuilt.official);
    }

    #[test]
    fn thread_spread() {
        let result = sample();
        let (min, max, mean) = result.timing.thread_spread().expect("spread");
        assert!((min - 1.4).abs() < 1e-12);
        assert!((max - 1.5).abs() < 1e-12);
        assert!((mean - 1.45).abs() < 1e-12);
    }
}
