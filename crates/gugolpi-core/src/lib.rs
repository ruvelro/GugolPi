//! Núcleo de GugolPi: benchmarks de CPU con verificación matemática.
//!
//! El núcleo no sabe de terminales ni de interfaces gráficas. Expone:
//!
//! - el trait [`Benchmark`] y la fábrica [`benchmark_for`], que devuelven la
//!   implementación de cada módulo ([`Module::Pi`], [`Module::Radical`], [`Module::Zeta`]);
//! - [`RunConfig`], la configuración explícita de un run;
//! - [`RunResult`], el resultado serializable y sellado con un hash de integridad;
//! - [`Progress`] y [`ProgressEvent`], el canal tipado de progreso;
//! - [`SystemInfo`], la ficha del sistema que acompaña a cada resultado.
//!
//! Las constantes que afectan a la puntuación viven en [`score`].

#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::float_cmp
    )
)]

pub mod benchmark;
pub mod bignum;
pub mod config;
pub mod error;
pub mod pi;
pub mod progress;
pub mod radical;
pub mod result;
pub mod score;
pub mod sysinfo;
pub mod timeutil;
pub mod zeta;

pub use benchmark::{Benchmark, CancelToken, benchmark_for};
pub use config::{Mode, Module, RunConfig, Size, Threads};
pub use error::{Error, Result};
pub use progress::{Progress, ProgressEvent};
pub use result::RunResult;
pub use sysinfo::SystemInfo;
