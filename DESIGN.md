# fs-wiki - Design

`fs-wiki` is the wiki program of FreeSynergy and the **owner of the text interface** (ADR 0031, 0050): every text is a wiki
entry, a collection of texts is a wiki. The database is the truth, files are exports (ADR 0042). The contract is `texts.capnp`
in [`fs-interfaces`](https://github.com/FreeSynergy/fs-interfaces) (ADR 0045); this program is its provider. This repo holds
two crates and no GUI:

* `crates/fs-text`: the **library** of text + layers (document, marks, Markdown, outside changes, links, templates, history).
  No storage, no interface. Its own design is in [`crates/fs-text/DESIGN.md`](crates/fs-text/DESIGN.md).
* `crates/fs-wiki-core`: the **program core**. The trait `Texts` (Rust form of `Texts` in `texts.capnp`) and `WikiRepo`, its
  implementation over `fs-store`.

## Interface status

**Interface status: draft, consumers: none (0 of 2).** (ADR 0045: the interface is a target; it stays draft until two independent
consumers have tested it, then 1.0, then only version jumps.) Nothing here is declared 1.0.

## State

Version 0.1.0 (first tag). Built on `fs-store` v0.2.0 and `fs-interfaces` v0.4.0 (git tags).

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
| `move_folder(prefix, path)` (`moveFolder` of `texts.capnp` 0.6.0, capability `text.move`): the text at `prefix` and all `prefix/...` move, links in all texts follow (also between the moved ones, with `#fragment`; `manual-old` is not below `manual`) | done, tested on a real `TursoStore` (plan, clash, interruption at every write, undo, 510 texts) |
| `import_text(path, media_type, text)`: new path = create; existing path = outside change: `import_external` diff becomes ordinary ops, layers move with the text, deleted text takes its marks along, unchanged file writes nothing | done, tested |
| `export_text(id, media_type)`: Markdown, layers not written | done |
| Media types (`TextFormat`, Factory): `text/markdown` (parameters and case ignored); anything else is `Error::MediaType` | done |
| Example `cargo run -p fs-wiki-core --example manifesto`: imports `fs-documentation/docs/en/manifesto.md`, reads it back (identical), lists its links | done |
| Tests | `fs-wiki-core`: 32 integration tests (`texts_crud` 6, `texts_list` 2, `links` 3, `move_text` 5, `move_folder` 8, `outside_change` 6, `provider` 2); `fs-text` has its own |
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
| `repo/folder.rs` | Command producer, plan then execute | the plan is made from reads, refused or executed in two phases; `MovePrefix` of fs-text is the strategy |
| `repo/moving.rs` | Command producer + Strategy | `rewrite_links_with` yields the rewrite, `MovePage` is the strategy |
| `repo/links.rs` | Read model | rows from the marks, back references from the index |
| `repo/transfer.rs` | Template method | create or outside-change import; export |
| `repo/book.rs` | Facade | thin trait impl, write lock, capability report |
| `media.rs` | Factory | media type -> `TextFormat` -> `fs-text` importer / exporter |
| `path.rs` | Newtype | `TextPath`: only valid paths exist |
| `error.rs` | none | plain error enum |

## Decisions

* **The database is the truth** (ADR 0042): `WikiRepo` never writes a file. `export_text` returns the text; the caller writes it.
  A file that comes back is an `import_text`, which is a diff, not a replacement.
* **A text's path is part of the record**, and indexed. A move changes the path of one text and the *links* in others, nothing else.
  Paths are literal: a link matches a text only when its page part equals the path (`[[manual/start]]`). Relative links such as
  `../start.md` are not resolved (the manifesto example shows the `interfaces/index.md` style).
* **Derived data is only in the index** (`path`, `link`, `block`): the record can always rebuild it.
* `fs-text` stays free of storage; the only change there is a test.

## Crates

* `fs-store` (git tag v0.2.0), `fs-interfaces` (git tag v0.6.0), `fs-text` (same workspace, path).
* `serde`, `serde_json`, `thiserror`, `async-trait`, `tokio` (sync only).

## Open points

Questions for the owner (all provisional, easy to reverse):

* **Stored media type** `application/vnd.freesynergy.text+json` and the record shape `{ path, document }`: ADR 0034 asks for a
  neutral name for the block specification; this is a placeholder until it exists.
* **`LinkEntry.at`** is `<block id>:<start>-<end>` (char offsets). The schema only says "where in the source".
* **Envelope type:** every text is an `Article`; the schema also names `Note`. When is a text a `Note` (short, no heading)?
* **Original of an imported file** (ADR 0030) is not kept: a file is an export, and later imports are diffs.

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

The property `markdown_model_is_stable` (import, export, import gives the same content) holds
without exclusions. Real text stays clean: `2 * 3`, `snake_case` and `a_b` get no backslashes.

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
