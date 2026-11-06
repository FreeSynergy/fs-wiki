# fs-text — design

Text core of the FreeSynergy wiki: **text + layers** (ADR 0042). Pure Rust library, no GUI,
no dependency on `fs-store` (all types are `serde`-serialisable so the store can persist them
later). ADRs: 0033 (one marking mechanism), 0034 (passage / page), 0039 (three editors),
0042 (database is the truth, files are exports), 0027 (UUIDv7), 0013 (no copying).

## Model

* `Document { id, blocks }`, `Block { id, text, marks, attrs }`. IDs are UUIDv7.
* A Markdown / code file is a document with **one** block of kind `Raw` (`Document::from_file_text`).
  A block document (imported from Markdown) has one block per Markdown leaf (paragraph, heading,
  code, table, html, rule, front matter). Nesting (quote, callout, list item, footnote) is
  flattened into `attrs.containers`; `inst` numbers tell instances apart.
* `Text` is a `ropey` rope addressed by **char offsets** (not bytes). Heading level, code info,
  table alignment, block ref `^id` are block attributes, not marks.
* `Anchor { offset, bias }` exists with insert/delete transforms; `MarkSet` currently moves plain
  offsets with the same rules (the API is the seam for CRDT ids later).

## Marks (ADR 0033)

`Mark { id, kind, value, start, end, expand }`; `start == end` is a **point** (insert/variable),
otherwise a **range** (format / name / replace). Registry (`REGISTRY`, `MarkKind::spec()`):

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

## Markdown (`md_import`, `md_export`)

`pulldown-cmark` 0.13: CommonMark, tables, task lists, strikethrough, footnotes, heading
attributes, YAML front matter, wiki links. Own handling: callouts `> [!type]± title`, mkdocs
`!!! type "title"` / `???` admonitions (pre-converted to callouts), block refs, empty list items.
Export is canonical-ish and records source style hints (`glue` = no blank line before block,
table delimiter widths, `_` vs `*`, bullet marker) so most files round-trip byte-identical.
Layer marks are **not** written to files (ADR 0042 §2); `ExportOptions::layers` writes
`{{var}}` and `[text]{#name}` for debugging. `ExportOptions::admonitions` writes the mkdocs dialect.

## External edits (`external`) — ADR 0042 §5

`plan_external_import(doc, new_text) -> Vec<Op>` / `import_external`. Raw documents: line diff,
then char diff in changed hunks, applied back to front as `Op::Edit`, so layer marks move with
the text and removed text takes its marks (undoable). Block documents: new text is parsed,
blocks are aligned by their Markdown (Myers), equal blocks keep id and layers, changed blocks get
text edits plus formatting marks from the file (layer marks kept), new/removed blocks become
`InsertBlock`/`RemoveBlock`.

## Links (`links`) — ADR 0042 §8

`extract_links(doc) -> Vec<LinkRow>` (source doc/block/mark, range, path, target uuid, shown
title, internal). `rewrite_links(docs, old, new)` moves pages and folders (`old` and `old/…`, keeps
`#fragment`), updates the mark value and — when the shown text *is* the readable path — the text.
Returns forward/inverse ops per document. `backlinks(rows, target)` reads the same table.

## Templates (`template`) — ADR 0042 §9

`{{name}}`, `{{date:%Y-%m-%d}}` (strftime subset, own UTC calendar code), `\{{` escape.
`instantiate_str` for text, `instantiate` for documents: placeholders are replaced keeping marks
(`Block::splice`), variable **point** marks are filled and become named ranges over the value.

## Undo (`ops`) — ADR 0042 §10

Every `Op` (`Edit`, `SetMarks`, `AddMark`, `RemoveMark`, `SetAttrs`, `InsertBlock`, `RemoveBlock`)
applies to a `Document` and returns its exact inverse (edits carry a snapshot of the block's
marks). `History { retention: { max_age, max_steps }, … }` groups ops into steps, undo/redo,
`compact` drops older steps (current state stays); forward ops of dropped steps not yet synced
stay in `pending_sync` until `mark_synced`. `reset` = "reset undo history". All serde.

## State

Works and is tested (21 unit/property tests, corpus round trip):
* model, rope text, anchors, mark registry/edit rules, point/range, merge, unmark;
* property tests (`proptest`): marks stay valid under random edits, undo restores text and marks
  exactly, outside edits do not change a mark's text, diff import reproduces the target text,
  Markdown model is stable (md → model → md → model);
* Markdown import/export incl. footnotes, front matter, wiki links, block refs, callouts, mkdocs
  admonitions; the whole `fs-documentation/docs/en` tree (1321 files) re-imports stably after
  export, 1256 byte-identical (with the mkdocs dialect for admonitions), the rest normalised;
* external edit import (raw and block), link table + rewrite, templates, history with retention.

Known limits / not done:
* Normalisations: indented code -> fenced, setext -> ATX, reference links -> inline, trailing
  double-space hard breaks -> `\`, `*`/`+` bullets kept per item but mixed styles are not
  re-aligned, table cells beyond the header width are dropped (GFM), `$math$`, definition lists,
  sub/superscript are not enabled.
* Emphasis whitespace-flanking is not repaired on export (a mark ending in a space may not parse
  back); layer syntax (`[x]{#n}`, `{{v}}`) is export-only.
* Markdown export of a table keeps one block per table (cells joined by TAB/LF in the text).
* Offsets are chars; no grapheme-boundary snapping. No CRDT / concurrent editing.
* Edit inverses snapshot the block's marks (O(marks) per op), fine for passage-sized blocks.
* Named passages' registry (find place, reuse, transclusion) and the "remind on save" UI hint are
  store/UI work; this crate only provides marks, ops and `History::op_count` as size indicator.
* Only the `Raw` one-block document is covered by the diff import at char level; block documents
  are aligned per block (no move detection).
