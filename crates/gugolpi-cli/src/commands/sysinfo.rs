//! `gugolpi sysinfo`.

use gugolpi_core::SystemInfo;
use gugolpi_core::result::ToolInfo;

use crate::cli::{GlobalArgs, SysinfoArgs};
use crate::exit::ExitCode;

/// Muestra la ficha del sistema.
pub fn run(global: &GlobalArgs, _args: &SysinfoArgs) -> anyhow::Result<ExitCode> {
    let system = SystemInfo::collect();
    let tool = ToolInfo::current();
    if global.json {
        let doc = serde_json::json!({ "tool": tool, "system": system });
        println!("{}", serde_json::to_string_pretty(&doc)?);
    } else if !global.quiet {
        println!("CPU:        {}", system.cpu_model);
        println!(
            "Núcleos:    {} físicos · {} lógicos",
            system.physical_cores, system.logical_cpus
        );
        if let Some(mhz) = system.cpu_frequency_mhz {
            println!("Frecuencia: {mhz} MHz");
        }
        println!(
            "Memoria:    {:.1} GiB total · {:.1} GiB disponibles",
            system.total_memory_bytes as f64 / f64::from(1u32 << 30),
            system.available_memory_bytes as f64 / f64::from(1u32 << 30)
        );
        println!(
            "SO:         {} {} ({})",
            system.os_name, system.os_version, system.arch
        );
        println!("SIMD:       {}", system.simd_level);
        println!(
            "GugolPi:    {} · commit {} · score_version {}",
            tool.version, tool.commit, tool.score_version
        );
    }
    Ok(ExitCode::Ok)
}
