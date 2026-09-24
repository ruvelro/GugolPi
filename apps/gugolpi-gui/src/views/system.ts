// Vista Sistema: ficha de la máquina y de la build.

import { esc, fmtBytes } from "../format";
import type { AppContext, View } from "../main";

export class SystemView implements View {
  constructor(private ctx: AppContext) {}

  mount(container: HTMLElement) {
    const { system, tool, results_dir } = this.ctx.summary;
    container.innerHTML = `
      <h1 class="page-title">Sistema</h1>
      <p class="page-sub">Esta ficha viaja dentro de cada resultado.</p>
      <div class="grid two">
        <div class="card"><h2>Máquina</h2>
          <dl class="kv">
            <dt>CPU</dt><dd>${esc(system.cpu_model)}</dd>
            <dt>Núcleos</dt><dd>${system.physical_cores} físicos · ${system.logical_cpus} lógicos</dd>
            <dt>Frecuencia</dt><dd>${system.cpu_frequency_mhz ? `${system.cpu_frequency_mhz} MHz` : "no disponible"}</dd>
            <dt>Memoria</dt><dd>${esc(fmtBytes(system.total_memory_bytes))} total · ${esc(fmtBytes(system.available_memory_bytes))} disponibles</dd>
            <dt>Sistema</dt><dd>${esc(system.os_name)} ${esc(system.os_version)} (${esc(system.arch)})</dd>
            <dt>SIMD</dt><dd>${esc(system.simd_level)}</dd>
          </dl>
        </div>
        <div class="card"><h2>GugolPi</h2>
          <dl class="kv">
            <dt>Versión</dt><dd>${esc(tool.version)}</dd>
            <dt>Commit</dt><dd class="mono">${esc(tool.commit)}</dd>
            <dt>score_version</dt><dd>${tool.score_version}</dd>
            <dt>Resultados</dt><dd class="mono">${esc(results_dir)}</dd>
          </dl>
          <p class="hint">Sólo son comparables entre sí los resultados con el mismo score_version, test, tamaño y modo, marcados como puntuables.</p>
        </div>
      </div>`;
  }
}
