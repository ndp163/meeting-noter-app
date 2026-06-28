# Meeting Noter — Claude Code rules

Tauri 2 desktop app (macOS), **offline/local** meeting transcription. React/TS
frontend, Rust backend, Swift `FluidAudioBridge` (ASR / VAD / diarization).
No data leaves the machine — never add network/telemetry without explicit ask.

Architecture: see [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).
Domain-specific rules live in nested files — read them when working there:
- [src/design-system/CLAUDE.md](src/design-system/CLAUDE.md) — DS + Claude Design sync
- [src-tauri/CLAUDE.md](src-tauri/CLAUDE.md) — Rust / FFI / audio pipeline

## Stack

React 19 · TypeScript (strict) · Vite · Tailwind 4 · zustand · react-router 7 ·
lucide-react · Tauri 2.

## Commands

**pnpm only** (repo is a pnpm workspace — never npm/yarn).

| Task | Command |
|---|---|
| Frontend dev | `pnpm dev` |
| Full app dev | `pnpm tauri dev` |
| Build | `pnpm build` (`tsc && vite build`) |
| Storybook | `pnpm storybook` |
| Style guard | `pnpm lint:styles` (also runs inside `pnpm build`) |
| Rust | `cargo …` inside `src-tauri/` |

## Layout

| Dir | Holds |
|---|---|
| `src/features/<feature>/` | Feature UI — files **kebab-case** (`main-content.tsx`) |
| `src/design-system/` | DS components — **PascalCase** (`Button.tsx`). Token-styled, no Tailwind |
| `src/services/` | Tauri `invoke` wrappers (one file per backend domain) |
| `src/store/` | zustand slices (`*.slice.ts`) bound in `index.ts` |
| `src/types/` | Shared TS types |
| `src/pages/` | Route entries |

## Styling — DS-first (the core rule)

This app has **two style vocabularies with a hard boundary**. Do not mix them.
`pnpm lint:styles` enforces this — it fails on any banned utility.

1. **All visuals go through the Design System.** Colours, borders, radius,
   surfaces, shadows, typography → use a DS component, or paint with `--ds-*`
   tokens. Two equivalent ways to apply a token:
   - inline style: `style={{ background: "var(--ds-surface)" }}`
   - Tailwind arbitrary value (good inside `cn()` conditionals):
     `bg-[var(--ds-surface)]`, `text-[var(--ds-text-2)]`, `shadow-[var(--ds-shadow)]`
   Token list: [src/design-system/CLAUDE.md](src/design-system/CLAUDE.md).
2. **Tailwind is for LAYOUT GLUE ONLY** — flex/grid, gap, padding/margin,
   sizing, positioning, font-size (`text-sm`). Nothing that paints colour.
3. **Banned (lint fails):** raw palette (`bg-white`, `text-gray-500`,
   `shadow-md`), leftover shadcn-neutral tokens (`bg-primary`, `text-secondary`),
   and legacy `custom-*` tokens (`text-custom-red`) — all removed, keep them out.
4. Never restyle DS component internals. Add layout glue on the **outside**.
5. Exempt: colours passed to a **JS API** that can't take a CSS var (e.g.
   wavesurfer `waveColor`). Mirror the token's hex and leave a comment.

```tsx
// GOOD — DS component + Tailwind layout; token via style or arbitrary value
<div className="flex flex-col gap-4">
  <Button variant="primary">Save</Button>
  <div className={cn("p-4", active ? "bg-[var(--ds-border)]" : "hover:bg-[var(--ds-surface-2)]")} />
</div>

// BAD — Tailwind painting visuals
<div className="bg-white border border-gray-200 rounded-xl shadow-md">…</div>
```

App-level base styles live in [src/styles/App.css](src/styles/App.css) — DS-only
(no Tailwind colour theme, no shadcn scaffold). Don't reintroduce a palette there.

## No shadcn

Do **not** run `npx shadcn add` or add shadcn/ui primitives — the scaffold was
removed (no `components.json`). The Design System is the only component source.
Need a primitive DS lacks (Dialog, Tooltip…)? Build it in `src/design-system/`
on `--ds-*` tokens — ask before adding deps.

## Conventions

- TypeScript strict; **no `any`** — model real types in `src/types/`.
- Import DS via the `@/design-system` barrel, not deep paths.
- Backend calls go through a `src/services/` wrapper, never raw `invoke` in components.
- UI state in zustand slices, not prop-drilled globals.

## Public repo — keep private info OUT

**This repository is public.** Everything committed (code, docs, commit
messages, history) is world-readable forever. Before writing anything into the
repo, assume it will be read by competitors and the public.

Never commit into this repo:
- **Secrets** — API keys, tokens, AWS/cloud creds, signing/private keys,
  `.env` files. Secrets belong in GitHub Actions secrets / a vault, referenced
  as `${{ secrets.X }}`.
- **Business strategy** — monetization, pricing, revenue plans, go-to-market,
  competitive analysis.
- **Anti-piracy / license-key internals** — key formats, signing scheme,
  threat model. Publishing the lock helps people pick it.
- **Paid-feature source** — paid code lives in the separate private repo, not
  here (open-core: this repo is the free core only).
- **PII / personal or employer data** — real names tied to private email,
  internal company info, customer data.
- **Real recordings** — no `.wav`/audio from actual meetings (privacy + it
  contradicts the offline promise).

Private/strategy docs live **outside the repo** (`~/meeting-noter-notes/` and
the future private `meeting-noter-pro` repo). If asked to write strategy,
pricing, or license-key design, put it there — not in the repo.

When unsure whether something is safe to commit, **ask first.**

## Git

- Branch before editing on `main`; conventional commits (`feat:`, `fix:`, `chore:`).
- Commit/push **only when asked**. Confirm before any outward-facing action.
