//! Ficha del sistema que acompaña a cada resultado y detección del nivel SIMD.

use serde::{Deserialize, Serialize};
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};

/// Ficha del sistema.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemInfo {
    /// Modelo de CPU tal como lo reporta el sistema.
    pub cpu_model: String,
    /// Núcleos físicos.
    pub physical_cores: usize,
    /// CPU lógicas (hilos de hardware).
    pub logical_cpus: usize,
    /// Frecuencia reportada en MHz, si el sistema la expone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cpu_frequency_mhz: Option<u64>,
    /// Memoria total en bytes.
    pub total_memory_bytes: u64,
    /// Memoria disponible en bytes al recoger la ficha.
    pub available_memory_bytes: u64,
    /// Nombre del sistema operativo.
    pub os_name: String,
    /// Versión del sistema operativo.
    pub os_version: String,
    /// Arquitectura de la build (`x86_64`, `aarch64`).
    pub arch: String,
    /// Nivel SIMD detectado en tiempo de ejecución.
    pub simd_level: String,
}

impl SystemInfo {
    /// Recoge la ficha de la máquina actual. Tarda unas decenas de milisegundos.
    pub fn collect() -> Self {
        let refresh = RefreshKind::nothing()
            .with_cpu(CpuRefreshKind::nothing().with_frequency())
            .with_memory(MemoryRefreshKind::nothing().with_ram());
        let system = System::new_with_specifics(refresh);
        let cpus = system.cpus();
        let cpu_model = cpus
            .first()
            .map(|cpu| cpu.brand().trim().to_owned())
            .filter(|brand| !brand.is_empty())
            .unwrap_or_else(|| "unknown".to_owned());
        let cpu_frequency_mhz = cpus.first().map(sysinfo::Cpu::frequency).filter(|&f| f > 0);
        let logical_cpus = cpus.len().max(1);
        let physical_cores = System::physical_core_count().unwrap_or(logical_cpus).max(1);

        Self {
            cpu_model,
            physical_cores,
            logical_cpus,
            cpu_frequency_mhz,
            total_memory_bytes: system.total_memory(),
            available_memory_bytes: available_memory(&system),
            os_name: System::name().unwrap_or_else(|| std::env::consts::OS.to_owned()),
            os_version: System::os_version().unwrap_or_else(|| "unknown".to_owned()),
            arch: std::env::consts::ARCH.to_owned(),
            simd_level: simd_level().to_owned(),
        }
    }

    /// Ficha ficticia para tests: dos núcleos y memoria de sobra.
    #[cfg(test)]
    pub(crate) fn placeholder() -> Self {
        Self {
            cpu_model: "Test CPU".to_owned(),
            physical_cores: 2,
            logical_cpus: 4,
            cpu_frequency_mhz: Some(3_000),
            total_memory_bytes: 16 << 30,
            available_memory_bytes: 8 << 30,
            os_name: "TestOS".to_owned(),
            os_version: "1.0".to_owned(),
            arch: "test".to_owned(),
            simd_level: "scalar".to_owned(),
        }
    }
}

/// Memoria disponible con reserva: algunos sistemas no exponen «disponible» y devuelven 0.
///
/// En ese caso se usa total − usada, y como último recurso la memoria total, para que la
/// comprobación previa de memoria no rechace runs por un dato ausente.
fn available_memory(system: &System) -> u64 {
    let total = system.total_memory();
    let available = system.available_memory();
    if available > 0 {
        return available;
    }
    let used = system.used_memory();
    if used > 0 && used < total {
        return total - used;
    }
    total
}

/// Nivel SIMD más alto disponible en la CPU actual.
///
/// Se guarda en cada resultado porque el despacho en tiempo de ejecución elige la ruta de
/// cálculo según este valor.
pub fn simd_level() -> &'static str {
    #[cfg(target_arch = "x86_64")]
    {
        if std::arch::is_x86_feature_detected!("avx512f") {
            "avx512"
        } else if std::arch::is_x86_feature_detected!("avx2")
            && std::arch::is_x86_feature_detected!("fma")
        {
            "avx2+fma"
        } else if std::arch::is_x86_feature_detected!("avx") {
            "avx"
        } else {
            "sse2"
        }
    }
    #[cfg(target_arch = "aarch64")]
    {
        // NEON es obligatorio en AArch64.
        "neon"
    }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        "scalar"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collect_gives_sane_values() {
        let info = SystemInfo::collect();
        assert!(info.logical_cpus >= 1);
        assert!(info.physical_cores >= 1);
        assert!(info.physical_cores <= info.logical_cpus);
        assert!(info.total_memory_bytes > 0);
        assert!(!info.simd_level.is_empty());
    }
}
