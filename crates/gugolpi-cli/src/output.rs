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
    let status = match verification.status {
        gugolpi_core::result::VerificationStatus::Passed => "correcta",
        gugolpi_core::result::VerificationStatus::Failed => "FALLIDA",
        gugolpi_core::result::VerificationStatus::Unverified => "sin referencia",
    };
    println!("  Verificación: {status} ({} errores)", verification.errors);
    if let Some(detail) = &verification.detail {
        println!("                {detail}");
    }
    if let Some(err) = verification.max_fft_error {
        println!(
            "  Error FFT:    {err:.4} (límite {})",
            gugolpi_core::score::FFT_ROUNDING_LIMIT
        );
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
    Ok(result.save_to_dir(dir)?)
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

/// Tabla del barrido de escalado: mejor tiempo por número de hilos y speedup respecto a 1 hilo.
pub fn print_scaling_table(rows: &[(gugolpi_core::suite::Job, Vec<RunResult>)]) {
    let Some(first) = rows.first().and_then(|(_, r)| best_time(r)) else {
        return;
    };
    println!();
    println!("Escalado");
    println!(
        "  {:>6}  {:>12}  {:>18}  {:>8}  {:>10}",
        "Hilos", "Tiempo (s)", "Throughput", "Speedup", "Eficiencia"
    );
    for (_, results) in rows {
        let Some(best) = best_time(results) else {
            continue;
        };
        let Some(sample) = results.first() else {
            continue;
        };
        let threads = sample.test.threads_used;
        let speedup = first / best;
        let efficiency = speedup / threads as f64 * 100.0;
        println!(
            "  {threads:>6}  {best:>12.3}  {:>18}  {speedup:>7.2}x  {efficiency:>8.1} %",
            format!(
                "{} {}",
                format_thousands(sample.timing.throughput.round() as u64),
                sample.timing.throughput_unit
            )
        );
    }
}

/// Mejor tiempo total de una lista de repeticiones.
pub fn best_time(results: &[RunResult]) -> Option<f64> {
    results
        .iter()
        .map(|r| r.timing.total_seconds)
        .reduce(f64::min)
}

fn mean_time(results: &[RunResult]) -> Option<f64> {
    if results.is_empty() {
        return None;
    }
    Some(results.iter().map(|r| r.timing.total_seconds).sum::<f64>() / results.len() as f64)
}

fn verification_label(result: &RunResult) -> &'static str {
    match result.verification.status {
        gugolpi_core::result::VerificationStatus::Passed => "correcta",
        gugolpi_core::result::VerificationStatus::Failed => "FALLIDA",
        gugolpi_core::result::VerificationStatus::Unverified => "sin ref.",
    }
}

/// Tabla final de una suite.
pub fn print_suite_table(name: &str, rows: &[(gugolpi_core::suite::Job, Vec<RunResult>)]) {
    println!();
    println!("Suite «{name}»");
    println!(
        "  {:<28}  {:>10}  {:>10}  {:>10}  {:>9}",
        "Test", "Mejor (s)", "Media (s)", "Verif.", "Oficial"
    );
    for (job, results) in rows {
        let (Some(best), Some(mean), Some(sample)) =
            (best_time(results), mean_time(results), results.first())
        else {
            continue;
        };
        println!(
            "  {:<28}  {best:>10.3}  {mean:>10.3}  {:>10}  {:>9}",
            truncate(&job.label, 28),
            verification_label(sample),
            if results.iter().all(|r| r.official) {
                "sí"
            } else {
                "no"
            }
        );
    }
}

/// Escribe el `summary.csv` de una suite: una fila por run.
pub fn write_suite_csv(
    path: &Path,
    suite: &str,
    rows: &[(gugolpi_core::suite::Job, Vec<RunResult>)],
) -> anyhow::Result<()> {
    let mut text = String::from(
        "suite,test,module,size,mode,threads,run,total_seconds,throughput,throughput_unit,verification,official,score_version,cpu\n",
    );
    for (job, results) in rows {
        for (i, r) in results.iter().enumerate() {
            let cfg = &r.test.config;
            text.push_str(&csv_row(&[
                suite,
                &job.label,
                cfg.module.name(),
                &cfg.size.label,
                &cfg.mode.to_string(),
                &r.test.threads_used.to_string(),
                &(i + 1).to_string(),
                &format!("{:.6}", r.timing.total_seconds),
                &format!("{:.0}", r.timing.throughput),
                &r.timing.throughput_unit,
                verification_label(r),
                if r.official { "true" } else { "false" },
                &r.tool.score_version.to_string(),
                &r.system.cpu_model,
            ]));
        }
    }
    fs::write(path, text).with_context(|| format!("no se pudo escribir {}", path.display()))?;
    Ok(())
}

/// Tabla de `compare`, un bloque por clave de comparabilidad.
pub fn print_compare_table(
    groups: &std::collections::BTreeMap<String, Vec<&crate::commands::compare::Loaded>>,
) {
    if groups.is_empty() {
        println!("No hay resultados oficiales que comparar.");
        return;
    }
    for (key, items) in groups {
        let versions: std::collections::BTreeSet<u32> =
            items.iter().map(|l| l.result.tool.score_version).collect();
        println!();
        print!("{key}");
        if versions.len() > 1 {
            print!("  (¡score_version distintas: no comparables entre sí!)");
        }
        println!();
        println!(
            "  {:<30}  {:<32}  {:>5}  {:>10}  {:>9}  {:>8}  {:>7}  {:>6}",
            "Fichero", "CPU", "Hilos", "Tiempo (s)", "Verif.", "Íntegro", "Score", "SIMD"
        );
        for l in items {
            let r = &l.result;
            println!(
                "  {:<30}  {:<32}  {:>5}  {:>10.3}  {:>9}  {:>8}  {:>7}  {:>6}",
                truncate(
                    &l.path
                        .file_name()
                        .map(|f| f.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                    30
                ),
                truncate(&r.system.cpu_model, 32),
                r.test.threads_used,
                r.timing.total_seconds,
                verification_label(r),
                if l.intact { "sí" } else { "NO" },
                r.tool.score_version,
                r.tool.simd_level,
            );
        }
    }
}

/// `compare --csv`.
pub fn print_compare_csv(
    groups: &std::collections::BTreeMap<String, Vec<&crate::commands::compare::Loaded>>,
) -> anyhow::Result<()> {
    let mut out = std::io::stdout().lock();
    writeln!(
        out,
        "group,file,cpu,threads,total_seconds,throughput,verification,official,intact,score_version,simd,started_utc"
    )?;
    for (key, items) in groups {
        for l in items {
            let r = &l.result;
            write!(
                out,
                "{}",
                csv_row(&[
                    key,
                    &l.path.display().to_string(),
                    &r.system.cpu_model,
                    &r.test.threads_used.to_string(),
                    &format!("{:.6}", r.timing.total_seconds),
                    &format!("{:.0}", r.timing.throughput),
                    verification_label(r),
                    if r.official { "true" } else { "false" },
                    if l.intact { "true" } else { "false" },
                    &r.tool.score_version.to_string(),
                    &r.tool.simd_level,
                    &r.timing.started_utc,
                ])
            )?;
        }
    }
    Ok(())
}

fn csv_row(fields: &[&str]) -> String {
    let mut row = String::new();
    for (i, field) in fields.iter().enumerate() {
        if i > 0 {
            row.push(',');
        }
        if field.contains([',', '"', '\n']) {
            row.push('"');
            row.push_str(&field.replace('"', "\"\""));
            row.push('"');
        } else {
            row.push_str(field);
        }
    }
    row.push('\n');
    row
}

fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        text.to_owned()
    } else {
        let mut cut: String = text.chars().take(max.saturating_sub(1)).collect();
        cut.push('…');
        cut
    }
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
