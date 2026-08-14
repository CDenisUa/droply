# Current State

> Updated: 2026-08-14 — Phase 1 (smallest complete vertical slice) complete
> and merged to `main`, verified end-to-end through the actual browser UI.

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
  `/migrations`) — `0001_create_downloads.sql` adds the `downloads` table
  (see Phase 1c below).
- **Docker Compose** (`postgres` + `droply-api` services) and
  `docker/api.Dockerfile` (multi-stage, ~small runtime image) — **verified
  end-to-end**: `docker compose up -d --build` builds the API image, starts
  both containers, `droply-api` connects to `postgres` over the compose
  network, runs migrations, and `/healthz`/`/readyz` both return 200 through
  the mapped host port. Host ports are non-default because this machine runs
  other local projects' containers: Postgres is on **5434** (not 5432/5433,
  already taken), the API is on **8082** (not 8080/8081, already taken). The
  in-container `DATABASE_URL` (`postgres:5432`, the compose-network hostname)
  is unaffected by the host remapping. `cargo test -- --include-ignored` also
  passes against this live container.
- **React PWA** (`apps/web`) — Vite + React 19 + TS strict + Tailwind v4 +
  `vite-plugin-pwa`. One route: Home (`/`) — disabled paste-URL form
  (Phase 1 wires it up) + live backend connectivity badge polling
  `/readyz`.
- **`ChepioTechFooter`** — present, dark-theme variant, tested.
- **PWA installability** — manifest with unique `id`, real branded icon set
  (`apps/web/public/icons/droply-app-icon/`: 48/180/192/512 + maskable-512,
  sourced from a user-uploaded 1254×1254 image), Apple meta tags in
  `index.html`. The maskable variant is a distinct, separately-padded file
  (icon content scaled to ~80% and centered) — the auto-generated asset
  pipeline initially emitted the same image for both `any` and `maskable`
  purpose entries, which is wrong (maskable needs safe-zone padding so
  Android's circular/squircle crop doesn't clip the artwork); regenerated
  it correctly with ImageMagick before committing. Not yet tested on a
  physical iOS device.
- **Tests, all green:**
  - Rust: `cargo test --workspace` — 8 passed, 1 ignored (DB-gated).
  - Frontend unit: `npx vitest run` — 8 passed (3 files).
  - Frontend e2e: `npx playwright test` — 1 passed (shell-loads smoke test).
  - `cargo clippy --workspace --all-targets -- -D warnings` — clean.
  - `cargo fmt --all -- --check` — clean.
  - `npx oxlint` — clean.
  - `npm run build` (frontend) — succeeds, generates service worker.
- **No CI workflow file exists yet** (`.github/workflows/` is absent) —
  this doc previously claimed a `ci.yml` had been written; that was
  inaccurate, corrected here. `cargo build/clippy/fmt/test` and
  `npm run lint/typecheck/test/build` are run manually per AGENTS.md rule
  20, not automated in Actions yet. A GitHub remote (`origin`) now exists
  and is reachable, but nothing had been pushed until this Phase 1 merge.

## Phase 1 progress (in flight)

- **`UrlValidator`** (SSRF protection, doc §27) — trait in
  `droply-application`, implementation `SsrfSafeUrlValidator` in
  `droply-infra`. Blocks non-http(s) schemes, `localhost`, loopback,
  private ranges (10/8, 172.16/12, 192.168/16), link-local (covers the
  169.254.169.254 AWS/GCP/Azure/DO metadata endpoint), shared address space
  100.64.0.0/10 (covers Alibaba Cloud's 100.100.100.200 metadata endpoint),
  IPv6 loopback/unique-local/link-local. Resolves DNS and checks **every**
  returned address, not just the first (defends against DNS rebinding).
  **Now wired in**: `DirectFileAnalyzer` validates the initial URL and
  re-validates every redirect hop before following it.
- **`MediaSourceAnalyzer` trait + `MediaSourceResolver`** (`droply-application`)
  — tries registered analyzers in order, first match wins. Currently one
  analyzer registered: `DirectFileAnalyzer`.
- **`DirectFileAnalyzer`** (`droply-infra`) — HEAD first, GET fallback (body
  never read) when HEAD isn't supported; manual redirect loop (max 5 hops,
  each re-validated); declines `.m3u8`/`.mpd` paths so they get an honest
  `UnsupportedSource` instead of being mis-labeled as a direct file (real
  `HlsAnalyzer`/`DashAnalyzer` land at Phase 4/5). Extracts
  Content-Type/Content-Length/Content-Disposition; filename derivation
  (`derive_filename`, `droply-domain`) prefers Content-Disposition over the
  URL path, sanitizes path separators/`..`/control chars/length.
- **`POST /api/sources/analyze`** — parses and delegates to the resolver;
  `DroplyError` variants map to HTTP status via `ApiError`
  (`InvalidUrl`→400, `UnsupportedSource`→422, `SourceUnavailable`→502,
  `ProtectedContent`→403, ...).
- **Manually verified against the real internet** (not just mocks): a real
  `raw.githubusercontent.com` URL analyzes correctly over real DNS+TLS;
  `169.254.169.254` (cloud metadata) and `localhost` are both correctly
  rejected with 400 through the live server.
- 24 new tests this slice (14 domain filename tests, 2 resolver tests, 6
  analyzer tests via `wiremock`, 4 endpoint integration tests, on top of the
  existing 9 `UrlValidator` tests) — all fast/deterministic, no live DNS or
  real network calls in the automated suite (only in the one-off manual
  check above).

- **`Download` entity + Postgres persistence** (Phase 1c) — `Download`
  struct in `droply-domain` (id, source_url, file_name, media_type, status,
  bytes_downloaded, total_bytes, created_at/started_at/completed_at, error)
  with domain methods (`transition`, `record_progress`, `fail`) that keep
  timestamps consistent — `started_at` is set once on first entry to
  `Downloading` (resuming from `Paused` doesn't reset it), `completed_at`
  on reaching any terminal status. `DownloadStatus::as_str`/`parse` give a
  stable TEXT-column mapping (not a Postgres native enum, so a new status
  never needs a migration). `DownloadRepository` trait
  (`droply-application`: create/find_by_id/update/list_recent — deliberately
  not a generic `Repository<T>`, see AGENTS.md rule 16) implemented by
  `PostgresDownloadRepository` (`droply-infra`) using **runtime-checked**
  `sqlx::query_as` + `#[derive(FromRow)]`, not the `query!`/`query_as!`
  macros — those need a live, already-migrated DB at *compile* time, which
  would break `cargo build` for anyone without Postgres running.
  `migrations/0001_create_downloads.sql` adds the table + two indexes
  (status, created_at DESC for the History view). 8 new tests (4 pure
  domain, 4 DB-gated repository round-trip tests) — the repository tests
  were run against the live Docker Postgres and confirmed passing, not just
  written.

- **Streaming downloads end-to-end** (Phase 1d) — `DownloadStrategy` trait +
  `DownloadStrategyResolver` (`droply-application`, mirrors the analyzer
  resolver). `DirectFileDownloadStrategy` (`droply-infra`) streams a direct
  file straight to a temp file, chunk by chunk, never buffering the whole
  thing (AGENTS.md rule 7-8); shares the `request_with_redirects` helper
  with `DirectFileAnalyzer` (extracted to `droply-infra::http`, so the
  SSRF-relevant redirect-revalidation logic exists in exactly one place).
  `apps/api/src/download_runner.rs` orchestrates: spawns a `tokio` task per
  download, walks `DownloadStatus` from `Pending`/`Queued` through to
  `Downloading` (`advance_to_downloading`, handles both a fresh create and
  a post-retry `Queued` start), races the strategy's execution against a
  500ms progress-flush loop, and persists the final `Completed`/
  `Cancelled`/`Failed` state. Cancellation: a `CancellationToken` per
  active download in `AppState::active_cancellations`
  (`Mutex<HashMap<Uuid, _>>`).
  **Endpoints:** `POST /api/downloads` (`{url}`, not doc's `{sourceId,
  variantId}` — see ADR 0006), `GET /api/downloads/:id`, `POST
  /api/downloads/:id/cancel`, `POST /api/downloads/:id/retry` (only from
  `Failed`, restarts from zero), `GET /api/downloads/:id/content`
  (single-range `Range` support, only once `Completed`).
  Fixed a real bug caught while writing the first content test: "download
  not found" was mapped to `SourceUnavailable` (502) instead of a proper
  404 — added a `DroplyError::NotFound` variant rather than reusing an
  unrelated error for the wrong HTTP status.
  **31 new tests** (11 Range-parsing unit tests, 4 `DirectFileDownloadStrategy`
  tests via `wiremock` including a real cancellation-mid-stream case, 2
  `DownloadStrategyResolver` tests, 2 new `Download` domain tests for
  `retry()`, 6 `/api/downloads` integration tests covering create→complete→
  serve, not-ready-yet content, 404, cancel-while-downloading, retry-after-
  failure with a mock that fails once then succeeds, and retry-rejected-
  when-not-failed) — stable across repeated runs despite being
  concurrency/timing-sensitive (spawned background tasks, cancellation
  races), not just passing once.
  **Manually verified against the real internet**: created a download of a
  real GitHub-hosted file, polled it through `pending → downloading →
  completed`, served the content back (byte-for-byte correct), served a
  `Range: bytes=0-4` partial request (206, correct bytes), confirmed
  cancel-after-complete and retry-after-complete both correctly no-op/reject,
  confirmed unknown IDs 404.

- **Frontend Analyze + Downloads flow (Phase 1e)** — the last piece of
  Phase 1's "smallest complete vertical slice," wiring the previously-inert
  Home page form to the backend and adding a Downloads view.
  - `AppNav` (`apps/web/src/app/AppNav.tsx`) — top nav with Home/Downloads
    links.
  - `apps/web/src/entities/download/types.ts` — frontend `Download`/
    `DownloadStatus` types mirroring the backend DTO, plus
    `ACTIVE_DOWNLOAD_STATUSES` (used to decide when to keep polling and
    whether Cancel is meaningful).
  - `features/source-analyzer` — `analyzeSource` (`POST
    /api/sources/analyze`) + `useAnalyzeSource` mutation hook.
  - `features/downloads` — `downloadsApi` (create/list/get/cancel/retry/
    content-URL), `useCreateDownload`, `useDownloadActions`
    (cancel + retry), `useDownloadsList` (polls `GET /api/downloads` every
    1s only while at least one download is in an active status, per
    ADR 0004 — no SSE yet), and `DownloadItem`/`DownloadsList` components.
  - `pages/Downloads` — renders `DownloadsList`; reachable via the nav.
  - Home page: paste-URL form now calls `useAnalyzeSource`, shows the
    result (title/mime/size via `formatBytes`), and a "Start Download"
    button that calls `useCreateDownload` and navigates to `/downloads`.
  - `shared/utils/formatBytes`, `shared/utils/describeApiError` (shared
    by Home's analyze/create-download errors and `DownloadItem`'s
    cancel/retry errors — turns a typed `ApiError` into its message, falls
    back to a caller-supplied generic string for non-`ApiError` failures).
  - **A real bug found only by manual browser verification, not by any
    automated test**: the backend's `CorsLayer` set `allow_methods` but
    never `allow_headers`, so the browser's preflight `OPTIONS` request for
    any JSON `POST` (analyze, create-download, ...) was rejected — `Content-
    Type: application/json` isn't a CORS-safelisted header. Every existing
    test (Rust integration tests via `oneshot`, frontend tests that mock
    `fetch`) bypasses real preflight entirely, and prior manual checks used
    `curl`, which also skips it — so this was invisible until an actual
    browser hit the actual backend. Fixed by adding
    `.allow_headers([CONTENT_TYPE])` in `cors_layer_from_env`
    (`apps/api/src/lib.rs`); guarded against regressing with a new
    `apps/api/tests/cors.rs` integration test that sends a real preflight
    request and asserts `Content-Type` comes back in
    `Access-Control-Allow-Headers`.
  - **Manually verified against the real internet through the browser**
    (Playwright-driven, not just `curl`): analyzed a real
    `raw.githubusercontent.com` file, started the download, watched it
    reach `Completed` on the Downloads page, confirmed the "Open" link
    serves byte-correct content with the right `Content-Disposition`, and
    exercised Cancel/Retry against the live backend.
  - 12 new/updated frontend test files (component tests for `HomePage`,
    `DownloadItem`, `DownloadsList`, unit tests for `formatBytes`,
    `describeApiError`, `client.ts`) — 30 unit tests total, all passing;
    `e2e/smoke.spec.ts` extended to cover the Downloads nav link.

## Explicitly not built yet (by design — see AGENTS.md rule 15)

- No formal job queue / concurrency limiting — every download spawns
  immediately, no cap on simultaneous downloads. Doc §24/§35's `IJobQueue`
  + concurrency-limit-2 is explicitly Phase 6 ("Advanced Download Manager"),
  not Phase 1.
- No automatic temp-file cleanup — files accumulate in `TEMP_STORAGE_PATH`
  until manually cleared. Deferred rather than half-built (deleting on
  every serve would break multi-request Range access, e.g. video seeking).
- No partial-resume — `retry()` always restarts from byte zero.
- No IndexedDB / library / player / file-manager frontend features.
- No SSE progress stream (ADR 0004 — planned, not implemented); frontend
  will need to poll `GET /api/downloads/:id` for now.
- `droply-media` crate doesn't exist (FFmpeg lands at Phase 4).
- No GitHub remote — repo is local-only.

## Known follow-ups

- No CI workflow exists yet — write `.github/workflows/ci.yml` (frontend +
  rust jobs) and confirm it runs green in Actions now that `origin` has
  commits.
- Temp-storage cleanup sweep (see above).
- Downloads left `Pending`/`Downloading` when the API process restarts are
  orphaned forever — the owning `tokio` task (and its
  `active_cancellations` entry) is gone, so `cancel` silently no-ops and
  nothing ever resumes or fails them. Observed directly against the local
  Docker Postgres volume across container restarts. Not a regression in
  Phase 1e's code; it's inherent to "every download spawns immediately, no
  queue" (see below) — recovering/reaping orphaned rows on startup belongs
  with the Phase 6 job queue, not before.

## Next planned work

Phase 1 is now a complete, browser-verified vertical slice: paste a URL →
analyze → start a direct-file download → watch it complete on the
Downloads page → open the result. Next up is whatever Phase 2 of
`Droply-Architecture.md` covers (not yet scoped in this doc) — plus the CI
workflow follow-up above before that work starts piling up unverified in
Actions.
