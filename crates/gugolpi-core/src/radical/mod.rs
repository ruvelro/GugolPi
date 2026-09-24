//! Módulo Radical: raíces cuadradas de 1..N por Newton–Raphson, como wPrime.
//!
//! El algoritmo por número `k` replica la descripción oficial de wPrime:
//!
//! 1. arrancar en `x = k / 2`;
//! 2. iterar `x ← x − (x² − k) / (2x)` hasta que el signo de `(x² − k) / (2x)` cambie
//!    respecto a la iteración anterior (o sea exactamente cero);
//! 3. aplicar [`RADICAL_REFINE_ITERATIONS`] iteraciones más;
//! 4. verificar `|x² − k| ≤ k · 2^RADICAL_TOLERANCE_EXPONENT`.
//!
//! Todo en `f64` y sin la instrucción `sqrt`: la carga es la cadena de divisiones y
//! multiplicaciones de Newton.

mod newton;

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use crate::benchmark::{Benchmark, CancelToken, preflight};
use crate::config::{Module, RunConfig};
use crate::error::{Error, Result};
use crate::progress::{Progress, ProgressEvent};
use crate::result::{
    RunResult, Timing, Verification, VerificationStatus, build_sealed, throughput_unit,
};
use crate::score::RADICAL_CHUNK;
use crate::sysinfo::SystemInfo;
use crate::timeutil::now_utc_rfc3339;

pub use newton::newton_sqrt;

/// Implementación del módulo Radical.
#[derive(Clone, Copy, Debug, Default)]
pub struct Radical;

/// Estadísticas de un hilo al terminar su parte.
#[derive(Clone, Copy, Debug, Default)]
struct ThreadStats {
    seconds: f64,
    numbers: u64,
    errors: u64,
    checksum: u64,
}

impl ThreadStats {
    fn merge(self, other: Self) -> Self {
        Self {
            seconds: self.seconds.max(other.seconds),
            numbers: self.numbers + other.numbers,
            errors: self.errors + other.errors,
            checksum: self.checksum.wrapping_add(other.checksum),
        }
    }
}

/// Procesa el rango cerrado `[first, last]` y devuelve errores y checksum.
///
/// El checksum es la suma módulo 2^64 de la representación en bits de cada raíz: es
/// independiente del orden y del reparto entre hilos, así que dos runs correctos del mismo
/// tamaño dan el mismo valor en cualquier máquina.
fn process_range(first: u64, last: u64) -> (u64, u64) {
    let mut errors = 0_u64;
    let mut checksum = 0_u64;
    for k in first..=last {
        let (root, ok) = newton::sqrt_and_check(std::hint::black_box(k as f64));
        errors += u64::from(!ok);
        checksum = checksum.wrapping_add(root.to_bits());
    }
    (errors, checksum)
}

/// Divide `[1, n]` en bloques de [`RADICAL_CHUNK`] y devuelve el rango del bloque `index`.
fn chunk_range(n: u64, index: u64) -> Option<(u64, u64)> {
    let first = index.checked_mul(RADICAL_CHUNK)?.checked_add(1)?;
    if first > n {
        return None;
    }
    let last = first.saturating_add(RADICAL_CHUNK - 1).min(n);
    Some((first, last))
}

#[cfg(test)]
fn chunk_count(n: u64) -> u64 {
    n.div_ceil(RADICAL_CHUNK)
}

/// Un hilo que toma bloques de la cola dinámica hasta agotarla.
fn dynamic_worker(
    n: u64,
    next_chunk: &AtomicU64,
    done: &AtomicU64,
    progress: &Progress,
    cancel: &CancelToken,
) -> Result<ThreadStats> {
    let start = Instant::now();
    let mut stats = ThreadStats::default();
    loop {
        if cancel.is_cancelled() {
            return Err(Error::Cancelled);
        }
        let index = next_chunk.fetch_add(1, Ordering::Relaxed);
        let Some((first, last)) = chunk_range(n, index) else {
            break;
        };
        let (errors, checksum) = process_range(first, last);
        let count = last - first + 1;
        stats.numbers += count;
        stats.errors += errors;
        stats.checksum = stats.checksum.wrapping_add(checksum);
        let total_done = done.fetch_add(count, Ordering::Relaxed) + count;
        progress.send(ProgressEvent::Advanced {
            done: total_done,
            total: n,
        });
    }
    stats.seconds = start.elapsed().as_secs_f64();
    Ok(stats)
}

/// Un hilo con un rango contiguo fijo (reparto estático).
fn static_worker(
    first: u64,
    last: u64,
    n: u64,
    done: &AtomicU64,
    progress: &Progress,
    cancel: &CancelToken,
) -> Result<ThreadStats> {
    let start = Instant::now();
    let mut stats = ThreadStats::default();
    let mut lo = first;
    while lo <= last {
        if cancel.is_cancelled() {
            return Err(Error::Cancelled);
        }
        let hi = lo.saturating_add(RADICAL_CHUNK - 1).min(last);
        let (errors, checksum) = process_range(lo, hi);
        let count = hi - lo + 1;
        stats.numbers += count;
        stats.errors += errors;
        stats.checksum = stats.checksum.wrapping_add(checksum);
        let total_done = done.fetch_add(count, Ordering::Relaxed) + count;
        progress.send(ProgressEvent::Advanced {
            done: total_done,
            total: n,
        });
        lo = hi + 1;
    }
    stats.seconds = start.elapsed().as_secs_f64();
    Ok(stats)
}

/// Rango contiguo del hilo `i` de `threads` sobre `[1, n]`.
fn static_range(n: u64, i: usize, threads: usize) -> (u64, u64) {
    let threads = threads as u64;
    let i = i as u64;
    let first = n * i / threads + 1;
    let last = n * (i + 1) / threads;
    (first, last)
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

impl Benchmark for Radical {
    fn module(&self) -> Module {
        Module::Radical
    }

    fn memory_required(&self, _config: &RunConfig) -> u64 {
        // Sólo pilas de hilo y contadores: despreciable.
        16 << 20
    }

    fn run(
        &self,
        config: &RunConfig,
        system: &SystemInfo,
        progress: &Progress,
        cancel: &CancelToken,
    ) -> Result<RunResult> {
        preflight(self, config, system)?;
        let n = config.size.value;
        let threads = config.resolve_threads(system.logical_cpus, system.physical_cores);
        let static_split = config.radical.static_split;

        let started_utc = now_utc_rfc3339();
        progress.send(ProgressEvent::Started {
            module: Module::Radical,
            total: n,
            unit: "números",
        });

        let next_chunk = AtomicU64::new(0);
        let done = AtomicU64::new(0);
        let clock = Instant::now();

        let per_thread: Vec<Result<ThreadStats>> = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..threads)
                .map(|i| {
                    let next_chunk = &next_chunk;
                    let done = &done;
                    scope.spawn(move || {
                        pin_to_core(i, config.affinity);
                        if static_split {
                            let (first, last) = static_range(n, i, threads);
                            static_worker(first, last, n, done, progress, cancel)
                        } else {
                            dynamic_worker(n, next_chunk, done, progress, cancel)
                        }
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|h| {
                    h.join().unwrap_or_else(|_| {
                        Err(Error::Internal("un hilo de Radical falló".to_owned()))
                    })
                })
                .collect()
        });
        let total_seconds = clock.elapsed().as_secs_f64();

        let mut stats_by_thread = Vec::with_capacity(threads);
        for stats in per_thread {
            stats_by_thread.push(stats?);
        }
        let merged = stats_by_thread
            .iter()
            .copied()
            .fold(ThreadStats::default(), ThreadStats::merge);

        progress.send(ProgressEvent::Verifying);
        let mut verification = Verification {
            status: VerificationStatus::Passed,
            errors: merged.errors,
            digest: Some(format!("{:016x}", merged.checksum)),
            max_fft_error: None,
            detail: None,
        };
        if merged.numbers != n {
            verification.status = VerificationStatus::Failed;
            verification.detail = Some(format!("se procesaron {} números de {n}", merged.numbers));
        } else if merged.errors > 0 {
            verification.status = VerificationStatus::Failed;
            verification.detail = Some(format!(
                "{} raíces no cumplen x² = k dentro de la tolerancia",
                merged.errors
            ));
        }

        let timing = Timing {
            started_utc,
            total_seconds,
            loops: Vec::new(),
            per_thread_seconds: stats_by_thread.iter().map(|s| s.seconds).collect(),
            throughput: if total_seconds > 0.0 {
                n as f64 / total_seconds
            } else {
                0.0
            },
            throughput_unit: throughput_unit(Module::Radical).to_owned(),
        };
        progress.send(ProgressEvent::Finished);
        Ok(build_sealed(config, system, threads, timing, verification))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Mode, RadicalOptions, Threads};
    use std::num::NonZeroUsize;

    fn run(n: &str, mode: Mode, static_split: bool) -> RunResult {
        let mut config = RunConfig::new(Module::Radical, n).expect("size");
        config.mode = mode;
        config.threads = Threads::Count(NonZeroUsize::new(3).expect("nonzero"));
        config.radical = RadicalOptions { static_split };
        Radical
            .run(
                &config,
                &SystemInfo::placeholder(),
                &Progress::none(),
                &CancelToken::new(),
            )
            .expect("run")
    }

    #[test]
    fn chunking_covers_the_whole_range() {
        let n = 2_500_001;
        let mut covered = 0;
        let mut index = 0;
        while let Some((first, last)) = chunk_range(n, index) {
            covered += last - first + 1;
            index += 1;
        }
        assert_eq!(covered, n);
        assert_eq!(index, chunk_count(n));
        assert!(chunk_range(n, index).is_none());
    }

    #[test]
    fn static_ranges_partition_without_gaps() {
        let n = 10_000_007;
        let threads = 7;
        let mut expected_first = 1;
        for i in 0..threads {
            let (first, last) = static_range(n, i, threads);
            assert_eq!(first, expected_first);
            expected_first = last + 1;
        }
        assert_eq!(expected_first, n + 1);
    }

    #[test]
    fn single_and_multi_give_identical_checksums() {
        let single = run("3000000", Mode::Single, false);
        let dynamic = run("3000000", Mode::Multi, false);
        let fixed = run("3000000", Mode::Multi, true);
        assert!(single.verification.passed());
        assert_eq!(single.test.threads_used, 1);
        assert_eq!(dynamic.test.threads_used, 3);
        assert_eq!(single.verification.digest, dynamic.verification.digest);
        assert_eq!(single.verification.digest, fixed.verification.digest);
        assert!(single.integrity_ok());
        assert!(!single.official, "un N libre no puntúa");
        assert!(!fixed.test.config.is_official());
    }

    #[test]
    fn cancellation_is_reported() {
        let config = RunConfig::new(Module::Radical, "32M").expect("size");
        let cancel = CancelToken::new();
        cancel.cancel();
        let err = Radical
            .run(
                &config,
                &SystemInfo::placeholder(),
                &Progress::none(),
                &cancel,
            )
            .expect_err("debe cancelar");
        assert!(matches!(err, Error::Cancelled));
    }

    #[test]
    fn wrong_module_is_rejected() {
        let config = RunConfig::new(Module::Zeta, "100M").expect("size");
        let err = Radical
            .run(
                &config,
                &SystemInfo::placeholder(),
                &Progress::none(),
                &CancelToken::new(),
            )
            .expect_err("módulo incorrecto");
        assert!(matches!(err, Error::InvalidConfig(_)));
    }
}
