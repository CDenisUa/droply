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
  - `POST /api/sources/analyze` — `{ "url": "..." }` in, `MediaSourceResult`
    out (`sourceType`, `title`, `mimeType`, `sizeBytes`, `durationSeconds`).
    Currently only resolves `DirectFileAnalyzer`. Errors map through
    `ApiError` (`apps/api/src/error.rs`): `InvalidUrl`→400,
    `UnsupportedSource`→422, `SourceUnavailable`→502, `ProtectedContent`→403.
  - `POST /api/downloads` — `{ "url": "..." }` in (not doc §26's literal
    `{ sourceId, variantId }` — ADR 0006), creates a `Download` row and
    starts executing it in the background, 202 + `Download` JSON out.
  - `GET /api/downloads/:id` — current status/progress.
  - `POST /api/downloads/:id/cancel` — signals the running task; response
    reflects state *before* the task observes the signal and persists
    `Cancelled` (still `Downloading`, typically) — cancellation is
    fire-and-forget from the handler's point of view, not synchronous.
  - `POST /api/downloads/:id/retry` — only from `Failed`; re-resolves the
    source (ADR 0006) and restarts from zero (no partial-resume yet).
  - `GET /api/downloads/:id/content` — only once `status == Completed`;
    single-range `Range` support (RFC 7233 subset — no multi-range).
- **Source resolution** (doc §11): `MediaSourceAnalyzer` trait +
  `MediaSourceResolver` in `droply-application`. Analyzers are tried in
  registration order; `main.rs` (the composition root) decides which
  analyzers and which `UrlValidator` implementation are actually wired —
  `apps/api`'s `app()` function takes an already-built `AppDependencies`
  bundle rather than constructing one, so tests can inject dependencies
  backed by a permissive validator instead of the real SSRF-checking one
  (see `apps/api/tests/support/mod.rs`).
- **Download execution** (doc §12, §34): `DownloadStrategy` trait +
  `DownloadStrategyResolver` in `droply-application` (mirrors the analyzer
  resolver). `DirectFileDownloadStrategy` (`droply-infra`) streams straight
  to a temp file (`AppState::temp_storage_path`, `TEMP_STORAGE_PATH` env
  var), reporting progress via a shared `AtomicU64` rather than a callback
  — decouples "how fast bytes arrive" from "how often we persist progress".
  `apps/api/src/download_runner.rs` is the actual orchestrator: spawns a
  `tokio::spawn` task per download, races the strategy's execution future
  against a periodic progress-flush loop (`tokio::select!`), and owns every
  `DownloadStatus` transition from `Queued` onward. Cancellation is a
  `tokio_util::sync::CancellationToken` per active download, tracked in
  `AppState::active_cancellations` (a `Mutex<HashMap<Uuid, _>>` — traffic is
  low enough that this doesn't need anything fancier).
  **No formal job queue / concurrency limiting yet** — every download spawns
  its own task immediately. Doc §24/§35's `IJobQueue` + concurrency-limit-2
  is explicitly Phase 6 scope ("Advanced Download Manager"), not Phase 1.
  **No automatic temp-file cleanup yet** — files accumulate in
  `TEMP_STORAGE_PATH` until manually cleared; a cleanup sweep is a known
  follow-up, not implemented (deleting on every serve would break multi-hop
  Range requests like video seeking).

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
- Icons: real branded art in `apps/web/public/icons/droply-app-icon/`
  (48/180/192/512 + a separately-padded maskable-512, see
  `metadata.json` in that folder for source provenance).
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
