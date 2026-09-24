//! Configuración explícita de un run: módulo, tamaño, modo e hilos.
//!
//! Todo lo que cambia la carga de trabajo se declara aquí y viaja dentro del
//! resultado, de modo que un fichero JSON describe por completo cómo se obtuvo.

use std::fmt;
use std::num::NonZeroUsize;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// Módulo de benchmark.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Module {
    /// Dígitos de Pi por Gauss–Legendre (reimplementación de SuperPi).
    Pi,
    /// Raíces cuadradas por Newton (reimplementación de wPrime).
    Radical,
    /// Criba segmentada de primos (propio de GugolPi).
    Zeta,
}

impl Module {
    /// Los tres módulos, en el orden de la spec.
    pub const ALL: [Module; 3] = [Module::Pi, Module::Radical, Module::Zeta];

    /// Nombre en minúsculas, el mismo que usa la CLI y el JSON.
    pub fn name(self) -> &'static str {
        match self {
            Module::Pi => "pi",
            Module::Radical => "radical",
            Module::Zeta => "zeta",
        }
    }

    /// Tamaños oficiales del módulo, en orden creciente.
    pub fn official_sizes(self) -> &'static [&'static str] {
        match self {
            Module::Pi => &[
                "16K", "32K", "64K", "128K", "256K", "512K", "1M", "2M", "4M", "8M", "16M", "32M",
            ],
            Module::Radical => &["32M", "1024M"],
            Module::Zeta => &["100M", "1G", "10G"],
        }
    }

    /// Tamaños extendidos: existen en GugolPi pero no en el original, no puntúan.
    pub fn extended_sizes(self) -> &'static [&'static str] {
        match self {
            Module::Pi => &["64M", "128M", "256M", "512M", "1G"],
            Module::Radical => &["128M", "4096M"],
            Module::Zeta => &["100G"],
        }
    }

    /// Interpreta un texto de tamaño según las unidades del módulo.
    ///
    /// - Pi: `K` = 1024 dígitos, `M` = 1 048 576, `G` = 2^30; sólo se admiten los tamaños listados.
    /// - Radical: `M` = 1 000 000; además se admite un entero libre (no oficial).
    /// - Zeta: `M` = 10^6, `G` = 10^9; además se admite un entero libre (no oficial).
    pub fn parse_size(self, text: &str) -> Result<Size> {
        let trimmed = text.trim();
        let unknown = || Error::UnknownSize {
            text: text.to_owned(),
            module: self,
        };
        let label = trimmed.to_ascii_uppercase();
        let listed = self.official_sizes().contains(&label.as_str())
            || self.extended_sizes().contains(&label.as_str());
        let official = self.official_sizes().contains(&label.as_str());

        let (kilo, mega, giga) = match self {
            Module::Pi => (1_024, 1_048_576, 1_073_741_824),
            Module::Radical | Module::Zeta => (1_000, 1_000_000, 1_000_000_000),
        };

        let value = if let Some(number) = label.strip_suffix('K') {
            parse_scaled(number, kilo)
        } else if let Some(number) = label.strip_suffix('M') {
            parse_scaled(number, mega)
        } else if let Some(number) = label.strip_suffix('G') {
            parse_scaled(number, giga)
        } else {
            label.parse::<u64>().ok()
        }
        .filter(|&v| v > 0)
        .ok_or_else(unknown)?;

        if self == Module::Pi && !listed {
            return Err(unknown());
        }
        let label = if listed { label } else { value.to_string() };
        Ok(Size {
            label,
            value,
            official,
        })
    }
}

fn parse_scaled(number: &str, unit: u64) -> Option<u64> {
    number.parse::<u64>().ok().and_then(|n| n.checked_mul(unit))
}

impl fmt::Display for Module {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl FromStr for Module {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "pi" => Ok(Module::Pi),
            "radical" => Ok(Module::Radical),
            "zeta" => Ok(Module::Zeta),
            other => Err(Error::InvalidConfig(format!(
                "módulo desconocido «{other}»"
            ))),
        }
    }
}

/// Tamaño de un run: etiqueta legible, valor numérico y si puntúa oficialmente.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Size {
    /// Etiqueta normalizada (`1M`, `32M`, `1G` o el entero libre).
    pub label: String,
    /// Valor numérico: dígitos (Pi) o N (Radical, Zeta).
    pub value: u64,
    /// `true` si es un tamaño oficial del módulo.
    pub official: bool,
}

impl fmt::Display for Size {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.label)
    }
}

/// Modo de ejecución.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// Un solo hilo: la cifra oficial de Pi y la cifra por núcleo de Radical y Zeta.
    #[default]
    Single,
    /// Varios hilos según [`Threads`].
    Multi,
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Mode::Single => "single",
            Mode::Multi => "multi",
        })
    }
}

impl FromStr for Mode {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "single" => Ok(Mode::Single),
            "multi" => Ok(Mode::Multi),
            other => Err(Error::InvalidConfig(format!("modo desconocido «{other}»"))),
        }
    }
}

/// Número de hilos pedido para el modo multi.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum Threads {
    /// Tantos hilos como CPU lógicas.
    #[default]
    Auto,
    /// Tantos hilos como núcleos físicos (sin SMT).
    Physical,
    /// Un número fijo.
    Count(NonZeroUsize),
}

impl Threads {
    /// Resuelve el número de hilos para una máquina concreta.
    pub fn resolve(self, logical: usize, physical: usize) -> usize {
        match self {
            Threads::Auto => logical.max(1),
            Threads::Physical => physical.max(1),
            Threads::Count(n) => n.get(),
        }
    }
}

impl fmt::Display for Threads {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Threads::Auto => f.write_str("auto"),
            Threads::Physical => f.write_str("physical"),
            Threads::Count(n) => write!(f, "{n}"),
        }
    }
}

impl FromStr for Threads {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "auto" => Ok(Threads::Auto),
            "physical" => Ok(Threads::Physical),
            other => other
                .parse::<NonZeroUsize>()
                .map(Threads::Count)
                .map_err(|_| Error::InvalidConfig(format!("hilos no válidos «{other}»"))),
        }
    }
}

impl TryFrom<String> for Threads {
    type Error = Error;

    fn try_from(value: String) -> Result<Self> {
        value.parse()
    }
}

impl From<Threads> for String {
    fn from(value: Threads) -> Self {
        value.to_string()
    }
}

/// Opciones específicas del módulo Radical.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct RadicalOptions {
    /// Reparto estático (rango contiguo por hilo) en vez de la cola dinámica.
    pub static_split: bool,
}

/// Configuración completa de un run.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunConfig {
    /// Módulo a ejecutar.
    pub module: Module,
    /// Tamaño del test.
    pub size: Size,
    /// Modo single o multi.
    #[serde(default)]
    pub mode: Mode,
    /// Hilos en modo multi (ignorado en single).
    #[serde(default)]
    pub threads: Threads,
    /// Fijar cada hilo a un núcleo.
    #[serde(default)]
    pub affinity: bool,
    /// Opciones del módulo Radical.
    #[serde(default, skip_serializing_if = "is_default")]
    pub radical: RadicalOptions,
}

fn is_default<T: Default + PartialEq>(value: &T) -> bool {
    *value == T::default()
}

impl RunConfig {
    /// Configuración por defecto de un módulo para un tamaño dado.
    pub fn new(module: Module, size: &str) -> Result<Self> {
        Ok(Self {
            module,
            size: module.parse_size(size)?,
            mode: Mode::Single,
            threads: Threads::Auto,
            affinity: false,
            radical: RadicalOptions::default(),
        })
    }

    /// Hilos efectivos para esta configuración en una máquina concreta.
    pub fn resolve_threads(&self, logical: usize, physical: usize) -> usize {
        match self.mode {
            Mode::Single => 1,
            Mode::Multi => self.threads.resolve(logical, physical),
        }
    }

    /// `true` si el run puntúa oficialmente: tamaño oficial y sin opciones que alteren la carga.
    pub fn is_official(&self) -> bool {
        self.size.official && self.radical == RadicalOptions::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pi_sizes_use_binary_units() {
        let size = Module::Pi
            .parse_size("1M")
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(size.value, 1_048_576);
        assert!(size.official);
        let size = Module::Pi
            .parse_size("16k")
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(size.value, 16_384);
        assert_eq!(size.label, "16K");
    }

    #[test]
    fn pi_rejects_unlisted_sizes() {
        assert!(Module::Pi.parse_size("3M").is_err());
        assert!(Module::Pi.parse_size("12345").is_err());
    }

    #[test]
    fn pi_extended_sizes_are_unofficial() {
        let size = Module::Pi
            .parse_size("1G")
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(size.value, 1 << 30);
        assert!(!size.official);
    }

    #[test]
    fn radical_sizes_use_decimal_units() {
        let size = Module::Radical
            .parse_size("32M")
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(size.value, 32_000_000);
        assert!(size.official);
        let size = Module::Radical
            .parse_size("1024M")
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(size.value, 1_024_000_000);
        assert!(size.official);
        let free = Module::Radical
            .parse_size("500000")
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(free.value, 500_000);
        assert_eq!(free.label, "500000");
        assert!(!free.official);
    }

    #[test]
    fn zeta_sizes() {
        let size = Module::Zeta
            .parse_size("1G")
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(size.value, 1_000_000_000);
        assert!(size.official);
        let size = Module::Zeta
            .parse_size("100G")
            .unwrap_or_else(|e| panic!("{e}"));
        assert!(!size.official);
    }

    #[test]
    fn zero_and_garbage_are_rejected() {
        assert!(Module::Radical.parse_size("0").is_err());
        assert!(Module::Zeta.parse_size("abc").is_err());
        assert!(Module::Radical.parse_size("").is_err());
    }

    #[test]
    fn threads_round_trip_through_serde() {
        let cfg = RunConfig {
            threads: Threads::Count(NonZeroUsize::MIN.saturating_add(7)),
            mode: Mode::Multi,
            ..RunConfig::new(Module::Radical, "32M").unwrap_or_else(|e| panic!("{e}"))
        };
        let json = serde_json::to_string(&cfg).unwrap_or_else(|e| panic!("{e}"));
        assert!(json.contains("\"threads\":\"8\""));
        let back: RunConfig = serde_json::from_str(&json).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(back, cfg);
    }

    #[test]
    fn single_mode_always_one_thread() {
        let mut cfg = RunConfig::new(Module::Radical, "32M").unwrap_or_else(|e| panic!("{e}"));
        cfg.threads = Threads::Auto;
        assert_eq!(cfg.resolve_threads(16, 8), 1);
        cfg.mode = Mode::Multi;
        assert_eq!(cfg.resolve_threads(16, 8), 16);
        cfg.threads = Threads::Physical;
        assert_eq!(cfg.resolve_threads(16, 8), 8);
    }
}
