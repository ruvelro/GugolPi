// Punto de entrada: navegación por vistas y arranque de la sesión de eventos.

import { api } from "./api";
import { esc } from "./format";
import { subscribe, wireSession } from "./session";
import type { Catalog, SystemSummary } from "./types";
import { HistoryView } from "./views/history";
import { RunView } from "./views/run";
import { SuitesView } from "./views/suites";
import { SystemView } from "./views/system";

export interface View {
  mount(container: HTMLElement): void;
  unmount?(): void;
}

export interface AppContext {
  catalog: Catalog;
  summary: SystemSummary;
}

const routes: { id: string; label: string; icon: string }[] = [
  { id: "run", label: "Ejecutar", icon: "▶" },
  { id: "suites", label: "Suites", icon: "☰" },
  { id: "history", label: "Historial", icon: "◷" },
  { id: "system", label: "Sistema", icon: "⚙" },
];

async function main() {
  const app = document.getElementById("app");
  if (!app) return;
  app.innerHTML = `<div class="empty">Cargando GugolPi…</div>`;
  let ctx: AppContext;
  try {
    const [catalog, summary] = await Promise.all([api.catalog(), api.systemSummary()]);
    ctx = { catalog, summary };
  } catch (err) {
    app.innerHTML = `<div class="error">No se pudo iniciar: ${esc(String(err))}</div>`;
    return;
  }
  await wireSession();

  app.innerHTML = `
    <nav class="nav">
      <div class="brand"><div class="brand-icon">π</div><div><h1>GugolPi</h1><small>v${esc(ctx.summary.tool.version)} · score ${ctx.summary.tool.score_version}</small></div></div>
      ${routes.map((r) => `<a href="#${r.id}" data-route="${r.id}"><span>${r.icon}</span>${r.label}</a>`).join("")}
      <div class="spacer"></div>
      <div class="status" id="nav-status">En reposo</div>
    </nav>
    <main class="content" id="content"></main>`;
  const content = document.getElementById("content") as HTMLElement;
  const status = document.getElementById("nav-status") as HTMLElement;
  subscribe((s) => {
    status.textContent = s.active ? `Ejecutando ${s.current?.job ?? "…"}` : "En reposo";
    status.classList.toggle("busy", s.active);
  });

  const views: Record<string, View> = {
    run: new RunView(ctx),
    suites: new SuitesView(ctx),
    history: new HistoryView(ctx),
    system: new SystemView(ctx),
  };
  let current: View | undefined;
  const navigate = () => {
    const id = location.hash.replace("#", "") || "run";
    const view = views[id] ?? views.run;
    current?.unmount?.();
    content.innerHTML = "";
    view.mount(content);
    current = view;
    document.querySelectorAll<HTMLAnchorElement>(".nav a").forEach((a) => a.classList.toggle("active", a.dataset.route === (views[id] ? id : "run")));
  };
  window.addEventListener("hashchange", navigate);
  navigate();
}

void main();
