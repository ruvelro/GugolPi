//! Módulo Zeta: cuenta los primos hasta N con una criba de Eratóstenes segmentada.
//!
//! Es el módulo propio de GugolPi: mide enteros, caché L1/L2 y predicción de saltos, y se
//! verifica contra π(N) y la suma de primos conocidos. La criba usa rueda módulo 30 (un byte
//! representa 30 números: los 8 residuos coprimos con 2, 3 y 5) y segmentos del tamaño de la
//! caché L1 de datos.

pub mod reference;
mod sieve;

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use crate::benchmark::{Benchmark, CancelToken, preflight};
use crate::config::{Module, RunConfig};
use crate::error::{Error, Result};
use crate::progress::{Progress, ProgressEvent};
use crate::result::{
    RunResult, Timing, Verification, VerificationStatus, build_sealed, throughput_unit,
};
use crate::score::ZETA_CHUNK_SEGMENTS;
use crate::sysinfo::SystemInfo;
use crate::timeutil::now_utc_rfc3339;

pub use sieve::{SEGMENT_NUMBERS, Sieve, SieveTotals, count_primes};

/// Implementación del módulo Zeta.
#[derive(Clone, Copy, Debug, Default)]
pub struct Zeta;

/// Estadísticas de un hilo al terminar.
#[derive(Clone, Copy, Debug, Default)]
struct ThreadStats {
    seconds: f64,
    totals: SieveTotals,
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

/// Un hilo que toma bloques de segmentos de la cola dinámica hasta agotarla.
fn worker(
    sieve: &Sieve,
    next_chunk: &AtomicU64,
    done: &AtomicU64,
    progress: &Progress,
    cancel: &CancelToken,
) -> Result<ThreadStats> {
    let start = Instant::now();
    let mut totals = SieveTotals::default();
    let mut workspace = sieve.workspace();
    let chunk_numbers = ZETA_CHUNK_SEGMENTS * SEGMENT_NUMBERS;
    loop {
        if cancel.is_cancelled() {
            return Err(Error::Cancelled);
        }
        let index = next_chunk.fetch_add(1, Ordering::Relaxed);
        let Some(low) = index.checked_mul(chunk_numbers) else {
            break;
        };
        if low > sieve.limit() {
            break;
        }
        let high = low.saturating_add(chunk_numbers - 1).min(sieve.limit());
        totals = totals.merge(sieve.sieve_range(&mut workspace, low, high));
        let processed = high - low + 1;
        let total_done = done.fetch_add(processed, Ordering::Relaxed) + processed;
        progress.send(ProgressEvent::Advanced {
            done: total_done,
            total: sieve.limit() + 1,
        });
    }
    Ok(ThreadStats {
        seconds: start.elapsed().as_secs_f64(),
        totals,
    })
}

fn verify(totals: SieveTotals, limit: u64) -> Verification {
    let digest = Some(format!("{:016x}", totals.sum_mod_2_64));
    match reference::entry_for(limit) {
        Some(expected) => {
            let count_ok = expected.prime_count == totals.count;
            let sum_ok = expected.prime_sum_mod_2_64 == format!("{:016x}", totals.sum_mod_2_64);
            if count_ok && sum_ok {
                Verification {
                    status: VerificationStatus::Passed,
                    errors: 0,
                    digest,
                    max_fft_error: None,
                    detail: None,
                }
            } else {
                Verification {
                    status: VerificationStatus::Failed,
                    errors: u64::from(!count_ok) + u64::from(!sum_ok),
                    digest,
                    max_fft_error: None,
                    detail: Some(format!(
                        "π(N) = {} (esperado {}), suma mod 2^64 = {:016x} (esperada {})",
                        totals.count,
                        expected.prime_count,
                        totals.sum_mod_2_64,
                        expected.prime_sum_mod_2_64
                    )),
                }
            }
        }
        None => Verification {
            status: VerificationStatus::Unverified,
            errors: 0,
            digest,
            max_fft_error: None,
            detail: Some(format!(
                "π(N) = {}; no hay referencia embebida para este N",
                totals.count
            )),
        },
    }
}

impl Benchmark for Zeta {
    fn module(&self) -> Module {
        Module::Zeta
    }

    fn memory_required(&self, config: &RunConfig, _system: &SystemInfo) -> u64 {
        // Primos de criba hasta √N (≈ 48 B cada uno) más un segmento por hilo.
        let root = (config.size.value as f64).sqrt() as u64;
        let sieving_primes = root / 8 + 16;
        sieving_primes * 48 + 64 * 1024 * 64 + (16 << 20)
    }

    fn run(
        &self,
        config: &RunConfig,
        system: &SystemInfo,
        progress: &Progress,
        cancel: &CancelToken,
    ) -> Result<RunResult> {
        preflight(self, config, system)?;
        let limit = config.size.value;
        let threads = config.resolve_threads(system.logical_cpus, system.physical_cores);
        let started_utc = now_utc_rfc3339();
        progress.send(ProgressEvent::Started {
            module: Module::Zeta,
            total: limit + 1,
            unit: "números",
        });

        let clock = Instant::now();
        let sieve = Sieve::new(limit);
        let next_chunk = AtomicU64::new(0);
        let done = AtomicU64::new(0);
        let per_thread: Vec<Result<ThreadStats>> = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..threads)
                .map(|i| {
                    let (sieve, next_chunk, done) = (&sieve, &next_chunk, &done);
                    scope.spawn(move || {
                        pin_to_core(i, config.affinity);
                        worker(sieve, next_chunk, done, progress, cancel)
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|h| {
                    h.join().unwrap_or_else(|_| {
                        Err(Error::Internal("un hilo de Zeta falló".to_owned()))
                    })
                })
                .collect()
        });
        let total_seconds = clock.elapsed().as_secs_f64();

        let mut stats = Vec::with_capacity(threads);
        for s in per_thread {
            stats.push(s?);
        }
        let totals = stats
            .iter()
            .fold(SieveTotals::default(), |acc, s| acc.merge(s.totals));

        progress.send(ProgressEvent::Verifying);
        let verification = verify(totals, limit);
        let timing = Timing {
            started_utc,
            total_seconds,
            loops: Vec::new(),
            per_thread_seconds: stats.iter().map(|s| s.seconds).collect(),
            throughput: if total_seconds > 0.0 {
                limit as f64 / total_seconds
            } else {
                0.0
            },
            throughput_unit: throughput_unit(Module::Zeta).to_owned(),
        };
        progress.send(ProgressEvent::Finished);
        Ok(build_sealed(config, system, threads, timing, verification))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Mode, Threads};
    use std::num::NonZeroUsize;

    #[test]
    fn zeta_1g_verifies_single_and_multi() {
        let mut config = RunConfig::new(Module::Zeta, "1G").expect("size");
        let single = Zeta
            .run(
                &config,
                &SystemInfo::placeholder(),
                &Progress::none(),
                &CancelToken::new(),
            )
            .expect("run");
        assert!(single.verification.passed(), "{:?}", single.verification);
        assert!(single.official);

        config.mode = Mode::Multi;
        config.threads = Threads::Count(NonZeroUsize::new(3).expect("nz"));
        let multi = Zeta
            .run(
                &config,
                &SystemInfo::placeholder(),
                &Progress::none(),
                &CancelToken::new(),
            )
            .expect("run");
        assert!(multi.verification.passed(), "{:?}", multi.verification);
        assert_eq!(multi.verification.digest, single.verification.digest);
        assert_eq!(multi.test.threads_used, 3);
    }

    #[test]
    fn free_n_is_unverified_but_counts_right() {
        let config = RunConfig::new(Module::Zeta, "1000000").expect("size");
        let result = Zeta
            .run(
                &config,
                &SystemInfo::placeholder(),
                &Progress::none(),
                &CancelToken::new(),
            )
            .expect("run");
        assert_eq!(result.verification.status, VerificationStatus::Unverified);
        assert!(!result.official);
        assert!(
            result
                .verification
                .detail
                .as_deref()
                .is_some_and(|d| d.contains("78498"))
        );
    }
}
