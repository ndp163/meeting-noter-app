import { clsx, type ClassValue } from "clsx"
import { twMerge } from "tailwind-merge"

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}

/**
 * Turn a raw thrown value into a user-facing message. Keeps technical detail
 * out of the UI (log the raw error separately); maps the common network case.
 */
export function friendlyError(e: unknown, fallback: string): string {
  const raw = String(e).toLowerCase();
  if (/network|connection|sending request|timed out|dns|offline/.test(raw)) {
    return "Network error — check your internet connection and try again.";
  }
  return fallback;
}

/** Human-readable byte size, e.g. `479464270` → `"480 MB"`. */
export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  const units = ["KB", "MB", "GB"]
  let value = bytes / 1024
  let unit = 0
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024
    unit++
  }
  return `${value >= 100 ? Math.round(value) : value.toFixed(1)} ${units[unit]}`
}
