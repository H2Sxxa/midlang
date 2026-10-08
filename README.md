# MidLang

MidLang is a translation middleware that decouples translation resources from
business data. Applications read and update translations through an HTTP API,
while the same store can later be used for diagnostics, export, and offline
artifacts.

> MidLang is in early development. APIs, storage formats, and routes may still
> change.

## Why MidLang?

- **Serve translations online** without bundling a frozen translation file
  into every application release.
- **Keep one source of truth** for online reads, writes, diagnostics, and
  future exports.
- **Track missing translations** as issues instead of losing them in logs.
- **Record translation history** through a per-key changelog.
- **Measure locale coverage** against a reference locale.
- **Use the WebUI** to inspect and manage translation resources.

## Quick start

### Start the server

The development command uses in-memory translation storage and an in-memory
SQLite database, so it does not modify local data files:

```sh
cargo run -p midlang-server -- --store mem --sqlite-url sqlite::memory:
```

The server listens on `127.0.0.1:4321` by default. On the first start, it
prints an administrator token once. Save the token before stopping the server.

Check that the service is running:

```sh
curl http://127.0.0.1:4321/health
```

The interactive API documentation is available at
[`http://127.0.0.1:4321/docs`](http://127.0.0.1:4321/docs), and the OpenAPI
document is available at
[`http://127.0.0.1:4321/openapi.json`](http://127.0.0.1:4321/openapi.json).

### Run the WebUI

In another terminal:

```sh
cd web/midlang-webui
pnpm install
pnpm dev
```

The console asks for the server URL and administrator token. Set
`VITE_MIDLANG_REMOTE_URL` in `web/midlang-webui/.env.local` if the server is
not running at the default URL.

## API example

Translation reads and writes use bearer-token authentication and
permission-scoped tokens.

Read a translation:

```sh
curl \
  -H "Authorization: Bearer <TOKEN>" \
  http://127.0.0.1:4321/t/zh-CN/greeting.hello
```

Write a translation:

```sh
curl -X PUT \
  -H "Authorization: Bearer <TOKEN>" \
  -H "Content-Type: application/json" \
  -d '{"value":"你好，世界"}' \
  http://127.0.0.1:4321/t/zh-CN/greeting.hello
```

Keys can also be addressed with an explicit namespace:

```text
/t/{locale}/{namespace}/{key}
```

The main translation permissions are:

- `translation:read`
- `translation:write`
- `translation:delete`

See the generated API documentation at `/docs` for authentication, token
management, issues, changelog, coverage, and statistics endpoints.

## Architecture

```text
application ── HTTP ──▶ midlang-server ──▶ midlang-core ──▶ translation store
                              │                    └──────▶ diagnostics
                              └───────────────────────────▶ SQLite service data
```

- [`crates/core`](crates/core) contains the translation domain logic, KV-store
  abstraction, memory/redb backends, and internal services.
- [`crates/server`](crates/server) contains the HTTP API, CLI, authentication,
  OpenAPI documentation, and service wiring.
- [`web/midlang-webui`](web/midlang-webui) contains the React/TypeScript
  administration console.
- [`web/midlang-docs`](web/midlang-docs) contains the VitePress documentation
  site.

Storage responsibilities are intentionally separate:

- The **KV store** holds translation data. It supports `mem` for development
  and `redb` for persistent local storage.
- **SQLite** holds service data such as authentication, issues, changelog, and
  coverage.

## Development

### Rust checks

Run the smallest relevant check for a change. For a workspace-wide validation:

```sh
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

### WebUI checks

```sh
cd web/midlang-webui
pnpm lint
pnpm build
```

If an API change affects the generated client, start the server and refresh it
before building:

```sh
pnpm generate:api
```

### Documentation

The detailed guide is in
[`web/midlang-docs`](web/midlang-docs). To run it locally:

```sh
cd web/midlang-docs
pnpm install
pnpm dev
```

## Project status

### Available now

- Memory and redb translation stores
- Authenticated translation read, write, and delete endpoints
- Permission groups and bearer tokens
- Missing-key issues
- Per-key changelog
- Locale listing, store statistics, and OpenAPI/Swagger UI
- React administration console

### In progress or planned

- Deployment and packaging improvements
- Bulk import and export
- Public issue, changelog, and coverage APIs
- Versioned export artifacts and offline runtime
- SDKs
- Plural and ICU message support

## License

MidLang is licensed under the
[GNU Affero General Public License v3.0](LICENSE).
