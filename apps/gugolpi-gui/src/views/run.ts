// Vista Ejecutar: elegir módulo y opciones, lanzar, ver progreso y resultado.

import { api } from "../api";
import { esc, fmtInt } from "../format";
import type { AppContext, View } from "../main";
import { sessionState, subscribe } from "../session";
import type { SessionState } from "../session";
import type { RunRequest } from "../types";
import { resultCardHtml } from "./result-card";

export class RunView implements View {
  private module: string;
  private unsubscribe?: () => void;
  private root?: HTMLElement;
  private lastError = "";

  constructor(private ctx: AppContext) {
    this.module = ctx.catalog.modules[0]?.name ?? "pi";
  }

  mount(container: HTMLElement) {
    this.root = container;
    this.renderShell();
    this.unsubscribe = subscribe((s) => this.renderLive(s));
  }

  unmount() {
    this.unsubscribe?.();
  }

  private info() {
    return this.ctx.catalog.modules.find((m) => m.name === this.module) ?? this.ctx.catalog.modules[0];
  }

  private renderShell() {
    if (!this.root) return;
    const info = this.info();
    const sizes = [...info.official_sizes.map((s) => ({ s, official: true })), ...info.extended_sizes.map((s) => ({ s, official: false }))];
    const logical = this.ctx.summary.system.logical_cpus;
    this.root.innerHTML = `
      <h1 class="page-title">Ejecutar</h1>
      <p class="page-sub">Elige un módulo y un tamaño. El modo single de Pi es la cifra comparable con SuperPi; el multi de Radical, con wPrime.</p>
      <div class="modules">
        ${this.ctx.catalog.modules.map((m) => `
          <button class="module ${m.name === this.module ? "selected" : ""}" data-module="${m.name}">
            <span class="name">${esc(m.title)}</span><span class="compat">${esc(m.compatible_with)}</span>
            <p>${esc(m.description)}</p>
          </button>`).join("")}
      </div>
      <div class="grid two">
        <div class="card">
          <h2>Opciones</h2>
          <form id="run-form" class="form">
            <label class="field">Tamaño
              <select name="size">${sizes.map(({ s, official }) => `<option value="${s}" ${s === info.default_size ? "selected" : ""}>${s}${official ? "" : " (no puntúa)"}</option>`).join("")}</select>
            </label>
            <label class="field">Modo
              <select name="mode"><option value="single" ${this.module === "pi" ? "selected" : ""}>single (1 hilo)</option><option value="multi" ${this.module !== "pi" ? "selected" : ""}>multi</option></select>
            </label>
            <label class="field">Hilos (multi)
              <select name="threads"><option value="auto">auto (${logical} lógicas)</option><option value="physical">físicos (${this.ctx.summary.system.physical_cores})</option>${[1, 2, 4, 8, 16, 32, 64].filter((n) => n <= logical).map((n) => `<option value="${n}">${n}</option>`).join("")}</select>
            </label>
            <label class="field">Repeticiones
              <input type="number" name="repeat" min="1" max="20" value="1" />
            </label>
            <label class="field check"><input type="checkbox" name="affinity" /> Afinidad por núcleo</label>
            <label class="field check"><input type="checkbox" name="scaling" /> Barrido de escalado</label>
            ${this.module === "radical" ? `<label class="field check"><input type="checkbox" name="static_split" /> Reparto estático (no puntúa)</label>` : ""}
          </form>
          <div class="actions">
            <button class="btn primary" id="btn-run">Ejecutar</button>
            <button class="btn danger" id="btn-cancel" disabled>Cancelar</button>
            <span class="hint" id="run-hint">${this.module === "pi" ? "En multi, Pi lanza una instancia independiente por hilo (SuperPi ×N)." : ""}</span>
          </div>
          <div id="run-error"></div>
        </div>
        <div class="card" id="live-card">
          <h2>En curso</h2>
          <div id="live"><div class="empty">Sin ejecución en curso.</div></div>
        </div>
      </div>
      <div class="card" id="result-card" style="margin-top:18px; display:none"></div>
    `;
    this.root.querySelectorAll<HTMLButtonElement>(".module").forEach((b) =>
      b.addEventListener("click", () => {
        if (sessionState().active) return;
        this.module = b.dataset.module ?? this.module;
        this.renderShell();
        this.renderLive(sessionState());
      }),
    );
    this.root.querySelector("#btn-run")?.addEventListener("click", () => void this.start());
    this.root.querySelector("#btn-cancel")?.addEventListener("click", () => void api.cancel());
  }

  private request(): RunRequest {
    const form = this.root?.querySelector<HTMLFormElement>("#run-form");
    const data = new FormData(form ?? undefined);
    return {
      module: this.module,
      size: String(data.get("size") ?? this.info().default_size),
      mode: data.get("mode") === "multi" ? "multi" : "single",
      threads: String(data.get("threads") ?? "auto"),
      affinity: data.get("affinity") === "on",
      repeat: Math.max(1, Number(data.get("repeat") ?? 1)),
      static_split: data.get("static_split") === "on",
      scaling: data.get("scaling") === "on",
    };
  }

  private async start() {
    this.lastError = "";
    try {
      await api.startRun(this.request());
    } catch (err) {
      this.lastError = String(err);
      this.renderLive(sessionState());
    }
  }

  private renderLive(s: SessionState) {
    if (!this.root) return;
    const live = this.root.querySelector("#live");
    const errorBox = this.root.querySelector("#run-error");
    const runBtn = this.root.querySelector<HTMLButtonElement>("#btn-run");
    const cancelBtn = this.root.querySelector<HTMLButtonElement>("#btn-cancel");
    if (runBtn) runBtn.disabled = s.active;
    if (cancelBtn) cancelBtn.disabled = !s.active;
    this.root.querySelectorAll<HTMLSelectElement | HTMLInputElement>("#run-form select, #run-form input").forEach((el) => (el.disabled = s.active));
    if (errorBox) {
      const errors = [this.lastError, ...s.errors.map((e) => `${e.job}: ${e.message}`)].filter(Boolean);
      errorBox.innerHTML = errors.length ? `<div class="error">${errors.map(esc).join("<br>")}</div>` : "";
    }
    if (live) {
      if (s.active) {
        const pct = s.progress && s.progress.total > 0 ? (s.progress.done / s.progress.total) * 100 : 0;
        const indeterminate = !s.progress || s.phase === "verifying";
        const which = s.jobs.length > 1 || s.repeat > 1 ? ` · job ${(s.current?.index ?? 0) + 1}/${s.jobs.length} · run ${s.current?.run ?? 1}/${s.repeat}` : "";
        live.innerHTML = `
          <div><strong>${esc(s.current?.job ?? s.jobs[0] ?? "")}</strong><span class="hint">${esc(which)}</span></div>
          <div class="progress ${indeterminate ? "indeterminate" : ""}"><div style="width:${pct.toFixed(1)}%"></div></div>
          <div class="hint">${s.phase === "verifying" ? "Verificando…" : s.progress ? `${fmtInt(s.progress.done)} / ${fmtInt(s.progress.total)} ${esc(s.progress.unit)}` : "Preparando…"}</div>
          <div class="log">${s.loops.map((l) => `Loop ${String(l.index).padStart(2)}: ${l.seconds.toFixed(3).padStart(9)} s  (acumulado ${l.cumulative_seconds.toFixed(3).padStart(9)} s)`).join("\n")}</div>`;
      } else if (s.cancelled) {
        live.innerHTML = `<div class="empty">Ejecución cancelada.</div>`;
      } else if (s.finished.length) {
        live.innerHTML = `<div class="empty">Terminado: ${s.finished.length} run(s) guardados en el historial.</div>`;
      } else {
        live.innerHTML = `<div class="empty">Sin ejecución en curso.</div>`;
      }
    }
    const card = this.root.querySelector<HTMLElement>("#result-card");
    if (card) {
      const last = s.finished[s.finished.length - 1];
      if (last) {
        card.style.display = "";
        const best = s.finished.length > 1 ? `<div class="hint">Mejor de ${s.finished.length}: ${Math.min(...s.finished.map((f) => f.result.timing.total_seconds)).toFixed(3)} s</div>` : "";
        card.innerHTML = resultCardHtml(last.result) + best + `<div class="hint mono">${esc(last.path)}</div>`;
      } else {
        card.style.display = "none";
      }
    }
  }
}
