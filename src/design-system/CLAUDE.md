# Design System — rules

Modern-minimal, token-based component set. Full usage spec:
[.design-sync/conventions.md](../../.design-sync/conventions.md). Sync gotchas:
[.design-sync/NOTES.md](../../.design-sync/NOTES.md).

## Invariants

- **Presentational only** — no data fetching, no zustand, no context/providers.
  Props in, markup out.
- **Token-styled, no Tailwind.** Visuals come from CSS classes + `--ds-*` custom
  properties. Components take `className` + `style` for outside layout glue;
  callers must not restyle internals.
- Each component = `Name.tsx` + `Name.stories.tsx` (PascalCase).
- New / renamed component → export it from [index.ts](index.ts) (the public
  barrel) **and** write a story, or sync discovery misses it.

## Where styling truth lives

- [tokens.css](tokens.css) — token *values* (palette, radius, spacing, type).
  Themes remap only `--ds-*` here (`ds-theme-modern` override class).
- [components.css](components.css) — class rules.
- [styles.css](styles.css) — entry that `@import`s both.

Token groups: `--ds-bg/surface/surface-2` · `--ds-border/border-2` ·
`--ds-text/text-2/text-3` · `--ds-accent*` · `--ds-rec*`/`--ds-ok` ·
`--ds-radius*` · `--ds-shadow*` · `--ds-font`/`--ds-text-*` · `--ds-space-1…6`.

## Claude Design sync guardrails

`dist-types/` and `.design-sync/sb-reference/` are **gitignored and generated** —
they go stale. Before any DesignSync / converter run, regenerate both or
component discovery returns **0**:

```bash
# 1. type declarations (run from this dir)
../../node_modules/.bin/tsc -p tsconfig.dts.json
# 2. storybook reference (run from repo root)
npx storybook build -c .storybook -o .design-sync/sb-reference
```

- Never commit `dist-types/`, `ds-bundle/`, or `.design-sync/sb-reference/`.
- DS resolves as its own package via [package.json](package.json)
  (`meeting-noter-ds`). Don't rename/remove that — discovery breaks without it.
- Fonts are system-font-first. Don't add a font *name* without shipping its
  `@font-face` (re-adds the `[FONT_MISSING]` warning otherwise).
