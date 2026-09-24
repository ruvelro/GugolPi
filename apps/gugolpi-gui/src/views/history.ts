// Vista Historial: resultados guardados, detalle, borrado y acceso a la carpeta.

import { api } from "../api";
import { capitalize, esc, fmtDate, fmtSeconds } from "../format";
import type { AppContext, View } from "../main";
import { subscribe } from "../session";
import type { HistoryEntry } from "../types";
import { badgeFor, resultCardHtml } from "./result-card";

export class HistoryView implements View {
  private root?: HTMLElement;
  private entries: HistoryEntry[] = [];
  private selected?: string;
  private filter = "";
  private unsubscribe?: () => void;

  constructor(private ctx: AppContext) {}

  mount(container: HTMLElement) {
    this.root = container;
    container.innerHTML = `
      <h1 class="page-title">Historial</h1>
      <p class="page-sub">Resultados guardados en <span class="mono">${esc(this.ctx.summary.results_dir)}</span></p>
      <div class="actions" style="margin:0 0 14px">
        <select id="hist-filter"><option value="">Todos los módulos</option>${this.ctx.catalog.modules.map((m) => `<option value="${m.name}">${esc(m.title)}</option>`).join("")}</select>
        <button class="btn small" id="hist-refresh">Actualizar</button>
        <button class="btn small" id="hist-open">Abrir carpeta</button>
      </div>
      <div class="grid two">
        <div class="card"><div id="hist-table"></div></div>
        <div class="card" id="hist-detail"><div class="empty">Selecciona un run para ver el detalle.</div></div>
      </div>`;
    container.querySelector("#hist-refresh")?.addEventListener("click", () => void this.load());
    container.querySelector("#hist-open")?.addEventListener("click", () => void api.openResultsDir());
    container.querySelector<HTMLSelectElement>("#hist-filter")?.addEventListener("change", (e) => {
      this.filter = (e.target as HTMLSelectElement).value;
      this.render();
    });
    let wasActive = false;
    this.unsubscribe = subscribe((s) => {
      if (wasActive && !s.active) void this.load();
      wasActive = s.active;
    });
    void this.load();
  }

  unmount() {
    this.unsubscribe?.();
  }

  private async load() {
    try {
      this.entries = await api.history();
    } catch (err) {
      this.entries = [];
      this.root?.querySelector("#hist-table")?.replaceChildren(Object.assign(document.createElement("div"), { className: "error", textContent: String(err) }));
      return;
    }
    this.render();
  }

  private render() {
    const table = this.root?.querySelector("#hist-table");
    if (!table) return;
    const rows = this.entries.filter((e) => !this.filter || e.result.test.module === this.filter);
    if (!rows.length) {
      table.innerHTML = `<div class="empty">Todavía no hay resultados.</div>`;
      this.renderDetail();
      return;
    }
    table.innerHTML = `<table><thead><tr><th>Fecha</th><th>Test</th><th class="num">Hilos</th><th class="num">Tiempo</th><th>Verif.</th></tr></thead><tbody>
      ${rows.map((e) => `<tr class="clickable ${e.path === this.selected ? "selected" : ""}" data-path="${esc(e.path)}">
        <td>${esc(fmtDate(e.result.timing.started_utc))}</td>
        <td>${esc(capitalize(e.result.test.module))} ${esc(e.result.test.size.label)} ${esc(e.result.test.mode)}${e.result.official ? "" : ' <span class="badge muted">no puntúa</span>'}</td>
        <td class="num">${e.result.test.threads_used}</td>
        <td class="num">${esc(fmtSeconds(e.result.timing.total_seconds))}</td>
        <td>${badgeFor(e.result.verification.status)}${e.intact ? "" : ' <span class="badge fail">manipulado</span>'}</td>
      </tr>`).join("")}</tbody></table>`;
    table.querySelectorAll<HTMLTableRowElement>("tr[data-path]").forEach((tr) =>
      tr.addEventListener("click", () => {
        this.selected = tr.dataset.path;
        this.render();
      }),
    );
    this.renderDetail();
  }

  private renderDetail() {
    const detail = this.root?.querySelector("#hist-detail");
    if (!detail) return;
    const entry = this.entries.find((e) => e.path === this.selected);
    if (!entry) {
      detail.innerHTML = `<div class="empty">Selecciona un run para ver el detalle.</div>`;
      return;
    }
    detail.innerHTML = `${resultCardHtml(entry.result)}
      <h3>Máquina</h3>
      <div class="hint">${esc(entry.result.system.cpu_model)} · ${entry.result.system.physical_cores} núcleos / ${entry.result.system.logical_cpus} hilos · ${esc(entry.result.system.os_name)} ${esc(entry.result.system.os_version)} · ${esc(entry.result.tool.simd_level)}</div>
      <div class="hint mono">${esc(entry.file_name)}${entry.intact ? " · íntegro" : " · HASH NO COINCIDE"}</div>
      <div class="actions"><button class="btn danger small" id="hist-delete">Borrar este resultado</button></div>`;
    detail.querySelector("#hist-delete")?.addEventListener("click", async () => {
      if (!confirm("¿Borrar este resultado del historial?")) return;
      await api.deleteHistoryEntry(entry.path);
      this.selected = undefined;
      await this.load();
    });
  }
}
