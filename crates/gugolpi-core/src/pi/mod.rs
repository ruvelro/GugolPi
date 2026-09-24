//! Módulo Pi: dígitos decimales de Pi por Gauss–Legendre con multiplicación FFT, como SuperPi.
//!
//! El comportamiento por defecto es el de SuperPi: un hilo, tamaños clásicos (16K–32M) e informe
//! del tiempo de cada loop. El modo multi lanza instancias independientes del mismo cálculo, una
//! por hilo (el "SuperPi ×N" de los overclockers).

mod agm;
pub mod reference;

use std::time::Instant;

use crate::benchmark::{Benchmark, CancelToken, preflight};
use crate::config::{Module, RunConfig};
use crate::error::{Error, Result};
use crate::progress::{Progress, ProgressEvent};
use crate::result::{
    RunResult, Timing, Verification, VerificationStatus, build_sealed, sha256_hex, throughput_unit,
};
use crate::score::{FFT_ROUNDING_LIMIT, PI_GUARD_LIMBS};
use crate::sysinfo::SystemInfo;
use crate::timeutil::now_utc_rfc3339;

pub use agm::{PiComputation, compute};

/// Implementación del módulo Pi.
#[derive(Clone, Copy, Debug, Default)]
pub struct Pi;

/// Resultado de una instancia: cálculo, tiempo y hash de los dígitos.
struct Instance {
    seconds: f64,
    computation: PiComputation,
    digest: String,
}

fn pin_to_core(index: usize, enabled: bool) {
    if !enabled {
        return;
    }
    if let Some(ids) = core_affinity::get_core_ids()
        && let Some(id) = ids.get(index % ids.len())
    {
        core_affinity::set_for_current(*id);
    }
}

/// Memoria estimada de una instancia para `digits` decimales.
///
/// Búfer complejo y tablas de twiddles (16 B/punto cada uno) más los números de trabajo del
/// AGM y de Newton, calibrado con el RSS medido: 67 MB en 1M, 263 MB en 4M y 1,8 GB en 32M.
pub fn memory_per_instance(digits: u64) -> u64 {
    let limbs = digits / 4 + 1 + PI_GUARD_LIMBS as u64;
    let fft_points = (2 * limbs).next_power_of_two();
    fft_points * 32 + limbs * 100
}

/// Lanza `instances` cálculos independientes y devuelve sus resultados y el tiempo total.
fn run_instances(
    digits: u64,
    instances: usize,
    affinity: bool,
    progress: &Progress,
    cancel: &CancelToken,
) -> Result<(Vec<Instance>, f64)> {
    let clock = Instant::now();
    let results: Vec<Result<Instance>> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..instances)
            .map(|i| {
                scope.spawn(move || {
                    pin_to_core(i, affinity);
                    // Sólo la primera instancia informa de sus loops, para no entremezclar.
                    let silent = Progress::none();
                    let reporter = if i == 0 { progress } else { &silent };
                    let start = Instant::now();
                    let computation = compute(digits, reporter, cancel)?;
                    let seconds = start.elapsed().as_secs_f64();
                    let digest = sha256_hex(computation.decimals.as_bytes());
                    Ok(Instance {
                        seconds,
                        computation,
                        digest,
                    })
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| {
                h.join().unwrap_or_else(|_| {
                    Err(Error::Internal("una instancia de Pi falló".to_owned()))
                })
            })
            .collect()
    });
    let total_seconds = clock.elapsed().as_secs_f64();
    let mut done = Vec::with_capacity(instances);
    for instance in results {
        done.push(instance?);
    }
    Ok((done, total_seconds))
}

/// Coteja todas las instancias con la referencia y el límite de error de la FFT.
fn verify(done: &[Instance], digits: u64) -> Verification {
    let max_fft_error = done
        .iter()
        .map(|i| i.computation.max_fft_error)
        .fold(0.0, f64::max);
    let mut verification = Verification {
        status: VerificationStatus::Passed,
        errors: 0,
        digest: done.first().map(|i| i.digest.clone()),
        max_fft_error: Some(max_fft_error),
        detail: None,
    };
    if let Some(expected) = reference::sha256_for(digits) {
        let mismatches = done.iter().filter(|i| i.digest != expected).count() as u64;
        if mismatches > 0 {
            verification.status = VerificationStatus::Failed;
            verification.errors = mismatches;
            verification.detail = Some(format!(
                "{mismatches} de {} instancias no coinciden con los dígitos de referencia",
                done.len()
            ));
        }
    } else {
        verification.status = VerificationStatus::Unverified;
        verification.detail =
            Some("no hay dígitos de referencia embebidos para este tamaño".to_owned());
    }
    if max_fft_error > FFT_ROUNDING_LIMIT {
        verification.status = VerificationStatus::Failed;
        verification.errors += 1;
        verification.detail = Some(format!(
            "error de redondeo de la FFT {max_fft_error:.3} > {FFT_ROUNDING_LIMIT} (CPU inestable)"
        ));
    }
    verification
}

/// Guarda los dígitos de la primera instancia si se pidió (fuera del tiempo medido).
fn save_digits(config: &RunConfig, done: &[Instance]) -> Result<()> {
    if let Some(path) = &config.pi.save_digits
        && let Some(first) = done.first()
    {
        let text = format!("3.{}\n", first.computation.decimals);
        std::fs::write(path, text).map_err(|e| {
            Error::Internal(format!(
                "no se pudieron guardar los dígitos en {}: {e}",
                path.display()
            ))
        })?;
    }
    Ok(())
}

impl Benchmark for Pi {
    fn module(&self) -> Module {
        Module::Pi
    }

    fn memory_required(&self, config: &RunConfig, system: &SystemInfo) -> u64 {
        let instances = config.resolve_threads(system.logical_cpus, system.physical_cores) as u64;
        memory_per_instance(config.size.value) * instances
    }

    fn run(
        &self,
        config: &RunConfig,
        system: &SystemInfo,
        progress: &Progress,
        cancel: &CancelToken,
    ) -> Result<RunResult> {
        preflight(self, config, system)?;
        let digits = config.size.value;
        let instances = config.resolve_threads(system.logical_cpus, system.physical_cores);
        let started_utc = now_utc_rfc3339();
        progress.send(ProgressEvent::Started {
            module: Module::Pi,
            total: digits,
            unit: "dígitos",
        });

        let (done, total_seconds) =
            run_instances(digits, instances, config.affinity, progress, cancel)?;
        progress.send(ProgressEvent::Verifying);
        let verification = verify(&done, digits);
        save_digits(config, &done)?;

        let timing = Timing {
            started_utc,
            total_seconds,
            loops: done
                .first()
                .map(|i| i.computation.loops.clone())
                .unwrap_or_default(),
            per_thread_seconds: done.iter().map(|i| i.seconds).collect(),
            throughput: if total_seconds > 0.0 {
                digits as f64 * instances as f64 / total_seconds
            } else {
                0.0
            },
            throughput_unit: throughput_unit(Module::Pi).to_owned(),
        };
        progress.send(ProgressEvent::Finished);
        Ok(build_sealed(
            config,
            system,
            instances,
            timing,
            verification,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Mode;

    #[test]
    fn pi_16k_single_matches_reference() {
        let config = RunConfig::new(Module::Pi, "16K").expect("size");
        let result = Pi
            .run(
                &config,
                &SystemInfo::placeholder(),
                &Progress::none(),
                &CancelToken::new(),
            )
            .expect("run");
        assert!(result.verification.passed(), "{:?}", result.verification);
        assert_eq!(result.timing.loops.len(), 13);
        assert!(result.official);
        assert!(result.integrity_ok());
    }

    #[test]
    fn pi_multi_instances_all_verify() {
        let mut config = RunConfig::new(Module::Pi, "16K").expect("size");
        config.mode = Mode::Multi;
        config.threads = crate::config::Threads::Count(std::num::NonZeroUsize::new(2).expect("nz"));
        let result = Pi
            .run(
                &config,
                &SystemInfo::placeholder(),
                &Progress::none(),
                &CancelToken::new(),
            )
            .expect("run");
        assert!(result.verification.passed());
        assert_eq!(result.test.threads_used, 2);
        assert_eq!(result.timing.per_thread_seconds.len(), 2);
    }
}
