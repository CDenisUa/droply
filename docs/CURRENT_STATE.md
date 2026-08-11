# Current State

> Updated: 2026-08-11 — Phase 0 (skeleton) complete, not yet merged to `main`.

## What actually works right now

- **Rust workspace** (`cargo build --workspace`) — 4 crates: `droply-domain`,
  `droply-application` (empty, see below), `droply-infra`, `apps/api`
  (binary: `droply-api`).
- **`GET /healthz`** — liveness, no dependencies.
- **`GET /readyz`** — checks Postgres connectivity, 200/503.
- **`DownloadStatus` state machine** in `droply-domain` — full transition
  table + 6 unit tests (happy path, retry-via-Queued, pause/resume, invalid
  skip, terminal-state rejection, cancellation reachability).
- **`DroplyError`** typed error enum in `droply-domain` (doc §37's list).
- **Postgres pool wiring** in `droply-infra` (`create_pool`,
  `create_pool_with_timeout`, `ping`), bounded to 5 connections, 10s connect
  timeout.
- **Migrations pipeline wired** (`sqlx::migrate!` runs on API startup from
  `/migrations`) — directory exists but is **empty**, no schema yet (nothing
  to persist until Phase 1's `Download` entity).
- **Docker Compose** (`postgres` + `droply-api` services) and
  `docker/api.Dockerfile` (multi-stage, ~small runtime image) — written but
  **not yet verified end-to-end**: the Docker daemon wasn't running in the
  dev environment when this was built, so `docker compose up` itself hasn't
  been exercised. Everything downstream of a live Postgres *has* been
  verified via `cargo test -- --include-ignored` against a manually
  reachable DB path (the CI job does run this against a real Postgres
  service container).
- **React PWA** (`apps/web`) — Vite + React 19 + TS strict + Tailwind v4 +
  `vite-plugin-pwa`. One route: Home (`/`) — disabled paste-URL form
  (Phase 1 wires it up) + live backend connectivity badge polling
  `/readyz`.
- **`ChepioTechFooter`** — present, dark-theme variant, tested.
- **PWA installability** — manifest with unique `id`, icon set (192/512/
  512-maskable, **placeholder art**), Apple meta tags in `index.html`.
  Not yet tested on a physical iOS device.
- **Tests, all green:**
  - Rust: `cargo test --workspace` — 8 passed, 1 ignored (DB-gated).
  - Frontend unit: `npx vitest run` — 8 passed (3 files).
  - Frontend e2e: `npx playwright test` — 1 passed (shell-loads smoke test).
  - `cargo clippy --workspace --all-targets -- -D warnings` — clean.
  - `cargo fmt --all -- --check` — clean.
  - `npx oxlint` — clean.
  - `npm run build` (frontend) — succeeds, generates service worker.
- **CI** (`.github/workflows/ci.yml`) — `frontend` and `rust` jobs, the
  latter with a real Postgres service container. Not yet run on GitHub
  (repo has no remote yet) — written and locally validated command-by-
  command, but the workflow file itself hasn't executed in Actions.

## Explicitly not built yet (by design — see AGENTS.md rule 15)

- `droply-application` crate is a placeholder (doc comment only) — no
  traits defined yet (`MediaSourceAnalyzer`, `DownloadStrategy`, `UrlValidator`,
  etc. all land in Phase 1).
- No `Download`/`MediaSource`/`MediaVariant`/`LibraryItem` persistence — no
  DB schema exists yet.
- No `/api/sources/analyze`, `/api/downloads/*` routes.
- No IndexedDB / library / player / file-manager frontend features.
- No SSE progress stream (ADR 0004 — planned, not implemented).
- `droply-media` crate doesn't exist (FFmpeg lands at Phase 4).
- No GitHub remote — repo is local-only, on `feature/phase-0-skeleton`
  branch, not yet merged to `main`.

## Known follow-ups

- Verify `docker compose up` end-to-end once Docker Desktop is running
  locally (build the `droply-api` image, confirm it talks to the
  `postgres` service, confirm migrations run in-container).
- Replace placeholder PWA icon art with real Droply branding.
- Push to a GitHub remote and confirm the CI workflow actually runs green
  in Actions (only validated locally so far).

## Next planned work

Phase 1 — Direct Downloader (see `../Droply-Architecture.md` §40, Phase 1):
`UrlValidator` (SSRF protection, doc §27), `DirectFileAnalyzer`
(`MediaSourceAnalyzer` impl), `POST /api/sources/analyze`, `Download`
entity + Postgres schema, `DirectFileDownloadStrategy` (streaming, doc §28),
frontend Analyze feature wired to the (currently disabled) Home page form.
