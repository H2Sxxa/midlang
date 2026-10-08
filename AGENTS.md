# MidLang Project Guidelines

MidLang is a translation middleware that decouples translation resources from
business data. The service exposes an HTTP API for analyzing and translating
text, while keeping service data and translation data in separate stores.

## Architecture

- `crates/core` contains the shared domain logic and KV-store
  implementations. Translation data is stored in the KV store, currently
  backed by memory or redb.
- `crates/server` contains the HTTP server, CLI, authentication, SQLite-backed
  service data, and API documentation.
- `web/midlang-webui` contains the React/TypeScript administration console.
- `web/midlang-docs` contains the VitePress documentation site.
- The Rust workspace currently includes `crates/core` and `crates/server`.

Keep storage responsibilities separate: SQLite is for service data; the KV
store is for translation data.

## Local Development

Run the server with ephemeral storage during development and tests:

```sh
cargo run -p midlang-server -- --store mem --sqlite-url sqlite::memory:
```

Do not use the default persistent redb/SQLite files for routine local
development unless the task specifically requires persistent data.

To run the web console:

```sh
cd web/midlang-webui
pnpm install
pnpm dev
```

The server listens on `127.0.0.1:4321` by default. Use the documented CLI
flags when a different port, store, or origin is required.

## Validation

For Rust changes, run the smallest relevant checks; for workspace-wide
changes, use:

```sh
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

For web console changes, run from `web/midlang-webui`:

```sh
pnpm lint
pnpm build
```

If an API change requires refreshing the generated client, start the server
and run `pnpm generate:api` before building the console.

## Conventions and Constraints

- Follow the existing Rust and TypeScript patterns; keep changes focused and
  update directly related documentation when behavior or commands change.
- Never add SQLite migration SQL. The project is unpublished, and schema
  changes must follow the repository's current initialization approach.
- Prefer in-memory server storage for development. Do not commit generated
  build output, local databases, tokens, or other runtime artifacts.
- Surface configuration and storage errors explicitly; do not silently fall
  back to another backend or hide initialization failures.
