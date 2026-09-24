// Tipos que reflejan los structs serde del núcleo (sólo los campos que usa la interfaz).

export interface SystemInfo {
  cpu_model: string;
  physical_cores: number;
  logical_cpus: number;
  cpu_frequency_mhz?: number;
  total_memory_bytes: number;
  available_memory_bytes: number;
  os_name: string;
  os_version: string;
  arch: string;
  simd_level: string;
}

export interface ToolInfo {
  name: string;
  version: string;
  score_version: number;
  commit: string;
  simd_level: string;
}

export interface SystemSummary {
  system: SystemInfo;
  tool: ToolInfo;
  results_dir: string;
}

export interface ModuleInfo {
  name: string;
  title: string;
  description: string;
  official_sizes: string[];
  extended_sizes: string[];
  default_size: string;
  compatible_with: string;
}

export interface SuiteInfo {
  name: string;
  repeat: number;
  jobs: string[];
}

export interface Catalog {
  modules: ModuleInfo[];
  suites: SuiteInfo[];
}

export interface RunRequest {
  module: string;
  size: string;
  mode: "single" | "multi";
  threads: string;
  affinity: boolean;
  repeat: number;
  static_split: boolean;
  scaling: boolean;
}

export interface LoopTiming {
  index: number;
  seconds: number;
  cumulative_seconds: number;
}

export type VerificationStatus = "passed" | "failed" | "unverified";

export interface RunResult {
  schema_version: number;
  tool: ToolInfo;
  system: SystemInfo;
  test: {
    module: string;
    size: { label: string; value: number; official: boolean };
    mode: "single" | "multi";
    threads: string;
    affinity: boolean;
    threads_used: number;
  };
  timing: {
    started_utc: string;
    total_seconds: number;
    loops?: LoopTiming[];
    per_thread_seconds?: number[];
    throughput: number;
    throughput_unit: string;
  };
  verification: {
    status: VerificationStatus;
    errors: number;
    digest?: string;
    max_fft_error?: number;
    detail?: string;
  };
  official: boolean;
  integrity?: string;
}

export type ProgressEvent =
  | { kind: "started"; module: string; total: number; unit: string }
  | { kind: "advanced"; done: number; total: number }
  | { kind: "loop"; index: number; seconds: number; cumulative_seconds: number }
  | { kind: "verifying" }
  | { kind: "finished" };

export interface RunProgress {
  job: string;
  index: number;
  run: number;
  of: number;
  event: ProgressEvent;
}

export interface RunFinished {
  job: string;
  index: number;
  run: number;
  of: number;
  result: RunResult;
  path: string;
}

export interface RunError {
  job: string;
  index: number;
  message: string;
}

export interface SessionStarted {
  jobs: string[];
  repeat: number;
}

export interface SessionFinished {
  cancelled: boolean;
}

export interface HistoryEntry {
  path: string;
  file_name: string;
  result: RunResult;
  intact: boolean;
}
