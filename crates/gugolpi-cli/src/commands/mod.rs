//! Un submódulo por subcomando.

pub mod radical;
pub mod sysinfo;

use std::io::Write;
use std::sync::mpsc::Receiver;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::Context;
use gugolpi_core::{
    Benchmark, CancelToken, Progress, ProgressEvent, RunConfig, RunResult, SystemInfo,
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

/// Ejecuta `repeat` veces un benchmark, muestra progreso y resultados, y devuelve el código
/// de salida.
pub fn execute(
    global: &GlobalArgs,
    benchmark: &dyn Benchmark,
    config: &RunConfig,
    repeat: u32,
) -> anyhow::Result<ExitCode> {
    let system = SystemInfo::collect();
    let cancel = CancelToken::new();
    let mut results = Vec::with_capacity(repeat as usize);

    for iteration in 1..=repeat {
        if !global.quiet && !global.json {
            if repeat > 1 {
                eprintln!("Run {iteration}/{repeat}");
            }
            eprintln!(
                "Ejecutando {} {} en modo {}…",
                config.module, config.size, config.mode
            );
        }
        let show_progress = !global.quiet && !global.json;
        let (progress, receiver) = if show_progress {
            Progress::channel()
        } else {
            (Progress::none(), Progress::channel().1)
        };
        let reporter = show_progress.then(|| thread::spawn(move || report_progress(&receiver)));

        let result = benchmark.run(config, &system, &progress, &cancel);
        drop(progress);
        if let Some(handle) = reporter {
            let _ = handle.join();
        }
        let result = result.context("el benchmark no pudo completarse")?;

        if let Some(dir) = &global.out {
            let path = output::write_result(dir, &result)?;
            if !global.quiet && !global.json {
                eprintln!("Resultado guardado en {}", path.display());
            }
        }
        if !global.quiet && !global.json {
            output::print_summary(&result);
        }
        results.push(result);
    }

    if global.json {
        output::print_json(&results)?;
    } else if !global.quiet {
        output::print_repeat_summary(&results);
    }

    Ok(exit_code_for(&results))
}

fn exit_code_for(results: &[RunResult]) -> ExitCode {
    if results.iter().all(|r| r.verification.passed()) {
        ExitCode::Ok
    } else {
        ExitCode::VerificationFailed
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
