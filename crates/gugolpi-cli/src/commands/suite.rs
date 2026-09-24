//! `gugolpi suite`: ejecuta una lista de tests definida en TOML (preset embebido o fichero).

use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use gugolpi_core::config::{PiOptions, RadicalOptions};
use gugolpi_core::{Mode, Module, RunConfig, RunResult, SystemInfo, Threads, benchmark_for};
use serde::Deserialize;

use crate::cli::{GlobalArgs, SuiteArgs};
use crate::commands::{Job, exit_code_for, run_job, scaling_jobs};
use crate::exit::ExitCode;
use crate::output;

/// Presets embebidos en el binario.
const PRESETS: &[(&str, &str)] = &[
    ("classic", include_str!("../../../../suites/classic.toml")),
    ("trio", include_str!("../../../../suites/trio.toml")),
    ("full", include_str!("../../../../suites/full.toml")),
];

/// Fichero de suite.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SuiteFile {
    /// Cabecera.
    pub suite: SuiteMeta,
    /// Tests, en orden.
    #[serde(default, rename = "test")]
    pub tests: Vec<SuiteTest>,
}

/// Cabecera `[suite]`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SuiteMeta {
    /// Nombre.
    pub name: String,
    /// Repeticiones por defecto de cada test.
    #[serde(default = "one")]
    pub repeat: u32,
}

/// Una entrada `[[test]]`.
#[derive(Debug, Deserialize)]
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
    pub fn parse(text: &str) -> anyhow::Result<Self> {
        let suite: Self = toml::from_str(text).context("la suite no es un TOML válido")?;
        if suite.tests.is_empty() {
            bail!("la suite «{}» no tiene ningún [[test]]", suite.suite.name);
        }
        Ok(suite)
    }

    /// Carga un preset por nombre o un fichero por ruta.
    pub fn load(name_or_path: &str) -> anyhow::Result<Self> {
        if let Some((_, text)) = PRESETS.iter().find(|(name, _)| *name == name_or_path) {
            return Self::parse(text);
        }
        let path = Path::new(name_or_path);
        if path.exists() {
            let text = std::fs::read_to_string(path)
                .with_context(|| format!("no se pudo leer {}", path.display()))?;
            return Self::parse(&text);
        }
        let names: Vec<&str> = PRESETS.iter().map(|(n, _)| *n).collect();
        bail!(
            "«{name_or_path}» no es un preset ({}) ni un fichero existente",
            names.join(", ")
        );
    }

    /// Expande los tests a jobs concretos para esta máquina.
    pub fn jobs(&self, system: &SystemInfo) -> anyhow::Result<Vec<Job>> {
        let mut jobs = Vec::new();
        for (index, test) in self.tests.iter().enumerate() {
            let position = index + 1;
            let module: Module = test
                .module
                .parse()
                .with_context(|| format!("test {position}: módulo «{}»", test.module))?;
            let mut config = RunConfig::new(module, &test.size)
                .with_context(|| format!("test {position}: tamaño «{}»", test.size))?;
            config.mode = match &test.mode {
                Some(mode) => mode
                    .parse::<Mode>()
                    .with_context(|| format!("test {position}: modo"))?,
                None => Mode::Single,
            };
            config.threads = match &test.threads {
                Some(t) => t
                    .parse::<Threads>()
                    .with_context(|| format!("test {position}: hilos"))?,
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

/// Directorio de resultados: el pedido con `--out` o `results/<suite>-<fecha>/`.
fn results_dir(global: &GlobalArgs, suite_name: &str) -> PathBuf {
    global.out.clone().unwrap_or_else(|| {
        let stamp = gugolpi_core::timeutil::now_utc_rfc3339().replace([':', '-'], "");
        PathBuf::from("results").join(format!("{suite_name}-{stamp}"))
    })
}

/// Ejecuta la suite.
pub fn run(global: &GlobalArgs, args: &SuiteArgs) -> anyhow::Result<ExitCode> {
    let suite = SuiteFile::load(&args.suite)?;
    let system = SystemInfo::collect();
    let jobs = suite.jobs(&system)?;
    let repeat = args.repeat.unwrap_or(suite.suite.repeat).max(1);
    let verbose = !global.quiet && !global.json;

    if args.dry_run {
        if !global.quiet {
            println!(
                "Suite «{}» · {} job(s) · {repeat} repetición(es)",
                suite.suite.name,
                jobs.len()
            );
            for (i, job) in jobs.iter().enumerate() {
                println!("  {:>2}. {}", i + 1, job.label);
            }
        }
        return Ok(ExitCode::Ok);
    }

    let dir = results_dir(global, &suite.suite.name);
    let per_run_global = GlobalArgs {
        json: global.json,
        quiet: global.quiet,
        out: Some(dir.clone()),
    };
    if verbose {
        eprintln!(
            "Suite «{}»: {} job(s) × {repeat}; resultados en {}",
            suite.suite.name,
            jobs.len(),
            dir.display()
        );
    }

    let mut rows: Vec<(Job, Vec<RunResult>)> = Vec::with_capacity(jobs.len());
    for job in &jobs {
        let benchmark = benchmark_for(job.config.module)?;
        let results = run_job(&per_run_global, benchmark.as_ref(), job, repeat, &system)?;
        rows.push((job.clone(), results));
    }

    let csv_path = dir.join("summary.csv");
    output::write_suite_csv(&csv_path, &suite.suite.name, &rows)?;
    if global.json {
        let flat: Vec<RunResult> = rows.iter().flat_map(|(_, r)| r.iter().cloned()).collect();
        output::print_json(&flat)?;
    } else if !global.quiet {
        output::print_suite_table(&suite.suite.name, &rows);
        println!("Resumen CSV: {}", csv_path.display());
    }
    Ok(exit_code_for(rows.iter().flat_map(|(_, r)| r.iter())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_parse_and_expand() {
        let system = SystemInfo::collect();
        for (name, text) in PRESETS {
            let suite = SuiteFile::parse(text).unwrap_or_else(|e| panic!("{name}: {e}"));
            let jobs = suite
                .jobs(&system)
                .unwrap_or_else(|e| panic!("{name}: {e}"));
            assert!(!jobs.is_empty(), "{name}");
        }
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
}
