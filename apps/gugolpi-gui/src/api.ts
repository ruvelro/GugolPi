// Acceso al backend. Dentro de Tauri usa `invoke`/`listen`; en un navegador normal usa un
// simulador para poder desarrollar y revisar la interfaz sin compilar la app nativa.

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  Catalog,
  HistoryEntry,
  RunRequest,
  RunResult,
  SystemSummary,
} from "./types";

export const isTauri = "__TAURI_INTERNALS__" in window;

type Handler = (payload: unknown) => void;

class MockBus {
  private handlers = new Map<string, Set<Handler>>();
  on(event: string, handler: Handler): () => void {
    const set = this.handlers.get(event) ?? new Set();
    set.add(handler);
    this.handlers.set(event, set);
    return () => set.delete(handler);
  }
  emit(event: string, payload: unknown) {
    this.handlers.get(event)?.forEach((h) => h(payload));
  }
}

const bus = new MockBus();
const mockHistory: HistoryEntry[] = [];
let mockCancelled = false;

const mockSystem: SystemSummary = {
  system: {
    cpu_model: "CPU simulada (modo navegador)",
    physical_cores: 8,
    logical_cpus: 16,
    cpu_frequency_mhz: 4200,
    total_memory_bytes: 32 * 2 ** 30,
    available_memory_bytes: 20 * 2 ** 30,
    os_name: "Navegador",
    os_version: navigator.userAgent.slice(0, 40),
    arch: "wasm",
    simd_level: "simulado",
  },
  tool: { name: "gugolpi", version: "0.1.0", score_version: 1, commit: "mock", simd_level: "simulado" },
  results_dir: "(simulado)",
};

const mockCatalog: Catalog = {
  modules: [
    { name: "pi", title: "Pi", description: "Dígitos de Pi por Gauss–Legendre con multiplicación FFT.", official_sizes: ["16K", "32K", "64K", "128K", "256K", "512K", "1M", "2M", "4M", "8M", "16M", "32M"], extended_sizes: [], default_size: "1M", compatible_with: "SuperPi" },
    { name: "radical", title: "Radical", description: "Raíces cuadradas de 1..N por Newton–Raphson.", official_sizes: ["32M", "1024M"], extended_sizes: ["128M", "4096M"], default_size: "32M", compatible_with: "wPrime" },
    { name: "zeta", title: "Zeta", description: "Primos hasta N con criba de Eratóstenes segmentada.", official_sizes: ["1G", "10G", "100G"], extended_sizes: ["100M"], default_size: "10G", compatible_with: "propio de GugolPi" },
  ],
  suites: [
    { name: "classic", repeat: 1, jobs: ["pi 1M single", "radical 32M multi ×auto"] },
    { name: "trio", repeat: 3, jobs: ["pi 1M single", "radical 32M multi ×auto", "zeta 10G multi ×auto"] },
    { name: "full", repeat: 1, jobs: ["pi 1M single", "pi 32M single", "radical 32M multi ×auto", "zeta 10G single"] },
  ],
};

function mockResult(job: string, module: string, size: string, mode: "single" | "multi", threads: number): RunResult {
  const total = 0.5 + Math.random() * 3;
  return {
    schema_version: 1,
    tool: mockSystem.tool,
    system: mockSystem.system,
    test: { module, size: { label: size, value: 1, official: true }, mode, threads: mode === "single" ? "auto" : String(threads), affinity: false, threads_used: mode === "single" ? 1 : threads },
    timing: {
      started_utc: new Date().toISOString().replace(/\.\d+Z$/, "Z"),
      total_seconds: total,
      loops: module === "pi" ? Array.from({ length: 19 }, (_, i) => ({ index: i + 1, seconds: total / 19, cumulative_seconds: (total / 19) * (i + 1) })) : [],
      per_thread_seconds: mode === "multi" ? Array.from({ length: threads }, () => total * (0.8 + Math.random() * 0.2)) : [total],
      throughput: 1_000_000 / total,
      throughput_unit: module === "pi" ? "digits/s" : "numbers/s",
    },
    verification: { status: "passed", errors: 0, digest: "0123456789abcdef", max_fft_error: module === "pi" ? 0.0002 : undefined },
    official: true,
    integrity: job,
  };
}

async function mockSession(jobs: { label: string; module: string; size: string; mode: "single" | "multi"; threads: number }[], repeat: number) {
  mockCancelled = false;
  bus.emit("session-started", { jobs: jobs.map((j) => j.label), repeat });
  outer: for (let index = 0; index < jobs.length; index++) {
    const job = jobs[index];
    for (let run = 1; run <= repeat; run++) {
      const steps = job.module === "pi" ? 19 : 25;
      bus.emit("run-progress", { job: job.label, index, run, of: repeat, event: { kind: "started", module: job.module, total: steps, unit: "pasos" } });
      for (let s = 1; s <= steps; s++) {
        await new Promise((r) => setTimeout(r, 60));
        if (mockCancelled) break outer;
        const event = job.module === "pi"
          ? { kind: "loop", index: s, seconds: 0.06, cumulative_seconds: 0.06 * s }
          : { kind: "advanced", done: s, total: steps };
        bus.emit("run-progress", { job: job.label, index, run, of: repeat, event });
      }
      bus.emit("run-progress", { job: job.label, index, run, of: repeat, event: { kind: "verifying" } });
      await new Promise((r) => setTimeout(r, 150));
      const result = mockResult(job.label, job.module, job.size, job.mode, job.threads);
      const entry: HistoryEntry = { path: `/simulado/${job.label}-${Date.now()}.json`, file_name: `${job.label.replaceAll(" ", "-")}.json`, result, intact: true };
      mockHistory.unshift(entry);
      bus.emit("run-finished", { job: job.label, index, run, of: repeat, result, path: entry.path });
    }
  }
  bus.emit("session-finished", { cancelled: mockCancelled });
}

const mockCommands: Record<string, (args?: Record<string, unknown>) => unknown> = {
  system_summary: () => mockSystem,
  catalog: () => mockCatalog,
  start_run: (args) => {
    const request = args?.request as RunRequest;
    const label = request.mode === "single" ? `${request.module} ${request.size} single` : `${request.module} ${request.size} multi ×${request.threads}`;
    const threads = request.mode === "single" ? 1 : Number(request.threads) || 16;
    const jobs = request.scaling
      ? [1, 2, 4, 8, 16].map((n) => ({ label: `${request.module} ${request.size} multi ×${n}`, module: request.module, size: request.size, mode: "multi" as const, threads: n }))
      : [{ label, module: request.module, size: request.size, mode: request.mode, threads }];
    void mockSession(jobs, request.repeat);
    return null;
  },
  start_suite: (args) => {
    const suite = mockCatalog.suites.find((s) => s.name === args?.name);
    if (!suite) throw new Error("preset desconocido");
    const jobs = suite.jobs.map((label) => {
      const [module, size, mode] = label.split(" ");
      return { label, module, size, mode: mode as "single" | "multi", threads: 16 };
    });
    void mockSession(jobs, (args?.repeat as number | undefined) ?? suite.repeat);
    return null;
  },
  cancel_session: () => {
    mockCancelled = true;
    return null;
  },
  history: () => mockHistory,
  delete_history_entry: (args) => {
    const i = mockHistory.findIndex((e) => e.path === args?.path);
    if (i >= 0) mockHistory.splice(i, 1);
    return null;
  },
  open_results_dir: () => null,
};

export async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (isTauri) return invoke<T>(command, args);
  const fn = mockCommands[command];
  if (!fn) throw new Error(`comando simulado desconocido: ${command}`);
  return fn(args) as T;
}

export async function on<T>(event: string, handler: (payload: T) => void): Promise<() => void> {
  if (isTauri) return listen<T>(event, (e) => handler(e.payload));
  return bus.on(event, (p) => handler(p as T));
}

export const api = {
  systemSummary: () => call<SystemSummary>("system_summary"),
  catalog: () => call<Catalog>("catalog"),
  startRun: (request: RunRequest) => call<void>("start_run", { request }),
  startSuite: (name: string, repeat?: number) => call<void>("start_suite", { name, repeat: repeat ?? null }),
  cancel: () => call<void>("cancel_session"),
  history: () => call<HistoryEntry[]>("history"),
  deleteHistoryEntry: (path: string) => call<void>("delete_history_entry", { path }),
  openResultsDir: () => call<void>("open_results_dir"),
};
