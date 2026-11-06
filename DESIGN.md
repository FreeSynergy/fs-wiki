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

Version 0.1.x (the app crate is new and untagged until reviewed). Built on `fs-store` v0.2.0 and `fs-interfaces` v0.4.0 (git tags).

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
| `move_to(id, path)`: other texts that link to the old page are rewritten with `rewrite_links_with` (exact page, `#fragment` kept, links to `old/...` stay); the moved text last | done, tested (also links inside the moved text, retry after a stop half way) |
| `import_text(path, media_type, text)`: new path = create; existing path = outside change: `import_external` diff becomes ordinary ops, layers move with the text, deleted text takes its marks along, unchanged file writes nothing | done, tested |
| `export_text(id, media_type)`: Markdown, layers not written | done |
| Media types (`TextFormat`, Factory): `text/markdown` (parameters and case ignored); anything else is `Error::MediaType` | done |
| Example `cargo run -p fs-wiki-core --example manifesto`: imports `fs-documentation/docs/en/manifesto.md`, reads it back (identical), lists its links | done |
| `fs-wiki-app` (binary): `--docs` (default `../fs-documentation/docs/en`), `--store` (file; default in memory), `--export` (enables the Export button), `--snapshot` + `--step` (a picture without a window, see below) | done |
| App: import of every `*.md` under the folder, path = relative path; failures collected with their reason and shown as the first entry "Import problems (n)" | done |
| App: tree of pages, open, read with formatting, follow links (also relative `.md` links, resolved **in the app**, `source/resolve.rs`), back, search by name and content (`Texts::find`, first 200 hits) | done |
| App: edit (Markdown text of the page; save = `import_external` against the stored document, then `Texts::edit`, so layers stay; the database is the truth, no file is written) and explicit Export (writes `<export folder>/<path>`) | done; typing in the real window not verified |
| App tests | 15: `map` 5, `resolve` 4, `import` 2 (folder unchanged, failures listed), `source` 4 (also: save never writes the folder, export only when asked) |
| Tests | 72: 46 `fs-text` as before + 2 `fs-text` serde + 24 `fs-wiki-core` (`texts_crud` 6, `texts_list` 2, `links` 3, `move_text` 5, `outside_change` 6, `provider` 2) |
| capnp-rpc server, GUI, editors, collections / shelves (ADR 0034), named passages' registry, templates in the program | **not started** |

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
| `source/resolve.rs` | Strategy (pure function) | relative links against the page they are on, `#fragment` and `?query` dropped, folders to `index.md` |
| `source/problems.rs` | Adapter | failed imports as a page of the neutral model |
| `args.rs` | Builder-like parser | no extra crate for seven options |

## Decisions

* **Relative links are resolved in the app, not in `fs-wiki-core`.** `fs-wiki-core` keeps its literal path rule (wiki links `[[a/b]]`); documentation links are `../x/y.md#z`. Moving it into the core would change what a link means in the contract; whether it should is a question for the owner.
* **The app is read, edit, export:** save goes to the database (`Texts::edit`); a file is only written by the explicit Export action (ADR 0042, 0046). A session that only reads and edits leaves the docs folder unchanged (tests `import`, `source`).
* **The database is the truth** (ADR 0042): `WikiRepo` never writes a file. `export_text` returns the text; the caller writes it.
  A file that comes back is an `import_text`, which is a diff, not a replacement.
* **A text's path is part of the record**, and indexed. A move changes the path of one text and the *links* in others, nothing else.
  Paths are literal: a link matches a text only when its page part equals the path (`[[manual/start]]`). Relative links such as
  `../start.md` are not resolved (the manifesto example shows the `interfaces/index.md` style).
* **Derived data is only in the index** (`path`, `link`, `block`): the record can always rebuild it.
* `fs-text` stays free of storage; the only change there is a test.

## Crates

* `fs-store` (git tag v0.2.0), `fs-interfaces` (git tag v0.4.0), `fs-text` (same workspace, path).
* `serde`, `serde_json`, `thiserror`, `async-trait`, `tokio` (sync only).

## Open points

First visible part (provisional, for the owner): the name of the reader and of the `fs-components-*` crates; `index.md` as the start page; the sidebar shows page titles, folders their name; import problems as a first tree entry; search results capped at 200; the Export folder is explicit (`--export`), never implied.

Measured on `fs-documentation/docs/en` (1389 pages, 510 folders, 6.3 MB), release build, 32 cores: import into an in-memory store **27 s** (the whole startup; debug build 330 s), peak memory **440 MB**, 0 failed. A second start with `--store file` still takes 31 s: importing an existing path is a diff per page. The tree is built whole (1900 nodes); the picture and the window then take well under a second. Reading and search results of `find` over all pages were not timed separately.

Questions for the owner (all provisional, easy to reverse):

* **Stored media type** `application/vnd.freesynergy.text+json` and the record shape `{ path, document }`: ADR 0034 asks for a
  neutral name for the block specification; this is a placeholder until it exists.
* **`LinkEntry.at`** is `<block id>:<start>-<end>` (char offsets). The schema only says "where in the source".
* **Envelope type:** every text is an `Article`; the schema also names `Note`. When is a text a `Note` (short, no heading)?
* **Move of a folder:** `move_to` moves one text. Moving `manual` does not move `manual/...`; a folder move would be a loop over `list`.
* **Original of an imported file** (ADR 0030) is not kept: a file is an export, and later imports are diffs.

Not done / limits:

* **Uniqueness of `path`** holds inside one process (write lock). Two programs writing the same store at once can still race; the
  store has no unique index (a wish for fs-store).
* **A move is several writes** (other texts, then the moved text); `fs-store` 0.2 has no transaction over several objects. Repeating
  an interrupted move finishes it. Each write is one undo step.
* **Deleting a text** leaves links to it in other texts (dangling, like in any wiki); they are shown as links to a path with no text.
* `LinkValue.target` (UUID of the target) is not filled; links resolve by path.
* `find` is substring only (no tokens, no ranking) and also searches code blocks.
* `subscribe` (`text.subscribe`) is not built; the capnp-rpc server is not written.
* Offset paging (`Paging`) costs O(offset) in the store; the wire `Cursor` will wrap the offset.

## Known issue: Markdown round trip

A nested emphasis that ends right before a literal star (`*a *b***`) comes back with other nesting, because of CommonMark's
rule of three. No text is lost and only runs of three delimiters (`***`, `___`) are affected. `known_issue_rule_of_three` holds the case, and the
property test leaves out `***` and `___` until the export is reworked.

Two lists with nothing written between them become one on re-import, and an empty quote (`>`) has no block at all. No text is
lost. `known_issue_adjacent_lists` holds the case, and the property test leaves out parts that are only an empty quote.
