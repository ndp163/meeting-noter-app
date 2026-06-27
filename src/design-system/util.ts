/** Minimal classname joiner — keeps the DS free of runtime deps. */
export function clsx(...parts: Array<string | false | null | undefined>): string {
  return parts.filter(Boolean).join(" ");
}
