// Formato de números y textos, en español.

export function fmtInt(value: number): string {
  return Math.round(value).toLocaleString("es-ES");
}

export function fmtSeconds(seconds: number, decimals = 3): string {
  return `${seconds.toLocaleString("es-ES", { minimumFractionDigits: decimals, maximumFractionDigits: decimals })} s`;
}

export function fmtBytes(bytes: number): string {
  const gib = bytes / 2 ** 30;
  return `${gib.toLocaleString("es-ES", { maximumFractionDigits: 1 })} GiB`;
}

export function fmtDate(utc: string): string {
  const date = new Date(utc);
  return Number.isNaN(date.getTime()) ? utc : date.toLocaleString("es-ES");
}

export function verificationLabel(status: string): string {
  switch (status) {
    case "passed":
      return "correcta";
    case "failed":
      return "FALLIDA";
    default:
      return "sin referencia";
  }
}

export function capitalize(text: string): string {
  return text.charAt(0).toUpperCase() + text.slice(1);
}

/** Escapa texto para insertarlo en HTML. */
export function esc(text: unknown): string {
  return String(text)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}
