# Getting started

::: warning Documentation in progress
This site is being filled in. This page covers what runs today; the API,
integration and export guides land next.
:::

## Run the server

MidLang ships as a Rust binary. From the repository root:

```bash
cargo run -p midlang-server -- --store mem --sqlite-url sqlite::memory:
```

The server binds `127.0.0.1:4321` by default. Useful flags:

| Flag | Purpose |
| --- | --- |
| `--port` | Bind port, `4321` by default. |
| `--store` | `redb` (default, persistent) or `mem` (throwaway). |
| `--store-url` | Redb file path, `translation.rdb` by default. |
| `--sqlite-url` | Shared SQLite database for auth, issues, changelog and coverage. |
| `--cors-origin` | Allowed origins; repeat or comma-separate. Defaults to all origins for local development. |
| `--coverage` | Maintain per-locale coverage. Off by default because it costs one scan of the store at startup. |

On the first start the server creates an admin token and prints it once. Store
it before the process exits; it is not shown again.

## Open the admin console

```bash
cd web/midlang-webui
pnpm install
pnpm dev
```

The console asks for a server URL and that token. Set
`VITE_MIDLANG_REMOTE_URL` (for example in `web/midlang-webui/.env.local`) to
pre-fill the URL field.

## Talk to the API directly

Reads and writes are bearer-token authenticated and permission-scoped:

```bash
curl -H "Authorization: Bearer $MIDLANG_TOKEN" \
  http://127.0.0.1:4321/t/zh-CN/greeting.hello
```

Keys can be namespaced, which yields a second route shape:
`/t/{locale}/{namespace}/{key}`.

## Next

The overview on the [home page](/) explains what MidLang is for, and
`ROADMAP.md` in the repository tracks what is implemented and what is next.
