// Vista Suites: presets y ejecución secuencial con tabla de resultados.

import { api } from "../api";
import { esc, fmtSeconds } from "../format";
import type { AppContext, View } from "../main";
import { sessionState, subscribe } from "../session";
import type { SessionState } from "../session";
import { badgeFor } from "./result-card";

export class SuitesView implements View {
  private unsubscribe?: () => void;
  private root?: HTMLElement;
  private lastError = "";

  constructor(private ctx: AppContext) {}

  mount(container: HTMLElement) {
    this.root = container;
    container.innerHTML = `
      <h1 class="page-title">Suites</h1>
      <p class="page-sub">Listas de tests predefinidas. Cada run se guarda en el historial; la CLI puede leer los mismos ficheros con <code>gugolpi compare</code>.</p>
      <div class="suite-list">
        ${this.ctx.catalog.suites.map((s) => `
          <div class="card">
            <h2>${esc(s.name)} <span class="hint">· ${s.jobs.length} job(s) × ${s.repeat}</span></h2>
            <ol>${s.jobs.slice(0, 8).map((j) => `<li>${esc(j)}</li>`).join("")}${s.jobs.length > 8 ? `<li>… y ${s.jobs.length - 8} más</li>` : ""}</ol>
            <button class="btn primary small" data-suite="${esc(s.name)}">Ejecutar ${esc(s.name)}</button>
          </div>`).join("")}
      </div>
      <div class="card" style="margin-top:18px">
        <h2>Progreso</h2>
        <div id="suite-live"></div>
        <div class="actions"><button class="btn danger small" id="suite-cancel" disabled>Cancelar</button></div>
      </div>`;
    container.querySelectorAll<HTMLButtonElement>("[data-suite]").forEach((b) =>
      b.addEventListener("click", () => void this.start(b.dataset.suite ?? "")),
    );
    container.querySelector("#suite-cancel")?.addEventListener("click", () => void api.cancel());
    this.unsubscribe = subscribe((s) => this.render(s));
  }

  unmount() {
    this.unsubscribe?.();
  }

  private async start(name: string) {
    this.lastError = "";
    try {
      await api.startSuite(name);
    } catch (err) {
      this.lastError = String(err);
      this.render(sessionState());
    }
  }

  private render(s: SessionState) {
    if (!this.root) return;
    const live = this.root.querySelector("#suite-live");
    const cancel = this.root.querySelector<HTMLButtonElement>("#suite-cancel");
    this.root.querySelectorAll<HTMLButtonElement>("[data-suite]").forEach((b) => (b.disabled = s.active));
    if (cancel) cancel.disabled = !s.active;
    if (!live) return;
    if (!s.jobs.length && !this.lastError) {
      live.innerHTML = `<div class="empty">Ninguna suite en marcha.</div>`;
      return;
    }
    const rows = s.jobs.map((job, index) => {
      const runs = s.finished.filter((f) => f.index === index);
      const error = s.errors.find((e) => e.index === index);
      const running = s.active && s.current?.index === index;
      const best = runs.length ? fmtSeconds(Math.min(...runs.map((r) => r.result.timing.total_seconds))) : running ? "en curso…" : error ? "error" : "pendiente";
      const status = runs.length ? badgeFor(runs.every((r) => r.result.verification.status === "passed") ? "passed" : runs.some((r) => r.result.verification.status === "failed") ? "failed" : "unverified") : error ? `<span class="badge fail">error</span>` : running ? `<span class="badge muted">en curso</span>` : "";
      return `<tr class="${running ? "selected" : ""}"><td>${index + 1}</td><td>${esc(job)}</td><td class="num">${runs.length}/${s.repeat}</td><td class="num">${esc(best)}</td><td>${status}</td></tr>`;
    });
    const pct = s.progress && s.progress.total > 0 ? (s.progress.done / s.progress.total) * 100 : 0;
    live.innerHTML = `
      ${this.lastError ? `<div class="error">${esc(this.lastError)}</div>` : ""}
      ${s.active ? `<div class="progress ${s.progress && s.phase !== "verifying" ? "" : "indeterminate"}"><div style="width:${pct.toFixed(1)}%"></div></div>` : ""}
      ${s.cancelled ? `<div class="hint">Cancelada.</div>` : ""}
      <table><thead><tr><th>#</th><th>Test</th><th class="num">Runs</th><th class="num">Mejor</th><th>Verif.</th></tr></thead><tbody>${rows.join("")}</tbody></table>
      ${s.errors.length ? `<div class="error">${s.errors.map((e) => `${esc(e.job)}: ${esc(e.message)}`).join("<br>")}</div>` : ""}`;
  }
}
