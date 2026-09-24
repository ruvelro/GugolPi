// Tarjeta de resultado, compartida por las vistas.

import { capitalize, esc, fmtInt, fmtSeconds, verificationLabel } from "../format";
import type { RunResult } from "../types";

export function badgeFor(status: string): string {
  const cls = status === "passed" ? "ok" : status === "failed" ? "fail" : "warn";
  return `<span class="badge ${cls}">${esc(verificationLabel(status))}</span>`;
}

export function resultCardHtml(result: RunResult, title?: string): string {
  const t = result.test;
  const loops = result.timing.loops ?? [];
  const perThread = result.timing.per_thread_seconds ?? [];
  const spread = perThread.length > 1
    ? `min ${fmtSeconds(Math.min(...perThread))} · max ${fmtSeconds(Math.max(...perThread))}`
    : "";
  return `
    <h2>${esc(title ?? `${capitalize(t.module)} ${t.size.label} · ${t.mode} · ${t.threads_used} hilo(s)`)}</h2>
    <div class="stats">
      <div class="stat"><div class="v">${esc(fmtSeconds(result.timing.total_seconds))}</div><div class="k">Tiempo total</div></div>
      <div class="stat"><div class="v">${esc(fmtInt(result.timing.throughput))}</div><div class="k">${esc(result.timing.throughput_unit)}</div></div>
      <div class="stat"><div class="v">${badgeFor(result.verification.status)}</div><div class="k">Verificación · ${result.verification.errors} errores</div></div>
      <div class="stat"><div class="v"><span class="badge ${result.official ? "ok" : "muted"}">${result.official ? "sí" : "no"}</span></div><div class="k">Puntuable · score ${result.tool.score_version}</div></div>
    </div>
    ${spread ? `<div class="hint">Por hilo: ${esc(spread)}</div>` : ""}
    ${result.verification.max_fft_error !== undefined ? `<div class="hint">Error FFT: ${result.verification.max_fft_error.toExponential(2)} (límite 0,25)</div>` : ""}
    ${result.verification.detail ? `<div class="hint">${esc(result.verification.detail)}</div>` : ""}
    ${result.verification.digest ? `<div class="hint">Checksum: <span class="mono">${esc(result.verification.digest)}</span></div>` : ""}
    ${loops.length ? `<h3>Loops</h3><div class="log">${loops.map((l) => `Loop ${String(l.index).padStart(2)}: ${l.seconds.toFixed(3).padStart(9)} s  (acumulado ${l.cumulative_seconds.toFixed(3).padStart(9)} s)`).join("\n")}</div>` : ""}
  `;
}
