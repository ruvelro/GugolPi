//! El trait [`Benchmark`] y la fábrica por módulo.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::config::{Module, RunConfig};
use crate::error::{Error, Result};
use crate::progress::Progress;
use crate::result::RunResult;
use crate::sysinfo::SystemInfo;

/// Un benchmark ejecutable. Pi, Radical, Zeta y las cargas de Gúgol lo implementan.
///
/// La CLI, la GUI y las suites sólo hablan con este trait.
pub trait Benchmark: Send + Sync {
    /// Módulo que implementa.
    fn module(&self) -> Module;

    /// Estima la memoria necesaria en bytes para la configuración dada.
    fn memory_required(&self, config: &RunConfig) -> u64;

    /// Ejecuta el benchmark y devuelve el resultado ya verificado y sellado.
    ///
    /// La configuración debe ser del módulo de `self`; `system` se recoge una sola vez
    /// por el llamador y viaja dentro del resultado.
    fn run(
        &self,
        config: &RunConfig,
        system: &SystemInfo,
        progress: &Progress,
        cancel: &CancelToken,
    ) -> Result<RunResult>;
}

/// Token de cancelación compartible entre hilos.
#[derive(Clone, Debug, Default)]
pub struct CancelToken {
    flag: Arc<AtomicBool>,
}

impl CancelToken {
    /// Token nuevo, no cancelado.
    pub fn new() -> Self {
        Self::default()
    }

    /// Pide la cancelación. Los benchmarks la comprueban entre bloques de trabajo.
    pub fn cancel(&self) {
        self.flag.store(true, Ordering::Release);
    }

    /// `true` si alguien ha pedido cancelar.
    pub fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::Acquire)
    }
}

/// Devuelve la implementación de un módulo.
///
/// Un módulo definido en la spec pero aún no implementado devuelve [`Error::Unavailable`].
pub fn benchmark_for(module: Module) -> Result<Box<dyn Benchmark>> {
    match module {
        Module::Radical => Ok(Box::new(crate::radical::Radical)),
        Module::Pi | Module::Zeta => Err(Error::Unavailable(module)),
    }
}

/// Comprobaciones comunes antes de cualquier run: módulo correcto y memoria suficiente.
pub(crate) fn preflight(
    benchmark: &dyn Benchmark,
    config: &RunConfig,
    system: &SystemInfo,
) -> Result<()> {
    if config.module != benchmark.module() {
        return Err(Error::InvalidConfig(format!(
            "la configuración es del módulo {} pero se ejecuta {}",
            config.module,
            benchmark.module()
        )));
    }
    let required = benchmark.memory_required(config);
    if required > system.available_memory_bytes {
        return Err(Error::InsufficientMemory {
            required,
            available: system.available_memory_bytes,
        });
    }
    Ok(())
}
