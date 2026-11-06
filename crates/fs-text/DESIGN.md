# fs-text — design

Text core of the FreeSynergy wiki: **text + layers** (ADR 0042). Pure Rust library, no GUI,
no dependency on `fs-store` (all types are `serde`-serialisable; `fs-wiki-core` in the same workspace stores them). ADRs: 0033 (one marking mechanism), 0034 (passage / page), 0039 (three editors),
0042 (database is the truth, files are exports), 0027 (UUIDv7), 0013 (no copying).

## Layout (ADR 0043: one concept per file, <= 200 lines, `mod.rs` only declares)

```
src/
  id.rs  anchor.rs
  text/      mod, storage (trait), rope (ropey), buffer (Text facade), grapheme
  mark/      mod, behavior (trait), kind, registry, item (Mark), set, merge, value
  doc/       mod, document, block, attrs, block_kind, container, same, source
  ops/       mod, op (enum), command (trait), apply, edit, marks, blocks, user, error
  history/   mod, log, step, retention, compact
  format/    mod, importer (trait), exporter (trait)
  markdown/  mod, adapter, syntax,
             import/{run, events, state, leaf, block, container, table, frontmatter,
                     wikilink, inline, callout, admonition, blockref, hints, loose, options,
                     refdef, attach, separator}
             export/{document, container, layout, callout, leaf, code, table, options,
                     inline/{emit, plan, writer, delim, atomic, escape, ctx, variant, verify}}
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
| `doc::source` + `import::attach` + `export::document` | Memento | each block keeps the exact text it was read from; an unchanged block is written back from it |
| `export::inline::verify` | Generate and test | a whole block is read back; the first form that reads right wins, else the form that escapes everything |
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
### Verbatim source (PROVISIONAL, the owner decides the final rule)

On import every block keeps the exact source text it came from (`BlockAttrs::source`, a
`Source`): its own lines plus the blank lines before it, with the file's line ending, BOM and
missing final newline; the last block takes everything up to the end of the file. Blocks that
share a source line (`- > a`) form one group that owns the lines together (`Source::rest`).
The pieces, written in order, are the file byte for byte (`tests/props_bytes.rs` proves it
for arbitrary text, with CRLF and BOM, and through JSON).

On export a block is written from its source **only if** its seal matches (128-bit FNV-1a over
the block's fingerprint, the fingerprints of the blocks of its group, and the source text itself,
so a changed text, also in JSON, is not used), the fingerprint is unchanged (text, Markdown
marks with kind/range/value but not ids and not layer marks, kind, containers, the other
attributes) and the block in front is the same block. A layer mark (passage, explanation,
variable) never invalidates a source: layers are not written to files.

**The output proves itself, and says what it could not do.** When anything was edited (some
block is written anew, or the last kept source does not reach the end of the file), the output is
read back and laid against the model (`export::report::analyse`): blocks written from their kept
source are anchors and must read back as they are; between anchors the blocks written anew may
read back as anything.

* If the anchors hold, the pass is final, even when a block written anew does not read back
  exactly (an edge blank or newline in a paragraph, markup inside a wiki link label, `&#10;` in
  a link, text that an open HTML block swallows). That block is written in its nearest form and
  named in the `ExportReport { inexact: [block, why], rewritten }` returned by
  `export_markdown_report`; `export_markdown` is the same without the report. Untouched
  neighbours stay byte for byte: a block that cannot be said exactly does not spread.
* Only if an anchor would read differently (a removed definition turns a reference link
  literal, a block that lost its container leaves the indented block after it in the list, a
  heading that became a paragraph merges with the glued line below) are the changed blocks'
  surroundings written anew, ever wider (radius 1, 3, 9, …), and the whole document last. Those
  neighbours are listed in `rewritten`. If even the whole document has inexact blocks, the
  least invasive pass is kept and reports them.

An untouched document skips all this at no cost. A one-word edit keeps changing one line.

A changed block (first pass) keeps what stood in front of it in the file (blank lines) while its predecessor is the same, and the blank lines after a changed
last block; it uses the file's line ending, the table dash widths, `_` vs `*`, bullet marker,
rule style, the case of a callout type (`[!NOTE]`) and the mkdocs `!!!` dialect (hint
`admonition`). The source is part of the stored form: serialised with the block (`attrs.source`,
skipped when absent), travels with `Op::SetAttrs`, and `Document::same_content` ignores it
(and nothing else: hints and `_` vs `*` count; tests that mean "the same up to layout style"
strip them explicitly). Costs: the kept source about doubles the text; the JSON of a document is
about 6.5 times the text (measured by the review; ids of blocks and marks). An untouched export is linear and does no
read-back.

A file that has no block (empty, blank lines only) is one empty paragraph that owns the text.
Link reference definitions (`[x]: /u "t"`, `[//]: # (comment)`) are kept as HTML-kind blocks
with the hint `unparsed`, also a second definition of the same label (`import::stray`; the parser
skips it), behind `> ` (it joins the quote of its neighbour) and with the destination on the next line. No text lives only in a kept source, and the export checks it: when a block is written anew, text
that stood next to it in its source (in front of it, after the last block) and is not in the output
is named in `ExportReport.inexact`. Besides that, the gaps hold only blank lines and the comment that keeps two
lists apart (checked over the docs and 2286 third-party files). Text a person typed is never
dropped; reference links themselves are still written inline.

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

Works and is tested (see `cargo test -p fs-text`: unit tests next to the code, integration tests per feature,
property tests (`tests/props_bytes.rs`: tab, `~`, `|`, `\`, `!`, `:`, braces, entities, CRLF, BOM; run with
`PROPTEST_CASES=200000` for a long soak), byte-level round trips (`tests/verbatim*.rs`), corpus round trip,
JSON form in `tests/serde_form.rs`):
* model, text behind `TextStorage`, anchors, mark registry/behaviour/edit rules, point/range, merge, unmark;
* property tests (`proptest`): marks stay valid under random edits, undo restores text and marks
  exactly, outside edits do not change a mark's text, diff import reproduces the target text,
  Markdown model is stable (md -> model -> md -> model);
* Markdown import/export (behind `Importer`/`Exporter`) incl. footnotes, front matter, wiki links,
  block refs, callouts, mkdocs admonitions; the whole `fs-documentation/docs/en` tree (1398 files)
  comes back byte-identical untouched and re-imports stably through the canonical writer alone
  (1328 of 1398 byte-identical even then);
* external edit import (raw and block), link table + rewrite, templates, history with retention.
* ADR 0043 refactor done: no file over 200 lines, every module names its pattern,
  `#![deny(missing_docs)]`, `check_rust_rules.py` reports 0 errors / 0 warnings.

Known limits / not done:
* The canonical writer reads every inline block back (`export::inline::verify`). If the text or
  kind would change it tries other delimiter forms and finally escapes all ASCII punctuation;
  marks that cannot be written the same way (crossing marks, emphasis inside emphasis of the same
  kind, flanking next to a no-break space) make a form second best, not wrong.
* The title of a callout is kept as written (Markdown, `Callout::title`: `See [x](a.md)`), so its
  links are rewritten by `rewrite_links` like those in text; `extract_links` does not list them.
* A fenced code block keeps `~~~` and a space before the info string (hint `fence-char`) when
  it is written anew. Still open: the indent of a multi-line definition grows by the container
  indent (`matrix-sdk-sqlite`), and a rule hint appears where `---` must become `***`
  (`turso_ext`): 2 of 2286 third-party files are not `same_content` again.
* A block reference (`text ^id`) needs a letter in the id and plain text: `E = mc ^2` and
  `` `^1` `` are text.
* Typed U+2028 becomes a no-break space (U+2028 is the hard break in block text); a tab or line
  break written as `&#9;` / `&#10;` in a table cell becomes a space (they separate cells and rows
  in the block text). Both need a model change to be exact.
* Inline HTML over several lines inside a container takes the container's indentation as part of
  its text; three nested empty bullets are written `* * *` (a rule); a second definition with
  the same label is not a block (it stays in the kept source only).
* mkdocs: `!!!` needs the Python-Markdown `admonition` extension; `> [!note]` is GitHub / Obsidian
  syntax and is not an admonition in mkdocs. The docs' `mkdocs.yml` lists neither `admonition`
  nor `pymdownx.blocks`, so a callout is kept in the dialect it was written in.
* Normalisations of a **changed** block only (an untouched block is verbatim): indented code -> fenced, setext -> ATX (a multi-line heading stays setext), reference links -> inline, trailing
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
