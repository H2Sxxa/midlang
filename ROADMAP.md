# MidLang Roadmap

MidLang is a translation middleware: it decouples translation resources from
business data. Online mode comes first: get the service deployed and integrated,
then versioned offline export is a derived capability over the same store.

## Sequencing

1. Deployable service (read + write + config + packaging).
2. Minimal integration (one stable API, one SDK, missing-key loop closed).
3. Online diagnostics (issues and changelog become queryable).
4. Versioned export (snapshot the store into release artifacts).
5. Offline runtime (consume artifacts without the service).

## Principles

- **Online first.** The first milestone that matters is a service someone can
  deploy and integrate, not a format pipeline.
- **One source of truth.** Both online serving and offline export read the same
  store, so they cannot diverge.
- **Export is a snapshot.** Once online works, exporting is serializing a store
  version into an artifact; it is a derived feature, not a second system.
- **Keep the value extensible.** Serve values as message objects rather than bare
  strings, so plurals/ICU can be added later without breaking clients.
- **No silent errors.** Handlers must distinguish not-found from failure instead
  of collapsing both into an empty response.

## Current state

- `crates/core`: `KVStore` + `RedbStore` (one redb table per locale,
  `String -> String`), `Translation` with an LRU cache and `get`/`set`/`delete`,
  `InternalService` with `IssueCollector` and `ChangelogRecorder` (SQLite, write
  only).
- `crates/server`: `TCPProtocalServer` with axum routes `/health`,
  `GET /t/{locale}/{key}` and `GET /t/{locale}/{namespace}/{key}`; `main.rs`
  assembles store/internal/translation and binds `127.0.0.1:port`.
- `crates/intl`: empty. `uds`, `rpc`, `grpc`: placeholders.
- `web/midlang-webui`: login, overview and settings shell without data views.
- Missing: no write API, no initial import, no issues/changelog read API, no SDK,
  no deployment config, no versioning or export.

## M1 - Deployable service

Goal: one command starts a service with a persistent store that can be read and
written.

- Make binding address, store path, sqlite path, cache size and logging
  configurable instead of hardcoded.
- Expose the write path: `set`/`delete` over the API (only reads exist today).
- Support importing an initial batch of translations, otherwise the service has
  nothing to serve.
- Health/readiness, structured logs, graceful shutdown, single binary and
  container packaging.
- Stop swallowing handler errors (`Err(_) => Json(None)`); return distinct
  not-found and failure responses and report failures.

Done when: on a clean machine the service starts, translations can be written,
`GET /t/...` returns them, and health checks pass.

## M2 - Minimal integration

Goal: a real application integrates and uses it.

- Freeze a versioned read/write API with stable error shapes.
- Ship one minimal SDK for the primary stack (Rust or TS).
- Close the missing-key loop: a client miss is reported and becomes queryable.
- Add one example app and a quick-start document.

Done when: the example reads translations through the SDK, and requesting an
undefined key makes it appear in issues.

## M3 - Online diagnostics

Goal: deliver the core online value, explaining translation problems.

- Issues API: missing keys grouped by locale/namespace/key with first/last seen
  and counts.
- Changelog API: per-key history (`origin`, `state`) plus rollback.
- A minimal view (web UI or CLI) to see what is missing, what changed and how to
  roll it back.

Done when: a key missing in production can be listed and located, and a change
can be rolled back.

## M4 - Versioned export

Goal: export artifacts from a store version for offline release.

- Version identity: content hash plus version number, and a `manifest.json` with
  the version, per-locale files and hashes.
- Deterministic serialization: stable ordering, encoding and line endings so the
  same version is byte-identical anywhere.
- Exporters, in order: native JSON, i18next, inlang, then XLIFF; each format
  declares what it cannot represent.
- CLI/HTTP: `export --version <sha> --format <fmt> --out <dir>`.

Done when: the same version exports byte-identically on any machine and drops
into an existing release process.

## M5 - Offline runtime

Goal: applications render translations without the service.

- Fill in `crates/intl`: artifact loading, locale resolution, fallback chains.
- Add an artifact-backed mode to the SDK behind the same interface.
- CI recipes to generate artifacts and commit them or attach them to a release.

Done when: the example renders fully with the service stopped.

## Later

- Plurals and ICU in the value model, together with the API/SDK evolution.
- More formats: ARB, gettext, Android, Apple, CSV/TMX.
- Glossary, translation memory, and MT/AI suggestions (review-gated, never
  auto-published).
- UDS and gRPC protocols.

## Non-goals (for now)

- A general expression or query engine.
- A full TMS or real-time collaborative editing.
- Broad format support before the first formats are lossless.

## Risks

- The store is flat `String -> String`. Fine for an online MVP, but adding
  plurals is a breaking change; message-object values make that cost small.
- Export being easy holds for the pipeline, not for fidelity. Determinism and
  format handling (plurals, escaping, nesting, round-trip) remain real work.
- Once the write API and SDK are public, interface changes get expensive, so M2
  is the point to be deliberate about the contract.
