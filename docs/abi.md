# Buntrail ABI 1

`bt_call` takes JSON and returns `{ "ok": true, "value": ... }` or
`{ "ok": false, "error": { "code": "...", "message": "..." } }`.
Release every returned C string, including error replies. Never free an engine
while a call is in flight. Calls with malformed JSON return errors, never throw.

## Documents

- `document.open`: text, optional path, version and remote connection ID -> id, revision=0, text, dirty.
- `document.get`: id -> id, revision, text, dirty, optional path.
- `document.edit`: id, revision, start, end, text -> id, revision, dirty. Ranges
  are UTF-16, end exclusive; revisions must match. Apply to the native mirror
  only after acceptance. No full document copy in the edit response.
- `document.undo` / `document.redo`: id -> full document state.
- `document.bind`: id, revision, path, version, optional remote -> state; binds
  a successfully created snapshot without losing undo history. Rejects stale revisions.
- `document.reload`: id, revision, path, expected_version, text, version, optional
  remote -> state; accepts a fresh `fs.read` snapshot only for a clean document
  whose revision, file binding and previous version still match. Native callers
  read off-thread and apply this command and the accepted mirror together on the
  GUI thread. Reload retains undo history; undoing it creates unsaved changes.
  Full document states include the observed `version` for bound files.
- `document.analyze`: id, language -> revision-tagged highlights, folds, symbols,
  hasErrors and truncated. Run off GUI thread and discard stale results.
- `document.save`: id -> state; requires bound path and observed version.
- `document.close`: id, discard=false -> null; refuses dirty close.

## Services (run off GUI thread)

- `fs.read`: path -> text, version. Feed these into document.open.
- `fs.list`: path -> {entries:[{name,path,directory}]}.
- `fs.create`: path, text -> version; exclusive creation, refuses existing paths.
- `fs.write`: path, text, version -> version; compare before replacement.
- `git`: root, args (array of literal arguments) -> stdout, stderr, status.
  UI must use explicit Git actions, not user-controlled command interpolation.
- `search`: root, query -> matches [{path,line,text}], truncated.
- `terminal.start`: root, optional shell -> session id.
- `terminal.read`: id -> bytes (lossless), output (lossy preview), alive, drained, truncated.
  Process exit is not output EOF; keep reading until drained is true.
- `terminal.write`: id, exactly one of text or bytes -> null. Input is queued
  with finite capacity; full/closed queues fail explicitly without accepting bytes.
- `terminal.resize`: id, columns, rows -> null.
- `terminal.close`: id -> null; explicit process termination only.

No successful command response is proof of native, remote or release acceptance.
The checked-in implementation and its tests define currently available commands.

## Remote services

`remote.start` takes transport (`ssh`, `docker`, `podman`), target and optional
constrained helper executable; it returns connection `id` and helper `instance`.
`remote.call` takes that id and a nested service request; `remote.close` closes
the transport. A remote document must carry its remote ID at open/bind, so a
save never falls back to a same-named local file. Remote helpers currently must
already be installed. Connection readiness is not proof of identity continuity.

The current ABI serializes commands as JSON, including terminal data. This is
a known performance limit relative to the planned binary terminal hot path;
actual performance/release acceptance remains open.

## Additive editor content interface

The donor-named `buntrail_*` editor exports in the same generated header accept the existing `BtEngine*` and numeric document IDs. They do not construct a separate engine. `buntrail_document_transaction` applies atomic UTF-16 replacements, undo/redo and composition intents; accepted `BuntrailBatch` values own only inserted delta bytes and replaced ranges. Native typing/history consumes those deltas. Existing JSON history still returns full document state.

`buntrail_snapshot_acquire_document/retain/read/release` return immutable Arc-backed snapshots that remain valid after document close or engine free. Each handle releases independently through a slot that becomes null; reads use byte offsets and copy at most 64 KiB. Batches/find plans also release through nullable slots. Callers must not dereference released handles or race reads against release. Offset mapping rejects scalar/surrogate boundaries and stale revisions.

`buntrail_find_plan/info/range/replace/release` preserve donor literal/regex find/replace and bounded results. Plans carry engine/document/source identity and revision; foreign, stale or closed-document application is rejected atomically. `buntrail_document_set_language` sets an editor override; empty bytes restore automatic donor filename/shebang/compound association.

Two small JSON additions serve the copied native content: `document.status` returns id/revision/dirty/path/version/remote/language_generation without text; `document.editor_analyze` accepts id and selects the donor language association automatically. Legacy `document.analyze` still requires its explicit language string. Both analysis paths share the cached donor LanguageAdapter, preserve the existing 1 MiB analysis admission/time/output budgets and reject changed/closed/language-reassociated publication. Rich grammar/query/availability/outline/injection data accompanies the legacy fields. Source text remains editable above analysis limits up to existing admission/edit limits.

`document.rebind` accepts id, expected_path, path and the captured optional remote route after a successful existing fs.rename. It validates the old binding/route and pending-save guard, then updates only the path. It preserves the observed content-digest version, dirty/saved checkpoint, revision and undo/selection history; it does not mark concurrent edits saved. This differs from document.bind, which binds a successfully saved revision.

Native final language delivery rechecks this authoritative status after the asynchronous Rust result arrives on the main queue. A changed revision, association generation (including override away/back), path or unavailable document suppresses the queued result; valid current analysis failures remain visible. This closes the extra delivery window after Rust publication validation.
