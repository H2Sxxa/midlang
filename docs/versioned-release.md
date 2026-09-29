# Versioned release: design

Status: draft. Version snapshots, manifests, and `/v` routes are not
implemented. The document fixes the design before implementation, because
version identity and URL shape are the parts clients pin and are expensive to
change afterwards.

## 1. Problem

The service serves translations from one mutable redb file (one table per
locale, `String -> String`). That answers "what is the current value", but a
consumer that ships software needs a *release*: an identified, immutable,
reproducible set of translations that a build can pin, that can be audited
later, and that can be rendered with the service stopped.

ROADMAP M4 (versioned export) and M5 (offline runtime) describe that capability.
This document covers the infrastructure both depend on.

## 2. Scope

In scope:

- what a version is and how it is identified;
- how a version is frozen out of the live store;
- how a version is served read-only over HTTP;
- how a version becomes an offline artifact.

Not in scope:

- batch import and the issues/changelog/coverage read APIs (ROADMAP M1 to M3)
  - they are prerequisites, not part of this design. The live write API now
  exists, but bulk loading is still not implemented;
- the plurals/ICU value model;
- publishing workflow and multi-tenancy. Authentication is implemented for the
  live HTTP routes, while the authorization policy for version routes is still
  to be finalized.

## 3. Facts this design builds on

Already in the tree:

- `KVRead` (get/list/statistics) and `KVStore: KVRead` (set/delete/delete_locale)
  are separate traits (`crates/core/src/store.rs`). A snapshot implements only
  `KVRead`, so "you cannot write to a version" is a type-level property rather
  than a runtime check.
- Listing returns `Page<T, C>` with a self-describing `QueryCursor` that carries
  ordering, filter and position (`crates/core/src/query/pagination.rs`). The
  cursor does not carry a version, and does not need to: the version is pinned
  by the URL.
- `KVRead::statistics()` counts locales and entries from redb table metadata, so
  asking "which locales does this store have" is O(#locales) and does not scan.
- `Translation` has no value cache. A snapshot therefore needs no cache, and no
  cache key needs a version component.

These are design prerequisites, not implemented MidLang features:

- `KvSnapshot`, `VersionRegistry`, the snapshot producer, manifests, and all
  `/v` routes described below do not exist yet.
- The versioned HTTP read path must accept `KVRead`, not `KVStore`, so a
  snapshot cannot be written through the type system.
- `Translation::get_key()` currently reports a missing key to the live issue
  service. Version reads must use a pure read path, or an internal service with
  diagnostics disabled, so reading a frozen version does not create live
  issues.

Verified against redb 4.2.0 (crate sources, not documentation recall):

- `ReadOnlyDatabase::open(path)` exists, is part of the default feature set, and
  opens the file read-only. It implements `ReadableDatabase` (`begin_read`,
  `list_tables`), so the same code that reads the live store can read a snapshot,
  and snapshots can live on a read-only mount.
- `ReadTransaction` is an MVCC snapshot: it does not see writes committed after
  it starts and it coexists with writers. Freezing a version therefore does not
  stop writes.
- `WriteTransaction::persistent_savepoint()` and `ephemeral_savepoint()` exist and
  are not feature-gated. A savepoint pins pages inside the live file: pages that
  become unused after it is created are not reclaimed while it lives, and any
  live savepoint blocks `Database::compact()`.
- redb updates its file in place. A raw file copy taken while writers are active
  is not a consistent snapshot.
- redb allows one writer at a time across a whole database file.

## 4. Version identity

A version has:

- a **version number**: a monotonically increasing integer, assigned by the
  service that produced it;
- a **content hash**: SHA-256 over the canonical logical content (see below),
  plus a per-locale hash;
- a **manifest** (`manifest.json`) that carries the number, the hash, the
  creation time, and per-locale entry counts and hashes.

The content hash is defined over logical records, not over redb file bytes:

```
record  := u32le(locale_len) locale_utf8
           u32le(key_len) key_utf8
           u32le(value_len) value_utf8
content := records for all locales sorted by locale name,
           then keys sorted by key (byte order), in that locale
hash    := SHA-256(content)
```

Length-prefixing every field makes the encoding unambiguous even if a future
key contains a delimiter byte. Hashing file bytes is wrong here: redb file
layout depends on insertion history and compaction and is not reproducible.
The logical hash is stable across platforms and snapshot mechanisms. Exporters
must use the same logical ordering, but each exported file also gets its own
SHA-256; the logical hash is not a hash of JSON or another format's bytes.

Sketch:

```json
{
  "version": 17,
  "hash": "sha256:9f2c...",
  "created_at": "2026-09-29T10:20:00Z",
  "locales": {
    "en": { "entries": 1043, "hash": "sha256:..." },
    "zh-cn": { "entries": 1043, "hash": "sha256:..." }
  }
}
```

## 5. What is versioned

Only translation resources are versioned.

`issues` and `changelog` describe the service, not a translation snapshot: they
are operational state with their own retention, and they keep changing while a
version stays frozen. They never appear under a version prefix, and a version
read does not report missing keys as issues. This boundary is drawn now so that
"the issues of v17" never becomes a question.

## 6. URL surface

```
GET /v                             list versions, newest first
GET /v/{v}/manifest                manifest.json for one version
GET /v/{v}/t/{locale}              Page<KVEntry>, with keyword/limit/cursor
GET /v/{v}/t/{locale}/{key}        one value
GET /v/{v}/stats/translations      KVStatistics for one version
```

- `{v}` is the version number.
- Version numbers are non-negative integers and are sorted numerically, not as
  filename strings (`17` must sort before `9`).
- There are no write routes under `/v`, and there is no "reject writes when a
  version is present" rule to write: the snapshot type has no write methods.
- Live routes stay unprefixed (`/t/{locale}/{key}` for the current store).
- Caching: version responses carry `ETag: "<content hash>"` and
  `Cache-Control: public, max-age=31536000, immutable`. `GET /v` is the only
  non-immutable one and is served with `Cache-Control: no-cache`, because the
  set of versions grows.
- Planned errors are distinct: unknown version, unknown locale within a known
  version, and missing key are `404`; an unreadable snapshot is `500` and is
  logged. No empty-success responses. The current
  `crates/server/src/protocol/error.rs` only maps live-store errors; it does not
  implement version errors yet.

## 7. Snapshot production

Three mechanisms were considered.

| # | mechanism | pros | cons |
|---|---|---|---|
| A | logical copy into a new redb file, reading through one `begin_read()` on the live store | consistent, keeps serving writes, snapshot file is self-contained, immutable, directly distributable, works offline | duplicates unchanged data; cost is O(total entries) per version |
| B | persistent savepoint in the live file | no copy, cheap, no duplication | not distributable, not a file, pins pages (no reclamation) while alive, blocks compaction, ties version lifetime to the live database |
| C | raw file copy | trivial | not a snapshot at all while writers are active; unsafe |

Recommended: **A** for release artifacts, with B kept as a possible later
optimization for online-only serving. A is chosen because the artifact is the
point of M4/M5: whatever the server can serve as `/v/17` must also be something
that can be copied to a release machine and read with no service running.

Producer steps:

1. `begin_read()` on the live store. From here on, writers may continue.
2. For each locale table in sorted name order, copy entries in key order into a
   new redb file, one write transaction for the whole copy (bulk insert is much
   faster than per-key commits). Compute the logical hash in the same pass.
3. Write the new file to a temp path in the versions directory, fsync, then
   rename it to its final name.
4. Write the manifest to a temp path, fsync, rename it **last**. A version is
   only visible once its manifest exists, so a crash leaves an orphaned data
   file and never a version that points at a half-written snapshot.
5. Log structured fields: version, entries, bytes, duration.

Layout:

```
<versions-dir>/17.rdb
<versions-dir>/17.manifest.json
```

The version list is derived from this directory: numeric filenames give the
ordering, and the manifest is the authoritative metadata. No separate index
file is needed,
and nothing is added to the live store. That matters because
`KVRead::statistics()` treats every normal redb table as a locale, so a metadata
table inside the live store would show up as a fake locale.

Retention (how many versions stay on disk, and whether a version referenced by
an exported release is pinned) is a policy decision, not a mechanism.

## 8. Read path and handle registry

- `KvSnapshot` wraps a `ReadOnlyDatabase` and implements `KVRead`, so
  `Page`/`QueryCursor`/`statistics` work against a version with no new query
  code.
- `VersionRegistry` keeps an LRU of `version -> Arc<KvSnapshot>`, bounded by a
  handle count (`--version-handle-limit`). Each open snapshot costs one file
  handle plus its redb page cache, so the bound is what keeps memory and
  descriptors finite. Eviction closes the handle.
- Because snapshots are immutable, nothing needs invalidation, and the same
  cache-free store read logic can serve a snapshot without a per-version cache.
  It must not use the live diagnostic side effect for missing keys (see the
  constraint in section 3).

## 9. Offline artifacts

An artifact is a serialization of one snapshot. Formats, in order: native JSON,
i18next, inlang, XLIFF. Each format declares what it cannot represent.

Determinism rules, so that the same version and format are byte-identical
anywhere:

- locales sorted by name, keys sorted by key, in every format;
- UTF-8, LF, no BOM;
- no timestamps or host names inside the data files - `created_at` lives in the
  manifest only;
- fixed escaping, and a documented rounding/ordering rule for numbers.

Layout:

```
<out>/manifest.json        version, hashes, per-file sha256
<out>/<locale>.json
```

`crates/intl` loads an artifact, resolves the locale, walks fallback chains and
returns the value, so the example app renders with the service stopped (M5).

Export reads from a snapshot, never from a live read transaction: a version that
a client can pin must also be a version the server can serve and that appears in
`GET /v`.

## 10. Operations

- Disk grows as O(versions x data). Retention and a versions directory
  (`--versions-dir`) are configuration, not hardcoded paths.
- Producing a snapshot is IO/CPU heavy and holds one read transaction for its
  duration. It does not block writers, but it competes for disk with them.
- Crash safety comes from temp file plus rename and from writing the manifest
  last.
- redb repairs a database on read-write open; a read-only open cannot. A
  snapshot file must therefore be produced from a live read-write handle (which
  step 1 does) and not by copying a file that was killed mid-write.

## 11. Open decisions

D1. **Resolved for the first implementation:** `{v}` in the URL and the CLI is
  the numeric version number; the content hash is in the manifest and is used
  for immutable response validators. This keeps the URL stable while still
  allowing content-addressed verification.

D2. Do we need a movable alias (`/tag/prod` -> version)? If yes, it must be
    short-cached (it can be repointed), unlike `/v/{v}`. Recommendation: no, or
    only if a real deployment needs to move a pin without editing a manifest.

D3. Snapshot mechanism A (independent file, recommended) or B (savepoint).

D4. Are versions on disk the *only* record of versions (recommended: directory
    plus manifests), or is there also a database table or an index file?

D5. Do version reads report missing keys as issues? Recommendation: no.

D6. Retention: how many versions stay on disk, and are versions referenced by a
    shipped release pinned forever?

D7. When is a version produced: only on explicit export, on a schedule, or on
    every write batch?

## 12. Prerequisites

This design cannot be implemented before these exist:

- a batch import (`set_batch`) - the live write API is available, but without
  bulk loading there is no practical way to load a real translation set
  efficiently (ROADMAP M1);
- configurable store/sqlite/versions paths and a deployable binary (M1);
- an answer for "import while serving": redb has one writer, so a bulk import
  blocks online writes and a policy is needed (ROADMAP risks).

The issues/changelog work (M3) is independent of this document.

## 13. Implementation order

P1. `KvSnapshot` (`ReadOnlyDatabase` + `KVRead`) and `VersionRegistry`, and the
    `/v/{v}/...` read routes, tested against a snapshot file produced by hand.

P2. The producer: freeze a live store into `versions/<n>.rdb` plus manifest,
    `GET /v`, retention, and the CLI entry point.

P3. Exporters and `crates/intl` (M4/M5 proper).

## 14. Risks

- Snapshot cost is O(total data) per version. Fine for the expected size (the
  perf probe put 100k entries in the low tens of MB); if it stops being fine,
  mechanism B or an incremental copy is the escape hatch, and `KVRead` hides the
  change from callers.
- Determinism across formats is real work: escaping, nesting, plurals and
  round-tripping are where fidelity is lost, and the ROADMAP already calls this
  out.
- Version identity is a public contract. Changing scheme (number to hash, or the
  URL prefix) after clients pin versions is a breaking change, which is why D1
  is worth settling before writing code.
