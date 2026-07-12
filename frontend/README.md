# Hestia desktop frontend

The web UI hosted by the Tauri shell (`crates/desktop`). Implements the approved
Hestia design system — dark-only, hearth amber on near-black, Minecraft Seven/Ten
pixel type — as a Tailwind v4 theme (`src/styles/index.css`).

Screens currently render from typed mock fixtures shaped after the daemon's
`proto` types. All domain data flows through the hooks in `src/data/` — screens
never touch the fixtures or the domain store directly — so wiring `hestiad` over
the client SDK later replaces the hook internals, not the components.

## Stack

- React 19 + TypeScript, built with Vite (Bun as the package manager);
  `@/` aliases `src/`
- Tailwind CSS v4 (CSS-first theme, no config file)
- TanStack Router (file-based routes in `src/routes/`, tree generated to
  `src/routeTree.gen.ts`)
- Zustand (the global UI store in `src/stores/`, the private domain store
  behind `src/data/`)
- Phosphor icons behind the `src/components/icons.ts` alias seam
- Self-hosted fonts: Minecraft Seven/Ten (OFL, from the design system) +
  Noto Sans / JetBrains Mono via Fontsource

## Layout

```
src/
├── routes/          file-based routes — thin: each wires a feature screen
│   └── __root.tsx   app shell: titlebar, sidebar, play bar, launch overlay
├── features/        one folder per screen domain
│   ├── library/     LibraryScreen, InstanceCard, InstanceRow
│   ├── instance/    InstanceScreen, Hero, one file per tab
│   ├── servers/     ServersLayout (rail), ServerDetail (console), NoServers
│   ├── discover/    DiscoverScreen, ProjectRow
│   ├── settings/    SettingsScreen
│   └── skins/       SkinsScreen (placeholder)
├── components/
│   ├── layout/      window chrome: TitleBar, TopBar, Sidebar, PlayBar, LaunchOverlay
│   ├── ui/          design-system primitives: Button, Badge, Panel, Tabs,
│   │                PlayButton, Stat, SectionHeading, StatusDot, Tile, …
│   └── icons.ts     Phosphor aliases (the swappable icon seam)
├── data/            the data-access seam: domain hooks (useInstances, useServer,
│                    useInstanceMods, …) over a private store + mock fixtures;
│                    daemon wiring replaces these internals only
├── stores/          global client state (selection, launch overlay, view prefs)
├── lib/             view-model types, cn/format/motion/router/tiles helpers
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
