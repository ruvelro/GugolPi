//! `gugolpi compare`: tabla comparativa de ficheros de resultado.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::Context;
use gugolpi_core::RunResult;
use gugolpi_core::result::VerificationStatus;

use crate::cli::{CompareArgs, GlobalArgs};
use crate::exit::ExitCode;
use crate::output;

/// Un resultado cargado de disco.
#[derive(Debug)]
pub struct Loaded {
    /// Ruta de origen.
    pub path: PathBuf,
    /// Resultado.
    pub result: RunResult,
    /// `true` si el hash de integridad coincide.
    pub intact: bool,
}

/// Recoge ficheros `.json` de las rutas dadas (directorios incluidos, sin recursión).
fn collect_paths(inputs: &[PathBuf]) -> anyhow::Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for input in inputs {
        if input.is_dir() {
            let mut entries: Vec<PathBuf> = std::fs::read_dir(input)
                .with_context(|| format!("no se pudo leer {}", input.display()))?
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|ext| ext == "json"))
                .collect();
            entries.sort();
            paths.extend(entries);
        } else {
            paths.push(input.clone());
        }
    }
    Ok(paths)
}

/// Carga un fichero de resultado (un objeto o una lista de objetos).
fn load(path: &Path) -> anyhow::Result<Vec<Loaded>> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("no se pudo leer {}", path.display()))?;
    let results: Vec<RunResult> = match serde_json::from_str::<RunResult>(&text) {
        Ok(single) => vec![single],
        Err(_) => serde_json::from_str(&text)
            .with_context(|| format!("{} no es un resultado de GugolPi", path.display()))?,
    };
    Ok(results
        .into_iter()
        .map(|result| Loaded {
            intact: result.integrity_ok(),
            path: path.to_path_buf(),
            result,
        })
        .collect())
}

/// Clave de comparabilidad: mismo módulo, tamaño y modo.
fn group_key(result: &RunResult) -> String {
    let cfg = &result.test.config;
    format!("{} {} {}", cfg.module, cfg.size, cfg.mode)
}

/// Ejecuta la comparación.
pub fn run(global: &GlobalArgs, args: &CompareArgs) -> anyhow::Result<ExitCode> {
    let mut loaded = Vec::new();
    for path in collect_paths(&args.inputs)? {
        loaded.extend(load(&path)?);
    }
    let shown: Vec<&Loaded> = loaded
        .iter()
        .filter(|l| args.include_unofficial || l.result.official)
        .collect();
    let hidden = loaded.len() - shown.len();

    let mut groups: BTreeMap<String, Vec<&Loaded>> = BTreeMap::new();
    for item in shown {
        groups
            .entry(group_key(&item.result))
            .or_default()
            .push(item);
    }
    for group in groups.values_mut() {
        group.sort_by(|a, b| {
            a.result
                .timing
                .total_seconds
                .total_cmp(&b.result.timing.total_seconds)
        });
    }

    if args.csv {
        output::print_compare_csv(&groups)?;
    } else if global.json {
        let all: Vec<&RunResult> = groups.values().flatten().map(|l| &l.result).collect();
        println!("{}", serde_json::to_string_pretty(&all)?);
    } else if !global.quiet {
        output::print_compare_table(&groups);
        if hidden > 0 {
            println!("({hidden} resultado(s) no oficiales ocultos; usa --include-unofficial)");
        }
    }
    let any_failed = loaded
        .iter()
        .any(|l| l.result.verification.status == VerificationStatus::Failed);
    Ok(if any_failed {
        ExitCode::VerificationFailed
    } else {
        ExitCode::Ok
    })
}
