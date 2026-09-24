//! Suites: listas de tests en TOML, presets embebidos y expansión a jobs concretos.
//!
//! La CLI y la GUI usan este módulo para que una suite signifique lo mismo en las dos.

use std::num::NonZeroUsize;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::config::{Mode, Module, PiOptions, RadicalOptions, RunConfig, Threads};
use crate::error::{Error, Result};
use crate::sysinfo::SystemInfo;

/// Presets embebidos en el binario: `(nombre, TOML)`.
pub const PRESETS: &[(&str, &str)] = &[
    ("classic", include_str!("../../../suites/classic.toml")),
    ("trio", include_str!("../../../suites/trio.toml")),
    ("full", include_str!("../../../suites/full.toml")),
];

/// Un run planificado: etiqueta legible y configuración completa.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Job {
    /// Etiqueta (`pi 1M single`, `radical 32M multi ×8`).
    pub label: String,
    /// Configuración completa.
    pub config: RunConfig,
}

impl Job {
    /// Etiqueta canónica de una configuración.
    pub fn label_for(config: &RunConfig) -> String {
        match config.mode {
            Mode::Single => format!("{} {} single", config.module, config.size),
            Mode::Multi => format!(
                "{} {} multi ×{}",
                config.module, config.size, config.threads
            ),
        }
    }

    /// Un job a partir de una configuración.
    pub fn from_config(config: RunConfig) -> Self {
        Self {
            label: Self::label_for(&config),
            config,
        }
    }
}

/// Configuraciones del barrido de escalado: 1, 2, 4 … hilos hasta las CPU lógicas (incluidas).
pub fn scaling_jobs(base: &RunConfig, system: &SystemInfo) -> Vec<Job> {
    let max = system.logical_cpus.max(1);
    let mut counts: Vec<usize> = std::iter::successors(Some(1_usize), |&n| Some(n * 2))
        .take_while(|&n| n < max)
        .collect();
    counts.push(max);
    counts
        .into_iter()
        .filter_map(NonZeroUsize::new)
        .map(|n| {
            let mut config = base.clone();
            config.mode = Mode::Multi;
            config.threads = Threads::Count(n);
            Job::from_config(config)
        })
        .collect()
}

/// Fichero de suite.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SuiteFile {
    /// Cabecera.
    pub suite: SuiteMeta,
    /// Tests, en orden.
    #[serde(default, rename = "test")]
    pub tests: Vec<SuiteTest>,
}

/// Cabecera `[suite]`.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SuiteMeta {
    /// Nombre.
    pub name: String,
    /// Repeticiones por defecto de cada test.
    #[serde(default = "one")]
    pub repeat: u32,
}

/// Una entrada `[[test]]`.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SuiteTest {
    /// `pi`, `radical` o `zeta`.
    pub module: String,
    /// Tamaño con las unidades del módulo.
    pub size: String,
    /// `single` (defecto) o `multi`.
    #[serde(default)]
    pub mode: Option<String>,
    /// `auto`, `physical` o un número.
    #[serde(default)]
    pub threads: Option<String>,
    /// Fijar hilos a núcleos.
    #[serde(default)]
    pub affinity: bool,
    /// Radical: reparto estático.
    #[serde(default, rename = "static")]
    pub static_split: bool,
    /// Barrido de escalado en vez de un run.
    #[serde(default)]
    pub scaling: bool,
    /// Etiqueta opcional para el resumen.
    #[serde(default)]
    pub label: Option<String>,
}

fn one() -> u32 {
    1
}

impl SuiteFile {
    /// Interpreta un TOML.
    pub fn parse(text: &str) -> Result<Self> {
        let suite: Self = toml::from_str(text)
            .map_err(|e| Error::InvalidConfig(format!("la suite no es un TOML válido: {e}")))?;
        if suite.tests.is_empty() {
            return Err(Error::InvalidConfig(format!(
                "la suite «{}» no tiene ningún [[test]]",
                suite.suite.name
            )));
        }
        Ok(suite)
    }

    /// Un preset embebido, por nombre.
    pub fn preset(name: &str) -> Option<Result<Self>> {
        PRESETS
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, text)| Self::parse(text))
    }

    /// Carga un preset por nombre o un fichero por ruta.
    pub fn load(name_or_path: &str) -> Result<Self> {
        if let Some(preset) = Self::preset(name_or_path) {
            return preset;
        }
        let path = Path::new(name_or_path);
        if path.exists() {
            let text = std::fs::read_to_string(path).map_err(|source| Error::Io {
                path: path.display().to_string(),
                source,
            })?;
            return Self::parse(&text);
        }
        let names: Vec<&str> = PRESETS.iter().map(|(n, _)| *n).collect();
        Err(Error::InvalidConfig(format!(
            "«{name_or_path}» no es un preset ({}) ni un fichero existente",
            names.join(", ")
        )))
    }

    /// Expande los tests a jobs concretos para esta máquina.
    pub fn jobs(&self, system: &SystemInfo) -> Result<Vec<Job>> {
        let mut jobs = Vec::new();
        for (index, test) in self.tests.iter().enumerate() {
            let position = index + 1;
            let context = |what: &str, e: Error| {
                Error::InvalidConfig(format!("test {position}: {what}: {e}"))
            };
            let module: Module = test.module.parse().map_err(|e| context("módulo", e))?;
            let mut config =
                RunConfig::new(module, &test.size).map_err(|e| context("tamaño", e))?;
            config.mode = match &test.mode {
                Some(mode) => mode.parse::<Mode>().map_err(|e| context("modo", e))?,
                None => Mode::Single,
            };
            config.threads = match &test.threads {
                Some(t) => t.parse::<Threads>().map_err(|e| context("hilos", e))?,
                None => Threads::Auto,
            };
            config.affinity = test.affinity;
            config.radical = RadicalOptions {
                static_split: test.static_split,
            };
            config.pi = PiOptions::default();
            if test.scaling {
                jobs.extend(scaling_jobs(&config, system));
            } else {
                let mut job = Job::from_config(config);
                if let Some(label) = &test.label {
                    job.label.clone_from(label);
                }
                jobs.push(job);
            }
        }
        Ok(jobs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_parse_and_expand() {
        let system = SystemInfo::placeholder();
        for (name, text) in PRESETS {
            let suite = SuiteFile::parse(text).unwrap_or_else(|e| panic!("{name}: {e}"));
            let jobs = suite
                .jobs(&system)
                .unwrap_or_else(|e| panic!("{name}: {e}"));
            assert!(!jobs.is_empty(), "{name}");
            assert!(SuiteFile::preset(name).is_some());
        }
        assert!(SuiteFile::preset("nope").is_none());
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let text = "[suite]\nname = \"x\"\n[[test]]\nmodule = \"pi\"\nsize = \"1M\"\nbogus = 1\n";
        assert!(SuiteFile::parse(text).is_err());
    }

    #[test]
    fn empty_suite_is_rejected() {
        assert!(SuiteFile::parse("[suite]\nname = \"x\"\n").is_err());
    }

    #[test]
    fn scaling_covers_powers_of_two_and_max() {
        let system = SystemInfo::placeholder(); // 4 CPU lógicas
        let base = RunConfig::new(Module::Radical, "32M").expect("size");
        let jobs = scaling_jobs(&base, &system);
        let threads: Vec<String> = jobs.iter().map(|j| j.config.threads.to_string()).collect();
        assert_eq!(threads, vec!["1", "2", "4"]);
        assert!(jobs.iter().all(|j| j.config.mode == Mode::Multi));
    }
}
