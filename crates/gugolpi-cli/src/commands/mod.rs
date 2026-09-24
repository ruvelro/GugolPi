//! Un submódulo por subcomando y la maquinaria común de ejecución.

pub mod compare;
pub mod pi;
pub mod radical;
pub mod suite;
pub mod sysinfo;
pub mod zeta;

use std::io::Write;
use std::num::NonZeroUsize;
use std::sync::mpsc::Receiver;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::Context;
use gugolpi_core::{
    Benchmark, CancelToken, Mode, Progress, ProgressEvent, RunConfig, RunResult, SystemInfo,
    Threads, benchmark_for,
};

use crate::cli::{GlobalArgs, RunArgs};
use crate::exit::ExitCode;
use crate::output;

/// Aplica las opciones comunes de ejecución a una configuración.
pub fn apply_run_args(config: &mut RunConfig, args: &RunArgs) {
    config.mode = args.mode;
    config.threads = args.threads;
    config.affinity = args.affinity;
}

/// Un run planificado: etiqueta legible y configuración.
#[derive(Clone, Debug)]
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

/// Ejecuta un job `repeat` veces con progreso y resúmenes según las opciones globales.
pub fn run_job(
    global: &GlobalArgs,
    benchmark: &dyn Benchmark,
    job: &Job,
    repeat: u32,
    system: &SystemInfo,
) -> anyhow::Result<Vec<RunResult>> {
    let cancel = CancelToken::new();
    let mut results = Vec::with_capacity(repeat as usize);
    let verbose = !global.quiet && !global.json;

    for iteration in 1..=repeat {
        if verbose {
            let suffix = if repeat > 1 {
                format!(" (run {iteration}/{repeat})")
            } else {
                String::new()
            };
            eprintln!("Ejecutando {}{suffix}…", job.label);
        }
        let (progress, receiver) = Progress::channel();
        let progress = if verbose { progress } else { Progress::none() };
        let reporter = verbose.then(|| thread::spawn(move || report_progress(&receiver)));

        let result = benchmark.run(&job.config, system, &progress, &cancel);
        drop(progress);
        if let Some(handle) = reporter {
            let _ = handle.join();
        }
        let result = result.with_context(|| format!("{} no pudo completarse", job.label))?;

        if let Some(dir) = &global.out {
            let path = output::write_result(dir, &result)?;
            if verbose {
                eprintln!("Resultado guardado en {}", path.display());
            }
        }
        if verbose {
            output::print_summary(&result);
        }
        results.push(result);
    }
    if verbose {
        output::print_repeat_summary(&results);
    }
    Ok(results)
}

/// Punto común de los comandos `pi`, `radical` y `zeta`: barrido de escalado o run normal.
pub fn execute(global: &GlobalArgs, config: RunConfig, args: &RunArgs) -> anyhow::Result<ExitCode> {
    let benchmark = benchmark_for(config.module)?;
    let system = SystemInfo::collect();
    let jobs = if args.scaling {
        scaling_jobs(&config, &system)
    } else {
        vec![Job::from_config(config)]
    };

    let mut all = Vec::new();
    for job in &jobs {
        let results = run_job(global, benchmark.as_ref(), job, args.repeat, &system)?;
        all.push((job.clone(), results));
    }

    if global.json {
        let flat: Vec<RunResult> = all.iter().flat_map(|(_, r)| r.iter().cloned()).collect();
        output::print_json(&flat)?;
    } else if args.scaling && !global.quiet {
        output::print_scaling_table(&all);
    }
    Ok(exit_code_for(all.iter().flat_map(|(_, r)| r.iter())))
}

/// Código de salida a partir de los resultados: 2 si alguna verificación falló.
pub fn exit_code_for<'a>(results: impl Iterator<Item = &'a RunResult>) -> ExitCode {
    let mut any_failed = false;
    for r in results {
        any_failed |= r.verification.status == gugolpi_core::result::VerificationStatus::Failed;
    }
    if any_failed {
        ExitCode::VerificationFailed
    } else {
        ExitCode::Ok
    }
}

/// Muestra el progreso por stderr, sin inundar: como mucho una actualización cada 200 ms.
fn report_progress(receiver: &Receiver<ProgressEvent>) {
    let mut stderr = std::io::stderr().lock();
    let mut last_print: Option<Instant> = None;
    let mut line_open = false;
    for event in receiver {
        match event {
            ProgressEvent::Advanced { done, total } => {
                let due = last_print.is_none_or(|t| t.elapsed() >= Duration::from_millis(200));
                if due || done == total {
                    let pct = if total > 0 {
                        done as f64 / total as f64 * 100.0
                    } else {
                        0.0
                    };
                    let _ = write!(stderr, "\r  progreso {pct:5.1} %");
                    let _ = stderr.flush();
                    last_print = Some(Instant::now());
                    line_open = true;
                }
            }
            ProgressEvent::Loop {
                index,
                seconds,
                cumulative_seconds,
            } => {
                if line_open {
                    let _ = writeln!(stderr);
                    line_open = false;
                }
                let _ = writeln!(
                    stderr,
                    "  Loop {index:>2}: {seconds:>9.3} s  (acumulado {cumulative_seconds:>9.3} s)"
                );
            }
            ProgressEvent::Verifying | ProgressEvent::Finished if line_open => {
                let _ = writeln!(stderr);
                line_open = false;
            }
            _ => {}
        }
    }
    if line_open {
        let _ = writeln!(stderr);
    }
}
