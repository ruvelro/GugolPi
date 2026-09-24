//! `gugolpi suite`: ejecuta una lista de tests definida en TOML (preset embebido o fichero).

use std::path::PathBuf;

use gugolpi_core::suite::{Job, SuiteFile};
use gugolpi_core::{RunResult, SystemInfo, benchmark_for};

use crate::cli::{GlobalArgs, SuiteArgs};
use crate::commands::{exit_code_for, run_job};
use crate::exit::ExitCode;
use crate::output;

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
