---
layout: home

hero:
  name: MidLang
  text: Translation middleware
  tagline: Decouple translation resources from business data. Serve them online, see what is missing, and pin a version for every release.
  actions:
    - theme: brand
      text: Get started
      link: /guide/getting-started
    - theme: alt
      text: How it works
      link: /#how-it-works

features:
  - icon: 🔌
    title: Served online
    details: Applications fetch keys over HTTP from a live store instead of shipping a frozen bundle, so a correction reaches production without a rebuild.
  - icon: 🎯
    title: One source of truth
    details: Online serving and offline export read the same store, so a release artifact can never disagree with what the service served.
  - icon: 🔍
    title: Diagnostics built in
    details: A missing key is recorded as an issue, every mutation lands in a per-key changelog, and coverage is measured against a reference locale.
  - icon: 📦
    title: Versioned export
    details: Snapshot the store into a content-hashed version with a manifest, then pin that version in your release pipeline.
  - icon: 📡
    title: Offline runtime
    details: An artifact-backed mode behind the same interface lets an application render even with the service stopped.
  - icon: 🧩
    title: Values that can grow
    details: Values travel as message objects rather than bare strings, so plurals and ICU can arrive later without breaking clients.
---

## How it works

An application asks for a key. MidLang answers from the store, or records the
miss so it can be translated next:

```text
your app ── GET /t/zh-CN/greeting.hello ──▶ MidLang ──▶ store (one table per locale)
                                              │
                                              └── miss ──▶ issues (what to translate next)
```

Reads and writes are authenticated and permission-scoped
(`translation:read`, `translation:write`, `translation:delete`). Every committed
write is published to the changelog and to coverage tracking, so history and
statistics stay current instead of being recomputed on demand.

## What it is not

- A general expression or query engine.
- A full translation management system or a real-time collaborative editor.
- Broad format support before the first formats are lossless.

## Status

MidLang is early. The store, the HTTP translation API and the admin console
exist; deployment hardening, bulk import, the diagnostics APIs and export are
still ahead.

- **Working now** — a redb-backed store with paged listing and statistics,
  authenticated translation read/write/delete over HTTP, opaque bearer tokens
  with permission groups, and missing-key issues plus a per-key changelog.
- **In progress** — configurable deployment and packaging, initial bulk import,
  and the issues/changelog/coverage read APIs.
- **Planned** — the SDK, versioned export, the offline runtime, and plurals/ICU
  in the value model.
