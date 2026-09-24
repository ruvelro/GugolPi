//! Sesión de ejecución: una lista de jobs que corre en un hilo aparte y emite eventos.
//!
//! Eventos (todos con payload JSON):
//! - `session-started` `{ jobs, repeat }`
//! - `run-progress` `{ job, index, run, of, event }` con el `ProgressEvent` del núcleo
//! - `run-finished` `{ job, index, run, of, result, path }`
//! - `run-error` `{ job, index, message }`
//! - `session-finished` `{ cancelled }`

use std::thread;

use gugolpi_core::suite::Job;
use gugolpi_core::{CancelToken, Error, Progress, ProgressEvent, RunResult, benchmark_for};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::state::AppState;

#[derive(Clone, Serialize)]
struct SessionStarted {
    jobs: Vec<String>,
    repeat: u32,
}

#[derive(Clone, Serialize)]
struct RunProgress {
    job: String,
    index: usize,
    run: u32,
    of: u32,
    event: ProgressEvent,
}

#[derive(Clone, Serialize)]
struct RunFinished {
    job: String,
    index: usize,
    run: u32,
    of: u32,
    result: RunResult,
    path: String,
}

#[derive(Clone, Serialize)]
struct RunError {
    job: String,
    index: usize,
    message: String,
}

#[derive(Clone, Serialize)]
struct SessionFinished {
    cancelled: bool,
}

fn emit<T: Serialize + Clone>(app: &AppHandle, name: &str, payload: T) {
    if let Err(err) = app.emit(name, payload) {
        eprintln!("no se pudo emitir {name}: {err}");
    }
}

/// Lanza la sesión en un hilo. Devuelve error si ya hay una en curso.
pub fn spawn(app: AppHandle, jobs: Vec<Job>, repeat: u32) -> Result<(), String> {
    let repeat = repeat.max(1);
    let cancel = app.state::<AppState>().begin_session()?;
    thread::Builder::new()
        .name("gugolpi-session".to_owned())
        .spawn(move || run_session(&app, &jobs, repeat, &cancel))
        .map_err(|e| format!("no se pudo crear el hilo de ejecución: {e}"))?;
    Ok(())
}

fn run_session(app: &AppHandle, jobs: &[Job], repeat: u32, cancel: &CancelToken) {
    let labels: Vec<String> = jobs.iter().map(|j| j.label.clone()).collect();
    emit(
        app,
        "session-started",
        SessionStarted {
            jobs: labels,
            repeat,
        },
    );
    let state = app.state::<AppState>();
    let mut cancelled = false;

    'jobs: for (index, job) in jobs.iter().enumerate() {
        let benchmark = match benchmark_for(job.config.module) {
            Ok(b) => b,
            Err(err) => {
                emit(
                    app,
                    "run-error",
                    RunError {
                        job: job.label.clone(),
                        index,
                        message: err.to_string(),
                    },
                );
                continue;
            }
        };
        for run in 1..=repeat {
            if cancel.is_cancelled() {
                cancelled = true;
                break 'jobs;
            }
            let (progress, receiver) = Progress::channel();
            let forwarder = {
                let app = app.clone();
                let label = job.label.clone();
                thread::spawn(move || {
                    for event in receiver {
                        emit(
                            &app,
                            "run-progress",
                            RunProgress {
                                job: label.clone(),
                                index,
                                run,
                                of: repeat,
                                event,
                            },
                        );
                    }
                })
            };
            let outcome = benchmark.run(&job.config, &state.system, &progress, cancel);
            drop(progress);
            let _ = forwarder.join();
            match outcome {
                Ok(result) => match result.save_to_dir(&state.results_dir) {
                    Ok(path) => emit(
                        app,
                        "run-finished",
                        RunFinished {
                            job: job.label.clone(),
                            index,
                            run,
                            of: repeat,
                            result,
                            path: path.display().to_string(),
                        },
                    ),
                    Err(err) => emit(
                        app,
                        "run-error",
                        RunError {
                            job: job.label.clone(),
                            index,
                            message: err.to_string(),
                        },
                    ),
                },
                Err(Error::Cancelled) => {
                    cancelled = true;
                    break 'jobs;
                }
                Err(err) => {
                    emit(
                        app,
                        "run-error",
                        RunError {
                            job: job.label.clone(),
                            index,
                            message: err.to_string(),
                        },
                    );
                    break;
                }
            }
        }
    }
    state.end_session();
    emit(app, "session-finished", SessionFinished { cancelled });
}
