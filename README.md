# Droply

An installable PWA for downloading, organizing, storing, browsing, and
playing files on your own device.

- Product vision & original spec: [`Droply-Architecture.md`](Droply-Architecture.md)
- Working architecture (kept in sync with the code): [`docs/architecture.md`](docs/architecture.md)
- What's actually built right now: [`docs/CURRENT_STATE.md`](docs/CURRENT_STATE.md)
- Why things changed from the original spec: [`docs/DECISIONS.md`](docs/DECISIONS.md)
- Rules for coding agents working in this repo: [`AGENTS.md`](AGENTS.md)

## Stack

- **Backend:** Rust — Axum, Tokio, SQLx, PostgreSQL (see ADR 0001-0004 for
  why this differs from the original ASP.NET Core spec).
- **Frontend:** React 19, TypeScript (strict), Vite, Tailwind CSS v4,
  `vite-plugin-pwa`, TanStack Query, Zustand, React Router.
- **Testing:** `cargo test`/`clippy`/`fmt` (backend), Vitest + React Testing
  Library + Playwright (frontend).

## Local development

Prerequisites: Rust (stable), Node.js 22+, Docker (for Postgres).

```bash
cp .env.example .env               # backend env vars
cp apps/web/.env.example apps/web/.env

docker compose up -d postgres      # Postgres on localhost:5434 (5432/5433 are used by other local projects)

cargo run -p droply-api            # API on :8080 (reads .env vars from your shell/export)

cd apps/web
npm install
npm run dev                        # frontend on :5173
```

### Tests

```bash
# Backend
cargo test --workspace                          # fast, no DB required
cargo test --workspace -- --include-ignored      # also runs DB-gated tests (needs DATABASE_URL)
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check

# Frontend (cd apps/web)
npm run lint
npm run typecheck
npm test
npm run build
npm run e2e                                      # Playwright, builds + serves dist/ first
```

## Repo layout

```text
apps/
  api/            Axum HTTP server (binary: droply-api)
  web/            React PWA
crates/
  droply-domain/       pure domain types, no I/O
  droply-application/  use-case orchestration, trait boundaries
  droply-infra/        Postgres, outbound HTTP, filesystem
migrations/       sqlx migrations (run automatically on API startup)
docker/           Dockerfile(s)
docs/             architecture.md, DECISIONS.md, CURRENT_STATE.md
```

## Workflow

Branch per task (`feature/*`/`fix/*`) → implement → tests → green tests →
merge to `main` → update `docs/CURRENT_STATE.md` (and `docs/DECISIONS.md` if
an architectural choice was made).
