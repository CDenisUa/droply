# Decisions (ADR-lite)

Each entry: status, date, decision, why. Never edit history — if a decision
changes, add a new entry and mark the old one `superseded`. Other docs link
here instead of repeating rationale.

---

## 0001 — Backend runtime: Rust instead of ASP.NET Core

- **Status:** accepted
- **Date:** 2026-08-11

`Droply-Architecture.md` v0.1 specifies ASP.NET Core / C# / EF Core / SignalR
throughout. Asked the user explicitly (Rust vs ASP.NET Core vs a Rust-on-edge
hybrid); user chose full Rust.

**Why:** No FFmpeg dependency in the early phases (0-3), so nothing forces a
native-process-friendly-but-heavy runtime. Rust fits the spec's own rules
better than .NET does — "never buffer a whole file in RAM", "always stream" —
and a small native binary sidesteps most of the pain from Render free tier's
cold start after spin-down (§44 of the architecture doc). Deployment target
is unchanged (Render free web service), only the language/runtime swaps.

**Consequence:** Every C#-specific instruction in the original doc (EF Core,
SignalR, `Process.Start`, nullable reference types, `dotnet watch`, the
`mcr.microsoft.com/dotnet/*` Docker base images) needs a Rust-equivalent
reading, not literal application. See ADR 0002-0004 for the concrete
replacements.

---

## 0002 — Web framework: Axum

- **Status:** accepted
- **Date:** 2026-08-11

**Why:** Tokio-native, integrates directly with `tower`/`tower-http`
middleware and `tracing` for the structured-logging requirement (doc §38).
Currently the more idiomatic default in the Rust ecosystem for a REST API
with streaming responses. Decided as an implementation detail (not asked)
per the user's own guidance to decide-and-document small choices rather than
blocking on every one — see memory `feedback-ask-questions-and-discuss`.

---

## 0003 — Database access: SQLx, no ORM

- **Status:** accepted
- **Date:** 2026-08-11

**Why:** The architecture doc explicitly says "do not create unnecessary
abstraction layers" and "do not add generic repositories by default" (§36,
§63). SQLx gives compile-time-checked raw SQL without an ORM's abstraction
tax (vs. SeaORM/Diesel), which matches that principle directly and keeps the
persistence layer thin and legible to coding agents.

---

## 0004 — Real-time progress: Server-Sent Events (SSE), not WebSockets/SignalR

- **Status:** accepted
- **Date:** 2026-08-11

**Why:** Progress updates (doc §25) are one-directional (server → client).
All commands (cancel, retry, pause) already go over REST (doc §26), so
nothing needs a bidirectional channel. SSE needs no extra client library
(native `EventSource`), auto-reconnects, and passes through Render/Cloudflare
HTTP proxying without special handling. Flagged to the user in the Phase 0
summary rather than blocked on — reconsider if a future feature genuinely
needs server-initiated bidirectional push.

---

## 0005 — WASM: out of scope for V1

- **Status:** accepted
- **Date:** 2026-08-11

Asked the user explicitly; chose to skip WASM entirely for now rather than a
shared Rust→WASM validation crate or ffmpeg.wasm client-side tooling.

**Why:** No concrete use case yet. Revisit only when one appears (e.g. if
duplicate URL-validation logic between frontend and backend becomes a real
maintenance cost, or Phase 7 media tools want in-browser processing).
