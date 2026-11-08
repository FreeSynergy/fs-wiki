# fs-wiki - Design

`fs-wiki` is the wiki program of FreeSynergy and the **owner of the text interface** (ADR 0031, 0050): every text is a wiki
entry, a collection of texts is a wiki. The database is the truth, files are exports (ADR 0042). The contract is `texts.capnp`
in [`fs-interfaces`](https://github.com/FreeSynergy/fs-interfaces) (ADR 0045); this program is its provider. This repo holds
six crates (one of them only for tests); the GUI lives only in `fs-wiki-app`, on [`fs-components`](https://github.com/FreeSynergy/fs-components) (Blinc):

* `crates/fs-text`: the **library** of text + layers (document, marks, Markdown, outside changes, links, templates, history).
  No storage, no interface. Its own design is in [`crates/fs-text/DESIGN.md`](crates/fs-text/DESIGN.md).
* `crates/fs-wiki-app`: the **first visible part**, a thin shell: imports a folder of Markdown files, maps texts to the neutral
  blocks of `fs-components`, and offers the wiki to the reader workbench as its data source. No Blinc type appears in it. It talks to
  the running `fs-wiki-server` through `fs-wiki-client` (ADR 0021), never to `WikiRepo` or a store.
* `crates/fs-wiki-client`: the **Cap'n Proto client**: `RpcTexts` implements `Texts` + `TextFeed` of the core over a Unix socket.
* `crates/fs-wiki-server`: the **Cap'n Proto server**: the contract `texts.capnp` (fs-interfaces v0.45.0) over a Unix socket, in front of `Texts` + `TextFeed`. It only translates; no text logic.
* `crates/fs-wiki-core`: the **program core**. The trait `Texts` (Rust form of `Texts` in `texts.capnp`) and `WikiRepo`, its
  implementation over `fs-objects`.

## Interface status

**Interface status: draft, consumers: none (0 of 2).** (ADR 0045: the interface is a target; it stays draft until two independent
consumers have tested it, then 1.0, then only version jumps.) Nothing here is declared 1.0.

## Handover (2026-11-07, end of the sprint day)

Branch `client` (`5ed6664`, four independent Opus rounds, `gates.sh` green on a fresh clone at that commit, 344 tests) holds
`fs-wiki-client` (`RpcTexts`: `Texts` + `TextFeed` over `texts.capnp`) and `fs-wiki-app` talking to `fs-wiki-server` over Cap'n Proto.
**Not merged, not tagged:** `origin/main` moved on (server round, tag `v0.3.0` by thread 3). Next plan: rebase `client` onto `main`,
resolve the version (the tag `v0.3.0` exists; this becomes `v0.3.1`), run `gates.sh` on a fresh clone, tag.
Open on `client`: the guess "no text named by the records exists on the server" can be replaced by `Texts::identity()`
(`SupportReport.instance`, `fs-objects` v0.4.0); "backup with only some of the texts" stays open. One low finding of round five:
with a single entry in the records, one failing `get` already counts as "all failed" and stops the import (`check.rs`); drop that
rule or trigger it only on connection faults. `check.rs` says `Pattern: none` but calls the server; `origins_log.rs` says `none`
but writes files: both belong to the open pattern questions. `deleted` in a record has no effect in the real program yet (the app
has no way to delete). A local, unpushed branch `t1-bump` (version 0.3.0 commit) was dropped in favour of the tag by thread 3.

## State

Version 0.3.0 (workspace, branch `bump-044`; the tag is set by the owner). Built on `fs-objects` v0.5.0, `fs-interfaces` v0.45.0 and `fs-components` v0.2.0 (git tags). New in 0.3.0: pages cut in the store at a place, no quadratic import, `LinkRow.title` / `internal`, store identity in the report, typed problem extensions, `TextFeed::subscribe` may say `Unsupported`.

| Area | State |
|---|---|
| `Texts` trait: required `capabilities`, `get`, `get_by_path`, `list`; every other method of the contract defaults to `Unsupported` (`create`, `edit`, `move_to`, `delete`, `find`, `links`, `backlinks`, `import_text`, `export_text`) | done |
| `WikiRepo` over `ObjectRepository` + `ObjectQuery` (`WikiRepo::new(Arc<dyn Store>, actor)`, `with_backend`, `with_instance`) | done |
| Capability report: all ten ids of the schema, `text.subscribe` included (one entry each, tested) | done |
| `TextFeed` (Observer, `feed.rs`): `subscribe(listener) -> Subscription`; `TextChange::{Created, Updated, Deleted}(id)`, told after every successful store write on the writer's thread (create, edit, `edit_if`, delete; a move tells every text it wrote, the moved one last; an import tells create, update or nothing). A refused or failed write tells nothing. Dropping the `Subscription` ends it | done, tested (`fs-wiki-core/tests/feed.rs`) |
| Server `fs-wiki-server`: **every method of `Texts`** (`supportReport`, `create`, `edit`, `editIf`, `move`, `moveFolder`, `delete`, `get`, `getByPath`, `list`, `find`, `links`, `backlinks`, `importText`, `exportText`, `subscribe` with `ended`); typed problems (`texts/not-found` 404, `texts/conflict` 409 with a detail starting `conflict:` as the schema says, `texts/path-taken` 409, `texts/move-refused` 409, `texts/invalid` 422, shared `invalid-request` 422, `too-large` 413, `unsupported` 501); every write through `run_to_end`; binary with `--socket`, `--data`, `--actor`, `--instance`. A provider that lacks a method answers `Unsupported` with the capability id (tested with a read-only provider). Nothing in the contract is left out | done, draft |
| Server limits: Markdown / file text at most `MAX_SOURCE_BYTES` (4 MiB), a document at most `MAX_INTAKE_BYTES` (16 MiB); Markdown whose document would exceed 16 MiB is refused **before** it is stored (a document is about 100 times larger than one-letter paragraphs); a page is cut by weight (`Page::of_weighted`); a single answer must fit (`check_answer`); `moveFolder` answers `too-large` (the move is done, the text says so) if the moved texts together exceed one message | done, tested |
| Envelope: AS2 `Article`, `context=texts`, IRI `https://<instance>/texts/<id>`, attribution, **derived name** (ADR 0032): front matter `title:`, else first heading, else first line of the first text block, else last path segment | done |
| Stored content: the record `{ path, document }` as JSON, media type `application/vnd.freesynergy.text+json` (provisional). The JSON form of `Document` is `fs-text`'s serde form (round trip of every mark kind tested in `fs-text/tests/serde_form.rs`) | done |
| `path` unique: checked in the store before every write under a per-repo write lock; a clash is `Error::PathTaken`; invalid paths are `Error::InvalidPath` (`TextPath`) | done (in-process only, see open points) |
| `create(path, markdown)`, `edit(id, document)` (id of the document forced to the text id; envelope, IRI and `published` stay), `delete` (the text and its layers; undoable through the store) | done |
| `list(prefix, page)`: one store query on the index `path`, **exact, case sensitive prefix** (text order range), path order, paged by the store | done |
| `page(query, after, size)` (`Texts::page`, `repo/page.rs`): the page behind `list` / `find` on the wire. A `Place` = (`path_key`, name, id) is where a text stands in the order; the page after a place is at most three store queries with a limit (same key and name, later id; same key, later name; later key), never the whole list. The index fields `path_key` (`sort_key(path)`) and `id_key` (id as text) are written with every text. The first page checks that every text has them (`Error::Stored`, names `upgrade_index`) | done, tested (`counting`, `places`) |
| `upgrade_index()` (`repo/upgrade.rs`): adds `path_key` and `id_key` to texts stored by 0.2.0 (only the index; the document is not read; `updated` moves, `revision` does not). A text that cannot be repaired is named in the `IndexReport`. The server runs it at start and prints the count | done, tested on a real 0.2.0 database (`tests/fixtures/v020.db`) and by the real program |
| `identity()` (`Texts::identity`, `WikiRepo::with_info`): the id of the store (`StoreInfo`), `None` for a provider without a store; the report's `instance` comes from it | done, tested over the socket |
| `find(term, page)`: one store query: `Contains` on the name, on the path and on the index `block` (plain text of each block) | done |
| Link table: marks are the truth; `links(id)` = `extract_links` of the stored document (`LinkEntry`: source, target path as written, place `<block>:<start>-<end>`, title, internal); `backlinks(path)` = one store query on the index `link` (page part of every internal link), then the rows of the texts found | done |
| `move_to(id, path)`: other texts that link to the old page are rewritten (exact page, `#fragment` kept, links to `old/...` stay); the moved text last, its own relative links written again for its new place. A link is first resolved against the page it stands on (`repo/target.rs`): `[[a/b]]` is a literal path, `[x](../a/b.md)` is relative to its page; a rewritten link keeps its written form (relative stays relative, `./`, trailing `/` and `#fragment` kept). `MovePrefix` of fs-text sees neither the page nor the style, so `repo/relink.rs` does this with the same ops | done, tested (also links inside the moved text, retry after a stop half way) |
| `move_folder(prefix, path)` (`moveFolder` of `texts.capnp`, capability `text.move`): the text at `prefix` and all `prefix/...` move, links in all texts follow (also between the moved ones and links inside moved pages whose position changed; `manual-old` is not below `manual`). A record of the last move (`repo/move_log.rs`) makes a repeat after a completed move answer with the moved texts and write nothing, refuses a move onto texts that are not part of this very move (`TargetNotEmpty`; no silent merge), and refuses another move while one is unfinished (`MoveUnfinished`; no split folder) | done, tested on a real `TursoStore` (plan, clash, interruption at every write, undo, 510 texts) |
| `import_text(path, media_type, text)`: new path = create; existing path = outside change: `import_external` diff becomes ordinary ops, layers move with the text, deleted text takes its marks along, unchanged file writes nothing | done, tested |
| `export_text(id, media_type)`: Markdown, layers not written | done |
| `edit_if(id, revision, document)` (`editIf` of `texts.capnp` 0.23, `TextEntry::revision`): compare-and-set under the write lock; a stale writer gets `Error::Conflict` and nothing is written. The revision is a random 64-bit value stored in the record and drawn anew at every write (never 0), not a time and not a counter: neither a clock stepping back nor a store undo that brings an old record back can make one come round again (a counter did, after an undo). `texts.capnp` still says "counts every write" in a comment (strategy to correct). Atomic **inside one process** only | done, tested with racing tasks |
| `Origins` (`FileOrigin`: file, hash of the file, text id, hash of the text's Markdown): what a file looked like when last imported or exported, what the app decides with. `WikiRepo` can keep it in its store (used by the core's own tests); the **app does not use that**: its records are its own file (`FileOrigins`), because they are not part of the contract and belong to a docs folder, not to a server | done |
| Media types (`TextFormat`, Factory): `text/markdown` (parameters and case ignored); anything else is `Error::MediaType` | done |
| Example `cargo run -p fs-wiki-core --example manifesto`: imports `fs-documentation/docs/en/manifesto.md`, reads it back (identical), lists its links | done |
| Tests | `fs-wiki-core`: the integration tests (`texts_crud`, `texts_list`, `links`, `move_text`, `move_folder`, `move_folder_failure`, `move_safety`, `relative_links`, `edit_if`, `outside_change`, `provider`); `fs-text` has its own |
| Client `fs-wiki-client` | `RpcTexts::connect(socket)` / `connect_with(socket, Timeouts)`; `Texts`: the **fourteen methods** of `Texts`, `capabilities` (the server's own report) and `TextFeed::subscribe`: the sixteen methods of the contract. `list` / `find` read as many cursor pages as `Paging` needs, each page a call of its own with its own 30 s deadline (texts before `offset` are read too: the cursor is a position the client cannot make). Problems read back as `fs_wiki_core::Error` (inverse of `errors.rs` of the server): `texts/not-found` to `NotFound(id)` / `NoSuchFolder(prefix)`, `texts/conflict` to `Conflict { id, expected, found }`, `path-taken` to `PathTaken`, `move-refused` to `MoveUnfinished` / `TargetNotEmpty`, `texts/invalid` to `InvalidPath` (the path is parsed again by the core's own `TextPath`, no reason is copied), `InvalidMove` (two known reasons), `MediaType`, else `Store(Invalid)`; `unsupported` to `Unsupported`; a method the server does not know to `Unsupported`; `too-large` to `Store(Invalid("too large: ..."))` (the core has no variant); a lost connection, a deadline or an unreadable reply to `Store(Backend)` | done |
| Client tests | 43 (37 over a real socket against the real server: `texts`, `paging` (more than two pages), `links`, `moves`, `errors`, `feed`, `gone` (nothing listening, a server that stops, a mute socket, the deadlines), `feed_end` (a feed the server ends: the old end does not touch a new subscription, tested with an old listener that holds the connection thread in `ended` while another thread subscribes; subscribe from inside `ended`; a listener that subscribes in `ended` on a dead connection is told a few times and then left alone), `move_large` (a folder move whose answer is `too-large` is still a move; about 65 s, it moves 3 x 14 MB); 6 unit tests: the table of problems the server cannot be made to send, and the reading of quoted paths) | done |
| Tests (server) | `fs-wiki-server` over a real Unix socket against the real `TursoStore`: `texts`, `paging`, `places` (a cursor is a place), `counting` (rows for n and 2n texts: list, find, import), `links`, `moves`, `transfer`, `report`, `instance`, `problems`, `feed` (waits for the event, no fixed time), `feed_ends` (`ended`; `Unsupported` subscribe), `cancel` (a client that goes away at a write; fails without `run_to_end`), `errors`, `limits`, `binary` (the real program across a restart), `upgrade` (the real program on a 0.2.0 database). Counts of the last run are in the report of the commit |
| `fs-wiki-app` (binary): `--docs` (default `../fs-documentation/docs/en`), `--store` (a file, or `memory`; default one file per docs folder in the data directory), `--export` (enables the Export button), `--snapshot` + `--step` (a picture without a window, see below) | done |
| App: sync of every `*.md` under the folder (`import.rs`, `sync.rs`), path = relative path. A record per file (hash, text, hash of the text) decides: file as recorded = nothing happens and the file is not parsed, whatever the database holds (a saved, moved or deleted text stays so); file changed and text as recorded = normal edit (ADR 0042 section 5); both changed = the database text stays, the file's version is a conflict shown on the problems page; new file = import; file gone = reported, text kept. Failures, conflicts, gone files and skipped entries (symbolic links, `.MD`, unreadable entries) are the first tree entry "Import problems (n)" | done |
| App: no server, no start: without a reachable socket the app does **not** fall back on a store of its own; it names the socket and how to start the server (`server.rs`, tested with the real binary). The texts are the server's, so **one server is one wiki**: use one server (its own `--data`) per documentation source. The records of imported files (`FileOrigin`: file, hash, text, text hash; not part of the contract) are the app's: `FileOrigins` (`origins.rs`), one file per docs folder, `<data dir>/freesynergy/fs-wiki/<folder>-<hash>.origins` (or `--origins FILE`), JSON lines, each save one synced append, rewritten atomically (temporary file, fsync, rename) at every start and after a failed append; a torn last line is ignored, a bad line elsewhere is an error and the file is left alone | done, provisional |
| App: tree of pages, open, read with formatting, follow links (also relative `.md` links, resolved **in the app**, `source/resolve.rs`), back, search by name and content (`Texts::find`, first 200 hits) | done |
| App: edit (Markdown text of the page; save = `import_external` against the stored document, then `Texts::edit_if` with the revision read, so layers stay and a second writer gets a conflict instead of being overwritten; the database is the truth, no file is written). Revision = the text's write counter. The lock is inside one process: two programs on one store file are not covered | done; typing in the real window not verified |
| App: unsaved drafts are backed up (`--drafts`, default `~/.local/share/fs-wiki/drafts`; one subfolder per documentation source, so two wikis do not meet; one JSON file per document, named by the hex of its path or by a hash for long paths; our subfolder 0700 and files 0600, a folder given by the person is not changed; unique temporary file, fsync of file and folder, rename; never inside the docs folder): after 0.5 s without typing and at least every 5 s while typing (a closed window does not flush: it loses at most the last pause); on the next edit of that document the person is offered Restore or Discard for the draft; a document that changed meanwhile makes Save report the conflict first; a save or a chosen discard clears it; a failing backup is shown | done; the close-window case is tested by dropping the widget, not with a real close |
| App: explicit Export (`source/export.rs`, `export_fs.rs`): the path is checked as a path (`TextPath`), every step is opened with `openat` and no-follow (a swapped link is met by the open itself), the file is written under a temporary name and renamed (a crash leaves the old or the new file; a hard link elsewhere is replaced, not written through). An existing file with other content is a conflict carrying both texts (`export_markdown_checked`, `WikiSource::last_difference`) and is replaced only after a second press **that answers the question just asked**: the answer is bound to the content shown (hash), so a file that changed after the question is asked about anew (`export_markdown_if_seen`). A page whose Markdown cannot say exactly what the page says (fs-text `ExportReport::inexact`) is asked about first in the same flow, before anything is written ("This file could not be written exactly: block 3 (...): reason"; if the target holds a file with other content, the same question says so and how many lines differ); the number of untouched blocks that had to be written anew (`rewritten`) is added to the answer. A replaced file keeps its mode (a 0600 file stays 0600) and its owner where allowed; temporary names are short and unique. Into the docs folder it records the file as in step and names an old file left by a move (never deleted) | done |
| App tests | the: `map` (also 1000 nested quotes), `resolve` (schemes, colons after the path, root, folders), `import`, `restart` and `conflicts` (file store and real folder: restart after save, move, outside change, both changed, delete), `source`, `save_race` (4 threads), `export`, `export_safety`, `export_page`, `args`, `origins`, `server`; all against the real `fs-wiki-server` started inside the test over a real socket (`tests/common/served.rs`), no stand-in |
| The reader window (`fs-wiki-app`, on `fs-components`) exists; typing in the real window was not verified. other editors, collections / shelves (ADR 0034), named passages' registry, templates in the program | **not started** |

## Structure and pattern map (ADR 0043)

Every file is one concept, <= 200 lines (tests included), `lib.rs` and every `mod.rs` only declare and re-export, every module doc
has a `Pattern:` line. For `fs-text` see its own DESIGN.md.

| Module (`fs-wiki-core/src`) | Pattern | Why |
|---|---|---|
| `feed.rs` | Observer | `TextFeed` / `TextListener` / `Registry` / `Subscription`: the writer does not know who listens and a listener cannot stop a write. (`fs-tasks-core` has the same shape; to move into one shared crate) |
| `repo/feed.rs` | Observer (subject) | `WikiRepo` tells its `Registry` after each store write (`write.rs`, `moving.rs`) |
| `texts.rs` | Repository (interface) + Null Object | `Texts`: what callers depend on; `Unsupported` defaults (ADR 0045) |
| `repo/handle.rs` | Repository + Facade | `WikiRepo` hides store, envelope rules, formats; one write lock |
| `repo/backend.rs` | Adapter | `Arc<dyn Store>` as `dyn ObjectRepository` + `dyn ObjectQuery` |
| `repo/record.rs` | Adapter | path + `Document` <-> `StoredObject` (record, envelope, index fields) |
| `repo/name.rs` | Strategy (fallback order) | the derived name (ADR 0032) |
| `repo/read.rs` | Builder consumer | one `Query` per read, run by the store; a lookup by path has no order |
| `repo/page.rs` | Iterator (resumable) over the Repository | the page after a `Place` as at most three limited queries |
| `repo/upgrade.rs` | Template method | find texts without `path_key` / `id_key`, add them, report |
| `repo/keys.rs` | none (constants) | names of the index fields |
| `page.rs` | Value object | `TextQuery`, `Place`, `Placed`, `TextPage` |
| `reason.rs` | none (plain data) | why a path or move is refused: sentence for people, token for programs |
| `repo/write.rs` | Template method | check path, build, write, read back; `*_locked` flows are reused by move and import |
| `repo/folder.rs` | Command producer, plan then execute | the plan is made from reads, refused or executed in two phases; `MovePrefix` of fs-text is the strategy |
| `repo/moving.rs` | Command producer + Strategy | `rewrite_links_with` yields the rewrite, `MovePage` is the strategy |
| `repo/links.rs` | Read model | rows from the marks, back references from the index |
| `repo/transfer.rs` | Template method | create or outside-change import; export |
| `repo/book.rs` | Facade | thin trait impl, write lock, capability report |
| `media.rs` | Factory | media type -> `TextFormat` -> `fs-text` importer / exporter |
| `path.rs` | Newtype | `TextPath`: only valid paths exist |
| `error.rs` | none | plain error enum |

`fs-wiki-server/src` (trait first: the generated `texts::Server` of the contract and the core traits `Texts` + `TextFeed`; the server adds no trait of its own):

| Module | Pattern | Why |
|---|---|---|
| `texts_service.rs` | Adapter + Facade | the root capability `TextsServer`; depends on `dyn Texts` + `dyn TextFeed`, never on `WikiRepo`; each method forwards to `reads` / `writes` |
| `reads.rs`, `writes.rs` | Adapter (+ Command for writes) | wire in, one trait call, wire out; every write runs through `run_to_end`, so a client that goes away cannot leave a move half done |
| `wire.rs`, `links.rs` | Adapter | texts and link rows on the wire; every size checked here (`fits` counts the document's JSON without building it and stops at the limit) |
| `paging.rs`, `cursor.rs` | Adapter / none | the page of the provider cut by weight, written with the cursor of its last place; the cursor as bytes (same shape as `fs-tasks-server`, to move into `fs-interfaces`) |
| `facts.rs` | none (pure function) | the extensions of a problem, from the core error |
| `errors.rs` | Adapter | core errors as RFC 9457 problems; an unknown media type is the explicit `Unsupported` of `text.import` / `text.export` |
| `feed.rs` | Observer + Adapter | `TextFeed` to the shared forwarder; a failed read ends the feed with `ended` |
| `main.rs` | none (thin binary) | options, store, serve until a signal |

`fs-wiki-client/src` (**pattern chosen: Adapter** — `Texts` and `TextFeed` of the core stand in front of the Cap'n Proto client of `texts.capnp`, so the app depends on the traits and not on `WikiRepo`; the connection thread, deadlines and the way back of an answer are the shared `Worker` of `fs-interfaces`; trait first, `connect` and `connect_with` were written before the calls):

| Module | Pattern | Why |
|---|---|---|
| `peer.rs` | Adapter + Facade | `RpcTexts`: the connection a frontend holds; one thread, calls from any thread |
| `remote.rs` | Strategy | the root capability and the handshake (`supportReport`); the only part that is the wiki's own |
| `texts_impl.rs` | Adapter | each method of `Texts` is one call |
| `calls/reads.rs`, `calls/writes.rs` | Adapter | one request in, one reply read back as core values |
| `calls/pages.rs` | Iterator (by hand) | `Paging` over opaque cursors |
| `read.rs` | Adapter | `TextEntry` and `LinkRow` to `TextEntry` and `LinkEntry` |
| `problems.rs`, `quoted.rs` | Adapter | RFC 9457 problems to `Error` (inverse of `errors.rs` of the server) |
| `feed.rs` | Observer + Adapter | one remote subscription per connection tells a local `Registry`; listeners come and go with their `Subscription` |

`fs-wiki-app/src`:

| Module | Pattern | Why |
|---|---|---|
| `import.rs` | Template method | collect, import each, report; the folder is only read |
| `map/` | Adapter (+ Composite builder in `nest.rs`) | `fs-text` document to the neutral blocks: flat blocks with containers become nested lists/quotes; unknown things become plain text |
| `wiki.rs` | Facade | `Texts` from the server and `Origins` from the app's own file, as one value for the sync |
| `origins.rs` | Repository over a log file | the records of imported files, not part of the contract |
| `words.rs` | none | the sentences the app says, each whole with `{named}` places, filled by `say`; code never compares them (the kinds of error are types: `OriginsError`) |
| `origins_log.rs` | none | the file format of the records: parse, atomic rewrite, lock |
| `server.rs` | Factory | the `Texts` of the app come from a running server and nowhere else; the message when there is none |
| `source/wiki.rs` | Adapter over `Texts` | the reader's `DataSource`; sync calls through a runtime handle |
| `source/resolve.rs` | Strategy (pure function) | relative links against the page they are on, `#fragment` and `?query` dropped; only `http`, `https`, `mailto` are opened (parsed scheme), other schemes are shown; a folder opens `index.md`, `README.md`, else its first page |
| `sync.rs`, `report.rs`, `hash.rs`, `store_path.rs` | State (decision table) / Value objects | one file against its record; the report; SHA-256; the default records file of a docs folder |
| `source/export.rs`, `export_fs.rs`, `export_page.rs` | Guard / Template method | the only code that writes a file; no-follow, temp and rename; records and old-file notice |
| `source/problems.rs` | Adapter | failed imports as a page of the neutral model |
| `args.rs` | none (a parser) | no extra crate for the few options |

## Try it by hand (start instructions)

Start (needs a graphical session). The wiki lives in the server, so **start the server first**, in one terminal; it keeps its data in its own folder (`--data DIR`, default below `$XDG_DATA_HOME`) and listens on `--socket PATH` (default `$XDG_RUNTIME_DIR/freesynergy/wiki.sock`). Then start the app in a second terminal; the first start imports the documentation, about 30 s for 1400 pages (later starts only look at what changed), with a summary line on the terminal before the window opens:

```
cp -r /srv/FreeSynergy/fs-documentation/docs/en /tmp/fs-docs-copy
cd /srv/FreeSynergy/fs-wiki
cargo run --release -p fs-wiki-server -- --socket /tmp/fs-wiki.sock --data /tmp/fs-wiki-data      # terminal 1
cargo run --release -p fs-wiki-app -- --socket /tmp/fs-wiki.sock --docs /tmp/fs-docs-copy --export /tmp/fs-docs-copy   # terminal 2
```

Without a server the app stops with that command line in its message; it never starts a store of its own. Stop the server with Ctrl-C when done.

Things to try, in this order (typing and clicking are not verified by tests; this is the check):

1. The window shows the tree of pages left, the start page (`index.md`) right. Open a few folders with the arrow keys and Enter, or by clicking; the page changes.
2. Follow a link in the text (underlined); `Back` (or Alt+Left) returns.
3. Type a word into the search field (for example `kanban`): the sidebar lists hits; click one. Clear the field to get the tree back.
4. Press `Edit`, change a word, press `Save` (or Ctrl+S): the page shows the change. Press `Edit`, type something, then `Cancel`: it asks once (press again to discard).
5. `Edit`, type something, **close the window without saving**, start the app again, open the same page, press `Edit`: a notice offers `Restore` or `Discard` for the draft. Restore brings your text back; nothing is written until you Save.
6. `Export` writes the page as Markdown to `/tmp/fs-wiki-export/<path>`; pressing it again for an existing file asks first. The documentation folder is never written.

What is known to be rough: emoji that Blinc has no glyph for show as boxes; the first start takes about 30 s; a table with many wide columns may reach past the right edge; the editor text area wraps narrower than its box.

## Decisions

* **Links are resolved against their page in the core when a link is judged for a move or indexed** (changed from the first version, which treated every link as a literal path and broke correct relative links): `[[a/b]]` and `![[a/b]]` stay literal; a Markdown link is relative to its page (`/x` from the root). The `link` index holds the paths a link *can* mean, so `backlinks` finds relative links. The core knows the reader's fallbacks as candidates (`x`, `x.md`, `x/index.md`, `x/README.md`): a page move rewrites a link written without `.md` or to the folder of an `index.md` (`repo/target.rs`, `relink.rs`); link reference definitions (stored as plain blocks) are rewritten with them. The app hands `[[a/b]]` to the reader as `/a/b` (`map/inline.rs`), so one rule holds: wiki links from the root, Markdown links from their page. **Stored form:** `Callout::title` is Markdown as written (changed by fs-text; no real data existed in the old plain form, so there is no migration; an old title like `*a*` would now be read as emphasis). Links in callout titles are found by `repo/titles.rs`, which reads the title with the Markdown parser of fs-text, so only real links count (not code spans, escaped brackets or loose text) and anything else stays byte for byte: in the link index, in `backlinks`, and rewritten with moves (also when their page moves); `links(id)` lists them with `at` = `<block>:title`. The reader shows a title that reads as one paragraph as inline Markdown (formatting, no markup), the whole title strong; a title that begins like a list, heading or quote is shown exactly as stored (`map/title.rs`). A move that makes a text the first candidate of a link that meant another (`z.md` in front of `z/index.md`) writes that link in full, so a correct link is never redirected; for this the sources are also looked up by the new path. Interrupted moves with such links end like the undisturbed one (tested at every write). A real link definition such as `[r]: /private-a` is content: the reader shows it (only comment forms are not displayed). A second definition with the same label is a block of its own (fs-text); `definitions.rs` handles it like any other. A link means the first existing candidate (`repo/shift.rs`): only if that text moves does the link change, and a new form that would lead to another existing page is written in full (`b.md`). For this a move reads the paths of all texts once (memory: the paths only). A dangling link into a moved folder still follows it. Limits: percent-encoding is not decoded when matching a moved path. Link reference definitions are found only in blocks fs-text marks `unparsed` (`definitions.rs`; text in `<pre>` or a comment is never rewritten), also with the destination on the next line; a new destination with a space is wrapped in `<...>`; definitions count in the link index. **Not displayed**: link-definition comments (`[//]: # (note)`, `[note]: # (text)`, `[x]: <> (text)`) and HTML comments (`<!-- note -->`) are not shown by the reader and not found by search (`is_hidden_block`); this is no secret: the editor shows them and every export contains them.
* **The app is read, edit, export:** save goes to the database (`Texts::edit`); a file is only written by the explicit Export action (ADR 0042, 0046). A session that only reads and edits leaves the docs folder unchanged (tests `import`, `source`).
* **The database is the truth** (ADR 0042): `WikiRepo` never writes a file. `export_text` returns the text; the caller writes it.
  A file that comes back is an `import_text`, which is a diff, not a replacement.
* **A text's path is part of the record**, and indexed. A move changes the path of one text and the *links* in others, nothing else.
  A path never contains U+10FFFF (it ends the order range that lists a folder, so such a text could not be listed or moved).
* **Derived data is only in the index** (`path`, `link`, `block`): the record can always rebuild it.
* `fs-text` stays free of storage; the core uses its public API only (`Document`, marks, `Op`); other changes there are in its own DESIGN.md.

## Crates

* `fs-objects` (git tag v0.5.0), `fs-interfaces` (git tag v0.45.0; the server with feature `rpc`), `fs-text` (same workspace, path).
* `serde`, `serde_json`, `thiserror`, `async-trait`, `tokio` (sync only); server and client: `capnp`, `capnp-rpc` (the client also `futures`, for the first `subscribe`); app: `sha2`, `rustix` (no-follow file access).

## Open points

Open points of 0.3.0 (honest list):

* `moveFolder` still answers `too-large` after the move was done when the moved texts together exceed one message (the text says how to read them). The cursor-less answer is the contract's.
* `fits` no longer builds the JSON string, but Markdown is still parsed twice per import (once to measure, once in the core): about 0.4 ms per page, small against the store.
* `fs-objects`: an exact index lookup is O(n), and `order_by_index` sorts every candidate before it cuts. Pages stay linear in rows read, but each page query costs the store O(texts left). Reported, not changed.
* `upgrade_index` moves `updated` of a repaired text (it is a write); `revision` and the document stay.
* The cursor module is a copy of the one in `fs-tasks-server`; one shared implementation belongs in `fs-interfaces`.
* `fs-wiki-app` and `fs-text` were not changed. The app is built on `fs-components` v0.2.0 (the tag v0.4.0 that thread 1 may use does not exist on GitHub yet).

First visible part (provisional, for the owner): the name of the reader and of the `fs-components-*` crates; `index.md` as the start page; the sidebar shows page titles, folders their name; import problems as a first tree entry; search results capped at 200; the Export folder is explicit (`--export`), never implied.

Measured on a **copy** of `fs-documentation/docs/en` (1398 pages, 510 folders), release build, 32 cores, **while the machine ran at load 20 to 64, so the numbers mean little** (`cargo run --release -p fs-wiki-app --example real_docs <socket> <copy> <folder> <new path>`, older numbers: they were taken with the core inside the app, before the socket): the first start into an empty store file took 76 s (the same code at load 64: 183 s; the code before this change, load unknown, 44 s in the same kind of run; an unloaded run of the older code gave 27 s in memory). Writing a record per file costs extra on the first start only. **A second start with nothing changed takes 58 ms** (before: 30.4 s, every file was parsed and diffed); a start after a folder move 40 ms and the moved folder does not come back. Moving `interactions/gestures` (76 texts) took 4 to 17 s depending on load. Links leading nowhere (3794 internal links, resolved as the reader does): 104 before a move and 104 after; the code before this change went from 104 to 131 (27 newly broken) and brought the 76 moved texts back at the next start. The store file is 56 MB. **The reader calls the source on the interface thread** (`examples/timing.rs`, older numbers, other machine state): list of all pages 284 ms once at start, `get_by_path` + mapping of a page 20 ms, `find` 34 to 59 ms per search keystroke; not measured again.

Server decisions (provisional, easy to reverse):

* **`links` / `backlinks` answer every link** (contract 0.39): `LinkRow.title` is the link text as written, `internal` is true for a target that is a text of this wiki (it may not exist yet) and false for an outside address (URL, mail). The target stays as written, with `#fragment`. Before 0.3.0 outside links had no row. The lists have no cursor: an answer above one message is `too-large`. `TextEntry.revision` is only compared for equality (`editIf`); it is no counter and no time.
* **`find` with an empty term is refused** (422), not "everything".
* **`list` / `find` pages are cut in the store** (0.3.0). The cursor is a place (form byte 2 + sort key, name, id), not a position: a text added or removed elsewhere changes neither what was read nor what comes next. A cursor of the old form (byte 1) is refused as `invalid-request`. The query is ordered by `path_key`, because the store's conditions compare the raw value and its order uses the folded value; with the folded value stored as the value both agree. Two paths that fold to the same key (`Alpha`, `alpha`) are told apart by name, then id. The page also stops by weight (`fitting`); an item above one message is `too-large` with the cursor past it.
* **Why the import of 1398 files did not finish (found 2026-11-07).** `text_at` (path lookup before every create / import) and `aux_get` (the file record) were queries with `order_by_index`: the store computes the sort key of every text for each lookup, so a lookup costs O(n) and an import O(n^2) (debug build, whole documentation: minutes). A path or a key names one object, so both lookups are now unordered with `limit(1)`. Measured on 50..400 real files of `fs-documentation` (debug, loaded machine): import 50 -> 1.7 s, 100 -> 4.1 s, 200 -> 11.4 s, 400 -> 37.8 s before; 100 -> 2.0 s, 200 -> 3.6 s, 400 -> 8.4 s after (parse alone: 0.18 s for 400). What remains is the store: an exact index lookup is still O(n) there (about 2.6 ms at 100 texts, 5.5 ms at 400), reported for `fs-objects`, not changed here. `moveFolder` and `backlinks` use ordered index queries too (one per move, not per text).
* **The server parses on the thread that serves all connections.** Hence the 4 MiB limit on Markdown. A document is large (one-letter paragraphs: 105 times the Markdown), which `fs-text`'s serde form could shrink (thread 1).
* **`moveFolder` is answered with all moved texts**, as the contract says; above one message it answers `too-large` after the move was done.
* **`media_type`**: only `text/markdown` is known; any other is `Unsupported` of that method, though the report lists the capability as offered.
* **Problem extensions** (keys of `fs-interfaces` `problem_keys`; the detail text is unchanged; `server/facts.rs`):

  | Problem | status | `id` | `kind` | `issue` | `path` | `size` / `limit` |
  |---|---|---|---|---|---|---|
  | `texts/not-found` | 404 | the text | `page` | | | |
  | `texts/not-found` (no text at or below a folder) | 404 | | `folder` | | the folder | |
  | `texts/conflict` | 409 | the text | `page` | | | |
  | `texts/path-taken` | 409 | | `page` | | the taken path | |
  | `texts/move-refused` | 409 | | `folder` | | the target in the way (unfinished move: the folder) | |
  | `texts/invalid` (path) | 422 | | `page` | token of `Reason` (`empty`, `fragment`, `empty-segment`, ...) | the path | |
  | `texts/invalid` (folder move) | 422 | | `folder` | `inside-itself`, `inside-old-folder` | the folder | |
  | `texts/invalid` (request) | 422 | | | `path-empty`, `term-empty`, `document-unreadable`, `media-type` ... | | |
  | `too-large` (source, document, moved texts) | 413 | | | | | bytes / limit |

* **`CONTRACT_VERSION`** is `texts 0.1 (draft)` (the schema header says DRAFT 0.1).

Questions for the owner (all provisional, easy to reverse):

* **One server is one wiki.** The app has no store of its own any more, so two documentation folders in one server would share one text namespace. Use one server (own `--data` and `--socket`) per documentation source, or tell me if the app should prefix paths per folder.
* **Records and servers (provisional; thread 1 decided).** The records file is not tied to a server. Later `SupportReport.instance :Uuid` in `common.capnp` (from an identity in `fs-objects`, built by thread 3) is to say which wiki a server is. It will solve "every text of a folder was deleted outside the app, so the files come back at the next start", but **not** "a server restored from a backup that has only some of the texts": that has the same id, and stays **open**. What is done now:
  * The check at every start asks the server, once per record, whether the text is there (`get`, 64 at once on the one connection, the whole check bounded by 60 s; a stuck server ends the start with a failure, tested). A listing would be one call, but it carries every document and took 5.5 s for 1398 pages; so the cost is about one second per 3000 records. **Measured** on a copy of the documentation (1398 pages, release build): first start 60.5 s; second start **about 390 ms** (375, 402, 391 ms; 16 ms before the check existed; 542 ms with one `get` after the other). The goal of 100 ms is not reachable with this contract: it needs a call that answers "which of these ids exist" without documents (wished for from the strategy: `exists(ids)` or a list of ids and paths).
  * One `get` that fails does not stop the start: that text is taken to be there (as the records say) and the file is reported as its own entry on the problems page ("not imported" list, with the reason). The check stops with a message only when it runs past the 60 s, or when **every** `get` of at least two records fails (the server is not there; with one record a failed `get` cannot tell, so that file is reported like any other and the import goes on; tested: `the_only_record_whose_read_fails_does_not_stop_the_import_either`). Each answer is reduced to a yes or no at once, so no document is kept in memory (tested: `check_single`).
  * **Whose records they are is the store's identity, not a guess.** The records file holds a first line `{"store":"<uuid>"}` (`Origins::recorded_store` / `belong_to`; `Texts::identity` of the server, from `SupportReport.instance`, which `RpcTexts` reads at the handshake). The identity of the server differs from the recorded one: another wiki; the records are ignored (`Report::records_ignored`, said on the terminal), dropped from the file, and every file is imported again. The same store whose texts were all deleted **outside** the app is the same wiki: the files are listed as missing and not imported again (tested). Records from before the line existed are kept and from then on belong to the store that is connected: **a file of that kind that belongs to another server is not recognised** (its files are listed as missing). A server without a store (`identity()` is `None`) is trusted. The old rule ("the server holds none of the texts the records name") is gone. A restored copy of a store keeps its id and stays as described below.
  * A text is missing while others are there: the file is **not** counted as unchanged and **not** imported again; it is listed in `Report::missing` and on the problems page ("Missing on the server"), and the terminal says how many (tested with a restored copy and with a text deleted outside the app). It stays listed at every start, because there is no screen yet where the person can confirm it. A text deleted through `Wiki::delete` marks its record (`FileOrigin::deleted`): that file counts as unchanged and is not listed. **Today this does not apply in the real program:** the app has no way to delete a text (the reader's `DocumentSource` has no delete), so only code that calls `Wiki::delete` (the tests, a later screen) uses it. If the text is deleted and the record cannot be updated, the error says so ("deleted on the server, record not updated"; the text is gone and the file will be listed as missing). For texts deleted outside the app, or lost by a restored copy, the list stays until the file is changed (then it is a conflict, as before) or the record is removed by hand.
* **Dead connection and the feed.** Only a connection that is really gone is dead: the server closed it (`Disconnected`) or the connection thread ended (`Fault::Gone`). Then no feed can be made on it: `subscribe` tells the new listener `ended` (not inside `subscribe`, but from a thread of its own, one listener at a time with 100 ms between) once per listener object, so a listener that subscribes in `ended`, even a fresh object each time, hears it at a bounded pace and never recurses (tested on a thread with a 256 KiB stack; without the fix the loop ran about 550 000 times in 3 s). A dead connection never revives: make a new `RpcTexts`. A subscription that fails otherwise (the deadline runs out, the server refuses) fails that one attempt: the listeners are told `subscription failed: ...`, the connection stays usable and the next `subscribe` tries again (tested with a busy server and a 1 s deadline). Subscribing from inside a listener callback (the connection's own thread) cannot wait there; another thread waits and settles the result.
* **Open pattern questions.** `fs-wiki-client/src/feed_job.rs` is named Observer + Command, but it is free functions with effects (mark dead, end the feed, start a delivery thread); `fs-wiki-app/src/origins_log.rs` is `Pattern: none` and is free functions on a file (parse, atomic rewrite, lock) with effects. If the table of coding.md §2 has a better name for "the storage of a Repository, kept in a file of its own" or for "delivery of a notice from a queue", these are the two to rename.
* **Open: sentences of the app in code (§2a).** The sentences of the server connection, the records and the check are in `words.rs` (`say` fills named places in one pass). Still in the code, to move one by one: those in `source/problems.rs`, `sync.rs`, `import.rs` (the "cannot read the record of earlier imports" and the gone-file reasons), `drafts.rs`, `drafts_orphans.rs`, `source/export*.rs`, `source/wiki.rs`, `source/resolve.rs` and `main.rs` (the conflict reason for orphan drafts, the stats line).
* **Limit: the client reads problems from their text.** `fs-wiki-client/src/problems.rs` tells some variants apart by the text of the problem (`the move of ...`, `invalid path ...`, `unknown media type ...`), because the server sends only a type, a status and a text. Wished for from the strategy: problem types of their own in the server for these cases (`texts/move-unfinished`, `texts/target-not-empty`, `texts/invalid-path`, `texts/media-type`), so that no text is compared.
* **Records lock.** `FileOrigins::open` takes an exclusive `flock` on `<records>.lock`: a second app on the same docs folder is refused at its start (`OriginsError::Busy`, a type of its own; the lock is held until the last write of the file has ended, also after the value was dropped). The helper files (`.tmp`, `.lock`) are named after the whole file name, so `a.origins` and `a.json` do not share them. Two apps on different docs folders use different files, but if they use the same server their texts share one namespace. The write and sync of a record run on a blocking thread (`spawn_blocking`).
* **Folder move answered `too-large`.** The server has done the move when its answer would not fit; the client then reads the state (nothing left at the old place, texts at the new one) and returns the moved texts, as the core does. If the state says the move did not happen, the error stays.
* **Paging under writes.** The cursor is a position in the list at the time of the call, not a snapshot. A text created, deleted or moved between two pages of one `list` / `find` can make an entry appear twice or be missed in that one answer. Read again if it matters.
* **Records file** (`FileOrigins`): JSON lines next to the app's data, one file per docs folder; the simplest thing that survives a restart and writes atomically. A different place (a store object of the server) would need a contract method for it. Two programs on one file: refused (lock, above).
* **A folder link without an index page** opens the first page of the folder (by path); a folder with no page is Missing.
* **A conflict** (file and text both changed) keeps the database text and shows the file's version on the problems page every start until the text equals the file; there is no "take the file" button yet.
* **A file that is gone** keeps its text and is listed; its record stays.
* **Export into the docs folder after a move** leaves the old file and says so.
* **A file emptied outside** (or holding only a byte order mark) while its text has content is a conflict, not an edit.
* **Drafts** of a page that moved or was deleted are listed on the problems page (never lost, not moved automatically); an empty or relative `XDG_DATA_HOME` is treated as unset.
* **Hidden folders** (`.git`) and non-Markdown files are skipped without a note; symbolic links, `.MD` and unreadable entries are listed.

* **Stored media type** `application/vnd.freesynergy.text+json` and the record shape `{ path, document }`: ADR 0034 asks for a
  neutral name for the block specification; this is a placeholder until it exists.
* **`LinkEntry.at`** is `<block id>:<start>-<end>` (char offsets). The schema only says "where in the source".
* **Envelope type:** every text is an `Article`; the schema also names `Note`. When is a text a `Note` (short, no heading)?
* **Original of an imported file** (ADR 0030) is not kept: only its hash and the hash of the text at that time (`FileOrigin`). A conflict shows the file's current version once per start, until a person resolves it (a save that makes the text equal the file clears it).

Not done / limits:

* **Uniqueness of `path`** holds inside one process (write lock). Two programs writing the same store at once can still race; the
  store has no unique index (a wish for fs-objects).
* **A move is several writes** (other texts, then the moved text); `fs-objects` 0.2 has no transaction over several objects. Repeating
  an interrupted move finishes it. Each write is one undo step.
* **A folder move** (`move_folder`) is `n` writes, in this order: (1) every text outside the folder that links into it, found by the
  `link` index (equal to the folder or in the range `folder/`), (2) the texts of the folder, shallow ones first, each once with its
  new path and its links already rewritten. Before the first write the whole plan is checked from reads (a target taken anywhere:
  `PathTaken`; target inside the folder: `InvalidMove`; no text at or below: `NoSuchFolder`; same path: nothing, the texts are returned).
  What a reader sees in between: during (1) other texts already link to the new paths, which do not exist yet (dangling for a
  moment); during (2) some texts are at the new path, the rest still at the old one, and a link between a moved and an unmoved text
  points to the new path, so it is dangling until the end. No text is missing or twice at any time: each write replaces one text
  under the same id. If the call fails in the middle, the state stays like that until the same call is repeated; it plans again from
  the store and does what is left, nothing twice. Tested by failing the store after every number of writes.
  Limits: a target that would put a moved text inside the old folder (moving `a/b` to `a` while `a/b/b/x` exists) is refused, because
  after an interruption a moved text could not be told from an unmoved one. Two programs on one store can interleave (no lock
  across processes). Planning keeps only ids and paths of the folder in memory (read 500 at a time with offset paging, O(offset));
  the outside texts linking in are read in one query each for `folder` and `folder/...`.
* **Undo of a folder move:** the store undoes one write per step (`undo(actor, n)`), not one move. Undoing all `n` writes (the outside
  texts and the moved texts) restores paths and documents; undoing fewer leaves a half moved folder, the same state as an interrupted
  move, and repeating `move_folder` finishes it again. Tested both ways. A single "undo the folder move" needs a group of activities
  in fs-objects (a wish).
* **Deleting a text** leaves links to it in other texts (dangling, like in any wiki); they are shown as links to a path with no text.
* `LinkValue.target` (UUID of the target) is not filled; links resolve by path.
* `find` is substring only (no tokens, no ranking) and also searches code blocks.
* **Client limits.** `Texts::page` (a page after a place) cannot use the server's cursor, which is a place the client cannot make: `RpcTexts::page` reads the server's pages from the start and drops the texts up to the place (tested against the provider itself, `tests/place.rs`); `list` / `find` cost a full read for each page as before. `LinkRow.title` and `internal` are read as the server sends them (fs-interfaces v0.45.0). `TextFeed::subscribe` answers `Unsupported` when the server's report lacks `text.subscribe`. `InvalidMove` is read from the `issue` extension of the problem (the reason's token), the prose is only a fallback. `too-large` has no variant in `Error` and arrives as `Store(Invalid("too large: ..."))`. A feed cannot be dropped per listener on the server: dropping a `Subscription` removes the listener locally, and the one remote subscription runs until the connection ends. The first `subscribe` blocks for one round trip; from inside a listener callback (the connection's own thread) it only queues the request, so it cannot lock the connection. Each remote subscription has a generation: the late end of an old one does not end a newer one. A `ChangeListener.ended` or a lost connection is told to every listener through the new `TextListener::ended(reason)` (default: nothing; `Registry::end`), after which the next `subscribe` starts a new remote subscription; changes may have been missed, read the state again.
* **Core change for the client:** `TextListener::ended` and `Registry::end` (additive, tested in `fs-wiki-core/tests/feed.rs`). `fs-interfaces` is v0.45.0 in all crates (shared client `Worker`).
* Not verified: a failed append of the records file (no way to make the disk fail in a test); typing in the real window against a server.
* `TextFeed::subscribe` returns `Result<Subscription, Unsupported>`: a provider whose report lacks `text.subscribe` says so (tested with a fake feed).
* Offset paging (`Paging`) costs O(offset) in the store; the wire `Cursor` will wrap the offset.

## Markdown round trip

The property `markdown_model_is_stable` (import, export, import gives the same content) holds for the inputs the test generates (a fixed set of fragments, not all of Markdown); the check on every file of the documentation tree is `tests/real_docs.rs`, which is `--ignored` and was not run for this section. Real text stays clean: `2 * 3`, `snake_case` and `a_b` get no backslashes.

* **Runs of delimiters.** CommonMark pairs `*`/`_` runs by flanking and the rule of three, which no
  local escaping rule predicts. `inline` therefore writes the plain, escaped form first and reads
  it back; only when the text or the emphasis differs, other writing variants are tried (`variant`:
  a literal delimiter char joins the run before an opener or after a closer, an emphasis keeps its
  `*` behind a `*`) and the first one that reads back right wins. Text that already reads back is
  untouched. A `_` span stays `_` when punctuation (a link end, a `*` closer) stands next to it.
* **Empty quote.** `>` is something the person wrote, so it is a block: an empty paragraph inside the
  quote (like an empty list item). It also keeps the lists around it apart.
* **Adjacent lists.** Two lists of one type with no block between them (for example around a link
  definition, which is not a block) would read back as one. The export puts `<!-- -->` between them,
  the usual way to say it in Markdown and invisible in other tools. The importer does not keep this
  comment as a block when it stands between two lists, because the model already says "two lists"
  (a different list marker needs no comment). Every other comment stays an HTML block.
* **Loose lists** are also recognised by a blank line before the next item (`*`, `* #`), not only by
  a paragraph in an item. A nested tight list in a loose item gets its blank line.
* **Thematic break** behind a `-` marker is written `***` (`- ---` is one rule, no item).
* **Wide table rows.** Cells beyond the header width are dropped by the parser; the importer reads them
  from the row's source and widens the table (empty header cells), so no text is lost.

`tests/real_docs.rs` (`--ignored`) imports and exports every file of the documentation tree and
checks that no word is lost and a second round is stable. Link reference definitions
(`[x]: /url`) are not part of the model and are still not written back.

## After the sprint (strategy list, 2026-11-07)

* `cursor.rs` is a copy of `fs-tasks-server`'s: move it to `fs-interfaces` (`rpc`).
* `issue` values from `Store(Invalid(reason))` are free store text, a sentence instead of an identifier (§2a): map them to identifiers.
* Exact index search in `fs-objects` is still O(n) (see `fs-objects` DESIGN.md): the import is faster (400 files 37,8 s to 8,4 s in the debug build) but not linear.
