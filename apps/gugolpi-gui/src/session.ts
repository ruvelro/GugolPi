// Estado de la sesión de ejecución, alimentado por los eventos del backend.

import { on } from "./api";
import type { LoopTiming, RunError, RunFinished, RunProgress, SessionFinished, SessionStarted } from "./types";

export interface SessionState {
  active: boolean;
  jobs: string[];
  repeat: number;
  current?: { job: string; index: number; run: number; of: number };
  progress?: { done: number; total: number; unit: string };
  phase: "idle" | "running" | "verifying";
  loops: LoopTiming[];
  finished: RunFinished[];
  errors: RunError[];
  cancelled: boolean;
}

type Listener = (state: SessionState) => void;

const state: SessionState = {
  active: false,
  jobs: [],
  repeat: 1,
  phase: "idle",
  loops: [],
  finished: [],
  errors: [],
  cancelled: false,
};

const listeners = new Set<Listener>();

function notify() {
  listeners.forEach((l) => l(state));
}

export function subscribe(listener: Listener): () => void {
  listeners.add(listener);
  listener(state);
  return () => listeners.delete(listener);
}

export function sessionState(): SessionState {
  return state;
}

let wired = false;

export async function wireSession() {
  if (wired) return;
  wired = true;
  await on<SessionStarted>("session-started", (p) => {
    state.active = true;
    state.jobs = p.jobs;
    state.repeat = p.repeat;
    state.current = undefined;
    state.progress = undefined;
    state.phase = "running";
    state.loops = [];
    state.finished = [];
    state.errors = [];
    state.cancelled = false;
    notify();
  });
  await on<RunProgress>("run-progress", (p) => {
    const sameRun = state.current && state.current.index === p.index && state.current.run === p.run;
    state.current = { job: p.job, index: p.index, run: p.run, of: p.of };
    if (!sameRun) {
      state.loops = [];
      state.progress = undefined;
    }
    switch (p.event.kind) {
      case "started":
        state.phase = "running";
        state.progress = { done: 0, total: p.event.total, unit: p.event.unit };
        break;
      case "advanced":
        state.progress = { done: p.event.done, total: p.event.total, unit: state.progress?.unit ?? "" };
        break;
      case "loop":
        state.loops.push({ index: p.event.index, seconds: p.event.seconds, cumulative_seconds: p.event.cumulative_seconds });
        break;
      case "verifying":
        state.phase = "verifying";
        break;
      case "finished":
        break;
    }
    notify();
  });
  await on<RunFinished>("run-finished", (p) => {
    state.finished.push(p);
    state.phase = "running";
    notify();
  });
  await on<RunError>("run-error", (p) => {
    state.errors.push(p);
    notify();
  });
  await on<SessionFinished>("session-finished", (p) => {
    state.active = false;
    state.phase = "idle";
    state.cancelled = p.cancelled;
    state.current = undefined;
    notify();
  });
}
