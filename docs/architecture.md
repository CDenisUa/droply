# Droply — Working Architecture

Kept in sync with the actual code. If this disagrees with the code, fix this
doc, not the code. For product vision/rationale, see
[`../Droply-Architecture.md`](../Droply-Architecture.md) (vision archive) and
[`DECISIONS.md`](DECISIONS.md) (why things changed from that vision). For
what's built right now vs. planned, see [`CURRENT_STATE.md`](CURRENT_STATE.md).

## Layering

```text
crates/droply-domain        pure types, no I/O — DownloadStatus, DroplyError
     ↑
crates/droply-application   use-case orchestration, trait boundaries
     ↑                      (MediaSourceAnalyzer, DownloadStrategy, ...)
     ↑
crates/droply-infra         concrete implementations — Postgres (sqlx),
     ↑                      outbound HTTP, filesystem
     ↑
apps/api                    Axum HTTP server (droply-api binary + lib)
```

`droply-media` (FFmpeg/FFprobe wrapper) is added at Phase 4 (HLS), not
before — see AGENTS.md rule 15 (no crate before there's a concrete near-term
need for it).

Frontend follows the doc's feature-based layout (`apps/web/src/{app,pages,
features,entities,shared}`), but `features/` and `entities/` subfolders are
only created once a phase actually needs them — Phase 0 only populated
`app/`, `pages/Home/`, and `shared/{api,components,hooks,testing}`.

## Backend

- **Runtime:** Rust, Axum + Tokio, not ASP.NET Core — ADR 0001.
- **DB:** SQLx against Postgres, no ORM — ADR 0003. Migrations live in
  `/migrations` (repo root) and run automatically on startup via
  `sqlx::migrate!` in `apps/api/src/main.rs`.
- **Real-time progress:** planned as SSE, not SignalR/WebSockets — ADR 0004.
  Not implemented yet (lands with the download queue, Phase 6).
- **Errors:** `DroplyError` (thiserror) in `droply-domain` for expected
  business failures; `anyhow` only at the `apps/api` binary edge for
  unexpected technical failures. See doc §37.
- **Endpoints implemented so far:**
  - `GET /healthz` — liveness, no DB dependency, always 200 while the
    process is up.
  - `GET /readyz` — readiness, pings Postgres, 200 if reachable else 503.
  - Everything else in the original doc's §26 API section (`/api/sources/
    analyze`, `/api/downloads`, etc.) is Phase 1+, not built yet.

## Frontend

- Vite + React 19 + TypeScript (strict) + Tailwind v4 + `vite-plugin-pwa`.
- `shared/api/client.ts` — typed `fetch` wrapper reading
  `VITE_API_BASE_URL`; throws a typed `ApiError` (with HTTP status) on
  non-2xx.
- `shared/hooks/useBackendStatus.ts` — TanStack Query hook polling
  `/readyz` every 15s, used by the Home page's connectivity badge.
- State split per doc §30: TanStack Query for server state (nothing else
  needs it yet in Phase 0), Zustand installed but not yet used (no client
  state to manage until Phase 1's download UI).
- `shared/components/ChepioTechFooter/` — mandatory dev-credit footer per
  the user's global branding rule, dark-theme variant (project has no light
  theme).

## PWA

- Manifest `id: "/droply"` set explicitly (global rule: prevents iOS
  conflating this app's identity with another PWA at the same origin).
- Icons are placeholder art (`apps/web/public/icons/*`, a simple
  monogram/arrow mark) generated programmatically — not final brand design,
  swap out when real icon art exists.
- `navigateFallback: 'index.html'` for SPA offline navigation, app-shell
  precaching only (no large media in the SW cache — doc §31/§32).

## Deployment target (unchanged from the vision doc, just Rust instead of .NET)

```text
apps/web  → Cloudflare Pages (static, `npm run build` → dist/)
apps/api  → Render free Web Service (Docker, docker/api.Dockerfile)
Postgres  → Neon free tier
```

Local dev: `docker compose up -d postgres`, then `cargo run -p droply-api`
and `npm run dev` (in `apps/web`) separately — see root `README.md`.

## Testing

- Rust: `cargo test --workspace` (fast, no DB) + `cargo test --workspace --
  --include-ignored` (needs `DATABASE_URL`, used in CI with a Postgres
  service container). `cargo clippy --workspace --all-targets -- -D
  warnings` and `cargo fmt --all -- --check` gate CI.
- Frontend: `npm test` (Vitest + RTL), `npm run e2e` (Playwright, builds +
  serves `dist/` and drives it headlessly — no backend dependency for the
  Phase 0 smoke test since the UI degrades gracefully when `/readyz` fails).
- CI: `.github/workflows/ci.yml`, two jobs (`frontend`, `rust`), both
  required on `main`.
