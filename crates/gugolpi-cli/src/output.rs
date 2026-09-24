//! Presentación de resultados: resumen legible, JSON y ficheros.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::Context;
use gugolpi_core::RunResult;

/// Resumen legible de un run, al estilo de los originales.
pub fn print_summary(result: &RunResult) {
    let test = &result.test;
    let cfg = &test.config;
    println!();
    println!(
        "{} {} · modo {} · {} hilo(s){}",
        capitalize(cfg.module.name()),
        cfg.size,
        cfg.mode,
        test.threads_used,
        if cfg.affinity { " · afinidad" } else { "" }
    );
    for l in &result.timing.loops {
        println!(
            "  Loop {:>2}: {:>9.3} s  (acumulado {:>9.3} s)",
            l.index, l.seconds, l.cumulative_seconds
        );
    }
    println!("  Tiempo total: {:.3} s", result.timing.total_seconds);
    println!(
        "  Throughput:   {} {}",
        format_thousands(result.timing.throughput.round() as u64),
        result.timing.throughput_unit
    );
    if let Some((min, max, mean)) = result.timing.thread_spread()
        && test.threads_used > 1
    {
        let imbalance = if max > 0.0 {
            (max - min) / max * 100.0
        } else {
            0.0
        };
        println!(
            "  Por hilo:     min {min:.3} s · max {max:.3} s · media {mean:.3} s · desequilibrio {imbalance:.1} %"
        );
    }
    let verification = &result.verification;
    let status = if verification.passed() {
        "correcta"
    } else {
        "FALLIDA"
    };
    println!("  Verificación: {status} ({} errores)", verification.errors);
    if let Some(detail) = &verification.detail {
        println!("                {detail}");
    }
    if let Some(digest) = &verification.digest {
        println!("  Checksum:     {digest}");
    }
    println!(
        "  Puntuable:    {} · score_version {} · {}",
        if result.official { "sí" } else { "no" },
        result.tool.score_version,
        result.tool.simd_level
    );
}

/// Resumen de varias repeticiones: mejor, media y desviación típica del tiempo total.
pub fn print_repeat_summary(results: &[RunResult]) {
    if results.len() < 2 {
        return;
    }
    let times: Vec<f64> = results.iter().map(|r| r.timing.total_seconds).collect();
    let best = times.iter().copied().fold(f64::MAX, f64::min);
    let mean = times.iter().sum::<f64>() / times.len() as f64;
    let variance = times.iter().map(|t| (t - mean).powi(2)).sum::<f64>() / (times.len() - 1) as f64;
    println!();
    println!(
        "Repeticiones: {} · mejor {:.3} s · media {:.3} s · desviación {:.3} s",
        results.len(),
        best,
        mean,
        variance.sqrt()
    );
}

/// Imprime uno o varios resultados como JSON por stdout.
pub fn print_json(results: &[RunResult]) -> anyhow::Result<()> {
    let mut stdout = std::io::stdout().lock();
    if let [single] = results {
        serde_json::to_writer_pretty(&mut stdout, single)?;
    } else {
        serde_json::to_writer_pretty(&mut stdout, results)?;
    }
    writeln!(stdout)?;
    Ok(())
}

/// Guarda un resultado en `dir` con un nombre único y devuelve la ruta.
pub fn write_result(dir: &Path, result: &RunResult) -> anyhow::Result<PathBuf> {
    fs::create_dir_all(dir).with_context(|| format!("no se pudo crear {}", dir.display()))?;
    let cfg = &result.test.config;
    let stamp = result.timing.started_utc.replace([':', '-'], "");
    let mut path = dir.join(format!(
        "{}-{}-{}-{stamp}.json",
        cfg.module, cfg.size, cfg.mode
    ));
    let mut counter = 1;
    while path.exists() {
        path = dir.join(format!(
            "{}-{}-{}-{stamp}-{counter}.json",
            cfg.module, cfg.size, cfg.mode
        ));
        counter += 1;
    }
    let json = serde_json::to_string_pretty(result)?;
    fs::write(&path, json).with_context(|| format!("no se pudo escribir {}", path.display()))?;
    Ok(path)
}

fn capitalize(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

fn format_thousands(value: u64) -> String {
    let digits = value.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push('\u{202f}');
        }
        out.push(ch);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thousands_separator() {
        assert_eq!(format_thousands(0), "0");
        assert_eq!(format_thousands(999), "999");
        assert_eq!(format_thousands(1_000), "1\u{202f}000");
        assert_eq!(format_thousands(32_000_000), "32\u{202f}000\u{202f}000");
    }
}
