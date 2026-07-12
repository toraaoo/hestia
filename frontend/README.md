# Hestia desktop frontend

The web UI hosted by the Tauri shell (`crates/desktop`). Implements the approved
Hestia design system — dark-only, hearth amber on near-black, Minecraft Seven/Ten
pixel type — as a Tailwind v4 theme (`src/styles/index.css`).

Screens currently render from typed mock fixtures (`src/lib/mock.ts`) shaped after
the daemon's `proto` types; wiring to `hestiad` over the client SDK is the next step
and swaps the data source, not the components.

## Stack

- React 19 + TypeScript, built with Vite (Bun as the package manager)
- Tailwind CSS v4 (CSS-first theme, no config file)
- TanStack Router (file-based routes in `src/routes/`, tree generated to
  `src/routeTree.gen.ts`)
- Zustand (`src/lib/store.ts`)
- Phosphor icons behind the `src/components/icons.ts` alias seam
- Self-hosted fonts: Minecraft Seven/Ten (OFL, from the design system) +
  Noto Sans / JetBrains Mono via Fontsource

## Layout

```
src/
├── routes/          file-based routes (screens live here)
│   ├── __root.tsx   app shell: titlebar, sidebar, play bar, overlays
│   ├── instance/    $instanceId.tsx — instance detail (tabs)
│   └── servers/     route.tsx (list rail layout) · $serverId.tsx (console)
├── components/      shared chrome (TitleBar, Sidebar, PlayBar) + ui/ primitives
├── lib/             store, mock fixtures, view-model types, helpers
├── styles/          index.css — the design-system theme (@theme tokens)
└── assets/          brand SVGs, pixel tiles, fonts (from the design system)
```

## Scripts

```bash
bun run dev           # Vite dev server (port 1420, used by `cargo tauri dev`)
bun run build         # production build + typecheck
bun run typecheck     # tsc --noEmit
bun run lint          # ESLint (typescript-eslint strict, type-checked)
bun run format        # Prettier (with Tailwind class sorting)
```

The window is frameless (`decorations: false` in `tauri.conf.json`); the in-app
titlebar provides drag + min/max/close via `@tauri-apps/api`. In a plain browser
those controls no-op, so `bun run dev` works standalone for UI work.
