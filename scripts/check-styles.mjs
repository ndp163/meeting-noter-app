#!/usr/bin/env node
/**
 * DS-first style guard (zero-dep).
 *
 * Enforces the styling rule in CLAUDE.md: in app code, visuals must come from
 * the design system — DS components or `[var(--ds-*)]` / `var(--ds-*)` tokens.
 * Tailwind is layout-only. This fails CI when a banned colour/shadow utility
 * (raw palette, leftover shadcn-neutral token, or legacy `custom-*`) appears.
 *
 * Allowed (not flagged): `bg-[var(--ds-surface)]`, layout utils (flex/gap/...),
 * font sizes (text-sm), radius (rounded-*), borders without a colour (border-r).
 *
 * Run: `pnpm lint:styles`
 */
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";

const ROOTS = ["src/features", "src/pages"];

// Utility prefixes that carry a colour/shadow value.
const PREFIX = "bg|text|border|ring|fill|stroke|from|to|via|divide|outline|placeholder|caret|decoration|shadow";
// Tailwind named colour scales (with optional -<shade>).
const SCALE = "slate|gray|zinc|neutral|stone|red|orange|amber|yellow|lime|green|emerald|teal|cyan|sky|blue|indigo|violet|purple|fuchsia|pink|rose";
// Leftover shadcn-neutral semantic tokens (the dead scaffold palette).
const SHADCN = "card|popover|primary|secondary|muted|accent|destructive|sidebar|input|foreground|background";

// A class token: optional variants (hover:, md:, ...), prefix, value. The
// lookbehind/lookahead keep us from matching inside `[var(--ds-shadow-lg)]`.
const TOKEN = new RegExp(
  `(?<![\\w-])(?:[a-z][a-z0-9-]*:)*(${PREFIX})-([a-z0-9]+(?:-[a-z0-9]+)*(?:\\/[0-9]+)?)(?![\\w[-])`,
  "g",
);

const isBanned = (prefix, value) => {
  if (value.startsWith("custom-")) return "legacy custom-* token";
  if (/^(white|black)$/.test(value)) return "raw color";
  if (new RegExp(`^(${SCALE})(-\\d{1,3})?$`).test(value)) return "raw Tailwind palette";
  if (new RegExp(`^(${SHADCN})$`).test(value) || /^chart-\d$/.test(value))
    return "shadcn-neutral token";
  if (prefix === "shadow" && /^(sm|md|lg|xl|2xl|inner)$/.test(value))
    return "raw Tailwind shadow (use shadow-[var(--ds-shadow*)])";
  return null;
};

const walk = (dir) => {
  const out = [];
  for (const name of readdirSync(dir)) {
    const p = join(dir, name);
    if (statSync(p).isDirectory()) out.push(...walk(p));
    else if (/\.(tsx|ts)$/.test(name)) out.push(p);
  }
  return out;
};

const violations = [];
for (const root of ROOTS) {
  let files = [];
  try {
    files = walk(root);
  } catch {
    continue;
  }
  for (const file of files) {
    const lines = readFileSync(file, "utf8").split("\n");
    lines.forEach((line, i) => {
      for (const m of line.matchAll(TOKEN)) {
        const reason = isBanned(m[1], m[2]);
        if (reason) violations.push({ file, line: i + 1, cls: m[0], reason });
      }
    });
  }
}

if (violations.length === 0) {
  console.log("✓ style guard: no banned utilities (DS-first OK)");
  process.exit(0);
}

console.error(`✗ style guard: ${violations.length} banned utility class(es)\n`);
for (const v of violations) {
  console.error(`  ${v.file}:${v.line}  ${v.cls}  — ${v.reason}`);
}
console.error(
  "\nFix: use a DS component or paint with [var(--ds-*)] tokens. See CLAUDE.md → Styling.",
);
process.exit(1);
