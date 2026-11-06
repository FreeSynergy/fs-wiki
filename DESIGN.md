# fs-wiki - Design

`fs-wiki` is the wiki program of FreeSynergy and the **owner of the text interface** (ADR 0031, 0050): every text is a wiki
entry, a collection of texts is a wiki. The database is the truth, files are exports (ADR 0042). The contract is `texts.capnp`
in [`fs-interfaces`](https://github.com/FreeSynergy/fs-interfaces) (ADR 0045); this program is its provider. This repo holds
three crates; the GUI lives only in `fs-wiki-app`, on [`fs-components`](https://github.com/FreeSynergy/fs-components) (Blinc):

* `crates/fs-text`: the **library** of text + layers (document, marks, Markdown, outside changes, links, templates, history).
  No storage, no interface. Its own design is in [`crates/fs-text/DESIGN.md`](crates/fs-text/DESIGN.md).
* `crates/fs-wiki-app`: the **first visible part**, a thin shell: imports a folder of Markdown files, maps texts to the neutral
  blocks of `fs-components`, and offers the wiki to the reader workbench as its data source. No Blinc type appears in it.
* `crates/fs-wiki-core`: the **program core**. The trait `Texts` (Rust form of `Texts` in `texts.capnp`) and `WikiRepo`, its
  implementation over `fs-store`.

## Interface status

**Interface status: draft, consumers: none (0 of 2).** (ADR 0045: the interface is a target; it stays draft until two independent
consumers have tested it, then 1.0, then only version jumps.) Nothing here is declared 1.0.

## State

Version 0.2.0 (workspace; the tag v0.2.0 is set by the owner). Built on `fs-store` v0.2.0, `fs-interfaces` v0.23.0 and `fs-components` v0.1.0 (git tags).

| Area | State |
|---|---|
| `Texts` trait: required `capabilities`, `get`, `get_by_path`, `list`; every other method of the contract defaults to `Unsupported` (`create`, `edit`, `move_to`, `delete`, `find`, `links`, `backlinks`, `import_text`, `export_text`) | done |
| `WikiRepo` over `ObjectRepository` + `ObjectQuery` (`WikiRepo::new(Arc<dyn Store>, actor)`, `with_backend`, `with_instance`) | done |
| Capability report: all ids of the schema except `text.subscribe` ("not built yet; it is possible") | done |
| Envelope: AS2 `Article`, `context=texts`, IRI `https://<instance>/texts/<id>`, attribution, **derived name** (ADR 0032): front matter `title:`, else first heading, else first line of the first text block, else last path segment | done |
| Stored content: the record `{ path, document }` as JSON, media type `application/vnd.freesynergy.text+json` (provisional). The JSON form of `Document` is `fs-text`'s serde form (round trip of every mark kind tested in `fs-text/tests/serde_form.rs`) | done |
| `path` unique: checked in the store before every write under a per-repo write lock; a clash is `Error::PathTaken`; invalid paths are `Error::InvalidPath` (`TextPath`) | done (in-process only, see open points) |
| `create(path, markdown)`, `edit(id, document)` (id of the document forced to the text id; envelope, IRI and `published` stay), `delete` (the text and its layers; undoable through the store) | done |
| `list(prefix, page)`: one store query on the index `path`, **exact, case sensitive prefix** (text order range), path order, paged by the store | done |
| `find(term, page)`: one store query: `Contains` on the name, on the path and on the index `block` (plain text of each block) | done |
| Link table: marks are the truth; `links(id)` = `extract_links` of the stored document (`LinkEntry`: source, target path as written, place `<block>:<start>-<end>`, title, internal); `backlinks(path)` = one store query on the index `link` (page part of every internal link), then the rows of the texts found | done |
| `move_to(id, path)`: other texts that link to the old page are rewritten (exact page, `#fragment` kept, links to `old/...` stay); the moved text last, its own relative links written again for its new place. A link is first resolved against the page it stands on (`repo/target.rs`): `[[a/b]]` is a literal path, `[x](../a/b.md)` is relative to its page; a rewritten link keeps its written form (relative stays relative, `./`, trailing `/` and `#fragment` kept). `MovePrefix` of fs-text sees neither the page nor the style, so `repo/relink.rs` does this with the same ops | done, tested (also links inside the moved text, retry after a stop half way) |
| `move_folder(prefix, path)` (`moveFolder` of `texts.capnp`, capability `text.move`): the text at `prefix` and all `prefix/...` move, links in all texts follow (also between the moved ones and links inside moved pages whose position changed; `manual-old` is not below `manual`). A record of the last move (`repo/move_log.rs`) makes a repeat after a completed move answer with the moved texts and write nothing, refuses a move onto texts that are not part of this very move (`TargetNotEmpty`; no silent merge), and refuses another move while one is unfinished (`MoveUnfinished`; no split folder) | done, tested on a real `TursoStore` (plan, clash, interruption at every write, undo, 510 texts) |
| `import_text(path, media_type, text)`: new path = create; existing path = outside change: `import_external` diff becomes ordinary ops, layers move with the text, deleted text takes its marks along, unchanged file writes nothing | done, tested |
| `export_text(id, media_type)`: Markdown, layers not written | done |
| `edit_if(id, revision, document)` (`editIf` of `texts.capnp` 0.23, `TextEntry::revision`): compare-and-set under the write lock; a stale writer gets `Error::Conflict` and nothing is written. The revision is a counter in the stored record (every write adds one), not a time, so a clock that steps back cannot reuse one. Atomic **inside one process** only | done, tested with racing tasks |
| `Origins` (`FileOrigin`: file, hash of the file, text id, hash of the text's Markdown): what a file looked like when last imported or exported, stored next to the texts. The core keeps it, the app decides with it | done |
| Media types (`TextFormat`, Factory): `text/markdown` (parameters and case ignored); anything else is `Error::MediaType` | done |
| Example `cargo run -p fs-wiki-core --example manifesto`: imports `fs-documentation/docs/en/manifesto.md`, reads it back (identical), lists its links | done |
| Tests | `fs-wiki-core`: the integration tests (`texts_crud`, `texts_list`, `links`, `move_text`, `move_folder`, `move_folder_failure`, `move_safety`, `relative_links`, `edit_if`, `outside_change`, `provider`); `fs-text` has its own |
| `fs-wiki-app` (binary): `--docs` (default `../fs-documentation/docs/en`), `--store` (a file, or `memory`; default one file per docs folder in the data directory), `--export` (enables the Export button), `--snapshot` + `--step` (a picture without a window, see below) | done |
| App: sync of every `*.md` under the folder (`import.rs`, `sync.rs`), path = relative path. A record per file (hash, text, hash of the text) decides: file as recorded = nothing happens and the file is not parsed, whatever the database holds (a saved, moved or deleted text stays so); file changed and text as recorded = normal edit (ADR 0042 section 5); both changed = the database text stays, the file's version is a conflict shown on the problems page; new file = import; file gone = reported, text kept. Failures, conflicts, gone files and skipped entries (symbolic links, `.MD`, unreadable entries) are the first tree entry "Import problems (n)" | done |
| App: store: by default one file per docs folder, `<data dir>/freesynergy/fs-wiki/<folder>-<hash>.db` (`$XDG_DATA_HOME` or `~/.local/share`), so a save survives the exit; `--store memory` keeps nothing and says so at start | done, provisional |
| App: tree of pages, open, read with formatting, follow links (also relative `.md` links, resolved **in the app**, `source/resolve.rs`), back, search by name and content (`Texts::find`, first 200 hits) | done |
| App: edit (Markdown text of the page; save = `import_external` against the stored document, then `Texts::edit_if` with the revision read, so layers stay and a second writer gets a conflict instead of being overwritten; the database is the truth, no file is written). Revision = the text's write counter. The lock is inside one process: two programs on one store file are not covered | done; typing in the real window not verified |
| App: explicit Export (`source/export.rs`, `export_fs.rs`): the path is checked as a path (`TextPath`), every step is opened with `openat` and no-follow (a swapped link is met by the open itself), the file is written under a temporary name and renamed (a crash leaves the old or the new file; a hard link elsewhere is replaced, not written through). An existing file with other content is a conflict carrying both texts (`export_markdown_checked`, `WikiSource::last_difference`) and is replaced only after a second press. Into the docs folder it records the file as in step and names an old file left by a move (never deleted) | done |
| App tests | the: `map` (also 1000 nested quotes), `resolve` (schemes, colons after the path, root, folders), `import`, `restart` and `conflicts` (file store and real folder: restart after save, move, outside change, both changed, delete), `source`, `save_race` (4 threads), `export`, `export_safety`, `export_page`, `args` |
| The reader window (`fs-wiki-app`, on `fs-components`) exists; typing in the real window was not verified. capnp-rpc server, other editors, collections / shelves (ADR 0034), named passages' registry, templates in the program | **not started** |

## Structure and pattern map (ADR 0043)

Every file is one concept, <= 200 lines (tests included), `lib.rs` and every `mod.rs` only declare and re-export, every module doc
has a `Pattern:` line. For `fs-text` see its own DESIGN.md.

| Module (`fs-wiki-core/src`) | Pattern | Why |
|---|---|---|
| `texts.rs` | Repository (interface) + Null Object | `Texts`: what callers depend on; `Unsupported` defaults (ADR 0045) |
| `repo/handle.rs` | Repository + Facade | `WikiRepo` hides store, envelope rules, formats; one write lock |
| `repo/backend.rs` | Adapter | `Arc<dyn Store>` as `dyn ObjectRepository` + `dyn ObjectQuery` |
| `repo/record.rs` | Adapter | path + `Document` <-> `StoredObject` (record, envelope, index fields) |
| `repo/name.rs` | Strategy (fallback order) | the derived name (ADR 0032) |
| `repo/read.rs` | Builder consumer | one `Query` per read, run by the store |
| `repo/write.rs` | Template method | check path, build, write, read back; `*_locked` flows are reused by move and import |
| `repo/folder.rs` | Command producer, plan then execute | the plan is made from reads, refused or executed in two phases; `MovePrefix` of fs-text is the strategy |
| `repo/moving.rs` | Command producer + Strategy | `rewrite_links_with` yields the rewrite, `MovePage` is the strategy |
| `repo/links.rs` | Read model | rows from the marks, back references from the index |
| `repo/transfer.rs` | Template method | create or outside-change import; export |
| `repo/book.rs` | Facade | thin trait impl, write lock, capability report |
| `media.rs` | Factory | media type -> `TextFormat` -> `fs-text` importer / exporter |
| `path.rs` | Newtype | `TextPath`: only valid paths exist |
| `error.rs` | none | plain error enum |

`fs-wiki-app/src`:

| Module | Pattern | Why |
|---|---|---|
| `import.rs` | Template method | collect, import each, report; the folder is only read |
| `map/` | Adapter (+ Composite builder in `nest.rs`) | `fs-text` document to the neutral blocks: flat blocks with containers become nested lists/quotes; unknown things become plain text |
| `source/wiki.rs` | Adapter over `Texts` | the reader's `DataSource`; sync calls through a runtime handle |
| `source/resolve.rs` | Strategy (pure function) | relative links against the page they are on, `#fragment` and `?query` dropped; only `http`, `https`, `mailto` are opened (parsed scheme), other schemes are shown; a folder opens `index.md`, `README.md`, else its first page |
| `sync.rs`, `report.rs`, `hash.rs`, `store_path.rs` | State (decision table) / Value objects | one file against its record; the report; SHA-256; the default store file of a docs folder |
| `source/export.rs`, `export_fs.rs`, `export_page.rs` | Guard / Template method | the only code that writes a file; no-follow, temp and rename; records and old-file notice |
| `source/problems.rs` | Adapter | failed imports as a page of the neutral model |
| `args.rs` | Builder-like parser | no extra crate for seven options |

## Decisions

* **Links are resolved against their page in the core when a link is judged for a move or indexed** (changed from the first version, which treated every link as a literal path and broke correct relative links): `[[a/b]]` and `![[a/b]]` stay literal; a Markdown link is relative to its page (`/x` from the root). The `link` index holds the path a link *means*, so `backlinks` finds relative links. What the app opens (`.md`, `index.md`, folders) is still decided in `source/resolve.rs`. Percent-encoding is not decoded when matching a moved path (a link with `%20` to a moved page is not rewritten); a question for the owner whether the core should know `.md`/`index.md` fallbacks.
* **The app is read, edit, export:** save goes to the database (`Texts::edit`); a file is only written by the explicit Export action (ADR 0042, 0046). A session that only reads and edits leaves the docs folder unchanged (tests `import`, `source`).
* **The database is the truth** (ADR 0042): `WikiRepo` never writes a file. `export_text` returns the text; the caller writes it.
  A file that comes back is an `import_text`, which is a diff, not a replacement.
* **A text's path is part of the record**, and indexed. A move changes the path of one text and the *links* in others, nothing else.
  A path never contains U+10FFFF (it ends the order range that lists a folder, so such a text could not be listed or moved).
* **Derived data is only in the index** (`path`, `link`, `block`): the record can always rebuild it.
* `fs-text` stays free of storage; the core uses its public API only (`Document`, marks, `Op`); other changes there are in its own DESIGN.md.

## Crates

* `fs-store` (git tag v0.2.0), `fs-interfaces` (git tag v0.23.0), `fs-text` (same workspace, path).
* `serde`, `serde_json`, `thiserror`, `async-trait`, `tokio` (sync only); app: `sha2`, `rustix` (no-follow file access).

## Open points

First visible part (provisional, for the owner): the name of the reader and of the `fs-components-*` crates; `index.md` as the start page; the sidebar shows page titles, folders their name; import problems as a first tree entry; search results capped at 200; the Export folder is explicit (`--export`), never implied.

Measured on a **copy** of `fs-documentation/docs/en` (1398 pages, 510 folders), release build, 32 cores, **while the machine ran at load 20 to 64, so the numbers mean little** (`cargo run --release -p fs-wiki-app --example real_docs <copy> <folder> <new path>`): the first start into an empty store file took 76 s (the same code at load 64: 183 s; the code before this change, load unknown, 44 s in the same kind of run; an unloaded run of the older code gave 27 s in memory). Writing a record per file costs extra on the first start only. **A second start with nothing changed takes 58 ms** (before: 30.4 s, every file was parsed and diffed); a start after a folder move 40 ms and the moved folder does not come back. Moving `interactions/gestures` (76 texts) took 4 to 17 s depending on load. Links leading nowhere (3794 internal links, resolved as the reader does): 104 before a move and 104 after; the code before this change went from 104 to 131 (27 newly broken) and brought the 76 moved texts back at the next start. The store file is 56 MB. **The reader calls the source on the interface thread** (`examples/timing.rs`, older numbers, other machine state): list of all pages 284 ms once at start, `get_by_path` + mapping of a page 20 ms, `find` 34 to 59 ms per search keystroke; not measured again.

Questions for the owner (all provisional, easy to reverse):

* **Default store** is a file per docs folder in the user's data directory (`--store memory` keeps the old behaviour).
* **A folder link without an index page** opens the first page of the folder (by path); a folder with no page is Missing.
* **A conflict** (file and text both changed) keeps the database text and shows the file's version on the problems page every start until the text equals the file; there is no "take the file" button yet.
* **A file that is gone** keeps its text and is listed; its record stays.
* **Export into the docs folder after a move** leaves the old file and says so.
* **Hidden folders** (`.git`) and non-Markdown files are skipped without a note; symbolic links, `.MD` and unreadable entries are listed.

* **Stored media type** `application/vnd.freesynergy.text+json` and the record shape `{ path, document }`: ADR 0034 asks for a
  neutral name for the block specification; this is a placeholder until it exists.
* **`LinkEntry.at`** is `<block id>:<start>-<end>` (char offsets). The schema only says "where in the source".
* **Envelope type:** every text is an `Article`; the schema also names `Note`. When is a text a `Note` (short, no heading)?
* **Original of an imported file** (ADR 0030) is not kept: only its hash and the hash of the text at that time (`FileOrigin`). A conflict shows the file's current version once per start, until a person resolves it (a save that makes the text equal the file clears it).

Not done / limits:

* **Uniqueness of `path`** holds inside one process (write lock). Two programs writing the same store at once can still race; the
  store has no unique index (a wish for fs-store).
* **A move is several writes** (other texts, then the moved text); `fs-store` 0.2 has no transaction over several objects. Repeating
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
  in fs-store (a wish).
* **Deleting a text** leaves links to it in other texts (dangling, like in any wiki); they are shown as links to a path with no text.
* `LinkValue.target` (UUID of the target) is not filled; links resolve by path.
* `find` is substring only (no tokens, no ranking) and also searches code blocks.
* `subscribe` (`text.subscribe`) is not built; the capnp-rpc server is not written.
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
