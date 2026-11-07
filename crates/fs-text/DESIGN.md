# fs-text — design

Text core of the FreeSynergy wiki: **text + layers** (ADR 0042). Pure Rust library, no GUI,
no dependency on `fs-objects` (all types are `serde`-serialisable; `fs-wiki-core` in the same workspace stores them). ADRs: 0033 (one marking mechanism), 0034 (passage / page), 0039 (three editors),
0042 (database is the truth, files are exports), 0027 (UUIDv7), 0013 (no copying).

## Layout (ADR 0043: one concept per file, <= 200 lines, `mod.rs` only declares)

```
src/
  id.rs  anchor.rs
  text/      mod, storage (trait), rope (ropey), buffer (Text facade), grapheme
  mark/      mod, behavior (trait), kind, registry, item (Mark), set, merge, value
  doc/       mod, document, block, attrs, block_kind, container, same
  ops/       mod, op (enum), command (trait), apply, edit, marks, blocks, user, error
  history/   mod, log, step, retention, compact
  format/    mod, importer (trait), exporter (trait)
  markdown/  mod, adapter, syntax,
             import/{run, events, state, leaf, block, container, table, frontmatter,
                     wikilink, inline, callout, admonition, blockref, hints, loose, options}
             export/{document, container, layout, callout, leaf, code, table, options,
                     inline/{emit, plan, writer, delim, atomic, escape, ctx}}
  external/  mod, diff, plan, blocks
  links/     mod, row, rewriter (trait), rewrite
  template/  mod, datetime, format, values, placeholder, instantiate, error
tests/       one file per feature (marks, history, external, links, templates, markdown,
             model, grapheme, props_*, roundtrip)
```

## Pattern map

| Module | Pattern | Why |
|---|---|---|
| `text::storage` / `rope` / `buffer` | Strategy + Adapter + Facade | the rope is one `TextStorage`; `Text` is the small API; a CRDT can replace the rope |
| `mark::registry` / `behavior` / `kind` | Registry + Strategy | kind -> `MarkBehavior` (expand, overlap, point, layer); new kinds add an entry, no `match` spread |
| `mark::item` / `merge` / `set` | Strategy client | edit and merge rules ask the behaviour, never the kind |
| `markdown::syntax` + `export::inline::writer` | Registry + Strategy | kind -> `MdSyntax`, which implements `MarkWriter`; the mark core stays format-free |
| `format` + `markdown::adapter` | Adapter | `Importer` / `Exporter` traits; Markdown is the first adapter, others plug in |
| `markdown::import::*` | Builder (event driven) | parser events build blocks; one handler file per concern |
| `markdown::import::admonition` | Adapter (dialect) | mkdocs admonitions rewritten to callouts before parsing |
| `markdown::export::*` | Template method + Strategy | document -> layout -> leaf -> inline; callout dialect chosen by option |
| `ops` | Command | `Op` + `Command::apply` -> inverse; handlers per op kind |
| `history` | Command history (Caretaker) | steps with inverses, undo/redo cursor, retention (Strategy data) |
| `external` | Command producer + Adapter over `similar` | diff -> ops, so marks move with text |
| `links::rewriter` | Strategy | `LinkRewriter` trait, `MovePrefix` = page/folder move |
| `links::rewrite` / `row` | Command producer / read model | undoable rewrite, link table |
| `template` | Template method + Value object | find, resolve, splice keeping marks |
| `doc` | Composite (flattened) | document -> blocks -> containers list |
| `text::grapheme` + `ops::user` | Factory (Command) + Adapter over `unicode-segmentation` | user edits and marks snap to grapheme clusters; replay / import stay exact |
| `id`, `anchor` | Factory / Value object | one id source; offset transforms in one type |

## Model

* `Document { id, blocks }`, `Block { id, text, marks, attrs }`. IDs are UUIDv7.
* A Markdown / code file is a document with **one** block of kind `Raw` (`Document::from_file_text`).
  A block document (imported from Markdown) has one block per Markdown leaf (paragraph, heading,
  code, table, html, rule, front matter). Nesting (quote, callout, list item, footnote) is
  flattened into `attrs.containers`; `inst` numbers tell instances apart.
* `Text` is an editable string addressed by **char offsets** (not bytes); the storage sits behind the `TextStorage` trait (default: a `ropey` rope, `RopeStorage`), so a CRDT sequence such as `loro` can replace it by changing `DefaultStorage`. Heading level, code info,
  table alignment, block ref `^id` are block attributes, not marks.
* `Anchor { offset, bias }` exists with insert/delete transforms; `MarkSet` currently moves plain
  offsets with the same rules (the API is the seam for CRDT ids later).

## Marks (ADR 0033)

`Mark { id, kind, value, start, end, expand }`; `start == end` is a **point** (insert/variable),
otherwise a **range** (format / name / replace). Registry (`REGISTRY`, `Registry::get(kind)` / `MarkKind::behavior()` return a `MarkBehavior`; how a kind is written is `markdown::md_syntax(kind)` + `MarkWriter`):

| kind | expand | overlap | point | Markdown |
|---|---|---|---|---|
| strong, emphasis, strike, code | after | merge | no | `**` `*` `~~` `` ` `` |
| link, image | none | multiple | no | `[t](d "title")`, `[[path\|t]]`, `<url>`, `![a](s)`, `![[x]]` |
| html | none | multiple | no | verbatim |
| passage (named), explanation (replace/mute) | none | multiple | no | layer only |
| variable | none | multiple | yes | layer only |
| footnote-ref | none | multiple | yes | `[^label]` |

Edit rules: insertion at a range start joins iff `expand.before`, at the end iff `expand.after`;
a point stays put iff `expand.before`. A deletion that covers a mark's whole text removes the
mark (ADR 0042 §6); the op's inverse restores it. Merge kinds are unioned on `MarkSet::add`.

## Markdown (`markdown::import`, `markdown::export`)

`pulldown-cmark` 0.13: CommonMark, tables, task lists, strikethrough, footnotes, heading
attributes, YAML front matter, wiki links. Own handling: callouts `> [!type]± title`, mkdocs
`!!! type "title"` / `???` admonitions (pre-converted to callouts), block refs, empty list items.
Export is canonical-ish and records source style hints (`glue` = no blank line before block,
table delimiter widths, `_` vs `*`, bullet marker) so most files round-trip byte-identical.
Layer marks are **not** written to files (ADR 0042 §2); `ExportOptions::layers` writes
`{{var}}` and `[text]{#name}` for debugging. `ExportOptions::admonitions` writes the mkdocs dialect.

## Grapheme-safe offsets

Offsets stay chars, but marks and user edits snap to grapheme clusters (UAX #29,
`unicode-segmentation`): emoji with skin tone, ZWJ sequences, `e` + combining accent, flags,
`\r\n` are one unit. `Block::add_mark` and `Op::AddMark` grow a range to whole clusters (points
go to the nearest boundary). `Document::edit_op` / `insert_op` / `delete_op` build an `Op::Edit`
on boundaries (insertions move to the nearest boundary, ranges grow). A plain `Op::Edit` is exact
on purpose: undo inverses, link rewrites and external-file import must reproduce text
byte for byte. `text::grapheme` offers `boundaries`, `snap`, `snap_range`, `is_boundary`.

## External edits (`external`) — ADR 0042 §5

`plan_external_import(doc, new_text) -> Vec<Op>` / `import_external` (`plan_external_import_with` takes any `Importer` + `Exporter`). Raw documents: line diff,
then char diff in changed hunks, applied back to front as `Op::Edit`, so layer marks move with
the text and removed text takes its marks (undoable). Block documents: new text is parsed,
blocks are aligned by their Markdown (Myers), equal blocks keep id and layers, changed blocks get
text edits plus formatting marks from the file (layer marks kept), new/removed blocks become
`InsertBlock`/`RemoveBlock`.

## Links (`links`) — ADR 0042 §8

`extract_links(doc) -> Vec<LinkRow>` (source doc/block/mark, range, path, target uuid, shown
title, internal). `rewrite_links(docs, old, new)` (= `rewrite_links_with(docs, &MovePrefix)`; any `LinkRewriter` works) moves pages and folders (`old` and `old/…`, keeps
`#fragment`), updates the mark value and — when the shown text *is* the readable path — the text.
Returns forward/inverse ops per document. `backlinks(rows, target)` reads the same table.

## Templates (`template`) — ADR 0042 §9

`{{name}}`, `{{date:%Y-%m-%d}}` (strftime subset, own UTC calendar code), `\{{` escape.
`instantiate_str` for text, `instantiate` for documents: placeholders are replaced keeping marks
(`Block::splice`), variable **point** marks are filled and become named ranges over the value.

## Undo (`ops`) — ADR 0042 §10

Every `Op` is a `Command` (`apply` returns the exact inverse); the variants (`Edit`, `SetMarks`, `AddMark`, `RemoveMark`, `SetAttrs`, `InsertBlock`, `RemoveBlock`)
applies to a `Document` and returns its exact inverse (edits carry a snapshot of the block's
marks). `History { retention: { max_age, max_steps }, … }` groups ops into steps, undo/redo,
`compact` drops older steps (current state stays); forward ops of dropped steps not yet synced
stay in `pending_sync` until `mark_synced`. `reset` = "reset undo history". All serde.

## State

Works and is tested (48 tests, the last two (`tests/serde_form.rs`) prove the JSON form of a document that `fs-wiki-core` stores: every mark kind, points, attributes and containers round-trip, also through `serde_json::Value`; before: 46 tests: unit tests next to the code, integration tests per feature,
property tests, corpus round trip):
* model, text behind `TextStorage`, anchors, mark registry/behaviour/edit rules, point/range, merge, unmark;
* property tests (`proptest`): marks stay valid under random edits, undo restores text and marks
  exactly, outside edits do not change a mark's text, diff import reproduces the target text,
  Markdown model is stable (md -> model -> md -> model);
* Markdown import/export (behind `Importer`/`Exporter`) incl. footnotes, front matter, wiki links,
  block refs, callouts, mkdocs admonitions; the whole `fs-documentation/docs/en` tree (1321 files)
  re-imports stably after export, 1256 byte-identical (with the mkdocs dialect), the rest normalised;
* external edit import (raw and block), link table + rewrite, templates, history with retention.
* ADR 0043 refactor done: no file over 200 lines, every module names its pattern,
  `#![deny(missing_docs)]`, `check_rust_rules.py` reports 0 errors / 0 warnings.

Known limits / not done:
* Normalisations: indented code -> fenced, setext -> ATX, reference links -> inline, trailing
  double-space hard breaks -> `\`, `*`/`+` bullets kept per item but mixed styles are not
  re-aligned, table cells beyond the header width are dropped (GFM), `$math$`, definition lists,
  sub/superscript are not enabled.
* Emphasis next to another emphasis with different delimiters (`_-_*a*`) can still export to something that re-parses differently; `export::inline::emit::apart_from_star` fixes only the simple adjacent-star case.
* Emphasis whitespace-flanking is not repaired on export (a mark ending in a space may not parse
  back); layer syntax (`[x]{#n}`, `{{v}}`) is export-only.
* Markdown export of a table keeps one block per table (cells joined by TAB/LF in the text).
* Offsets are chars. No CRDT / concurrent editing (the `TextStorage` trait is the seam).
* Edit inverses snapshot the block's marks (O(marks) per op), fine for passage-sized blocks.
* Named passages' registry (find place, reuse, transclusion) and the "remind on save" UI hint are
  store/UI work; this crate only provides marks, ops and `History::op_count` as size indicator.
* Only the `Raw` one-block document is covered by the diff import at char level; block documents
  are aligned per block (no move detection).
