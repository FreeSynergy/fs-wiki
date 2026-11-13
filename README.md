# FreeSynergy Wiki — write and know together

**Every text you write is a page you can find, link and keep, and the database is the truth, not a pile of files.**

FreeSynergy Wiki turns texts into connected pages. Pages link to each other, and you can see what links to a page. Move a page or a whole folder and the links that pointed at it follow. Search finds words in titles, paths and content.

What is different here: a text is stored as text plus layers, not as one flat file. You can import a folder of Markdown files, edit in the program, and export Markdown again when you decide to. Saving is not exporting: nothing is written to your files until you press Export. If both the file and the database changed, the program shows you the conflict instead of overwriting either. This program owns the open text interface, so other programs can reach texts through one interface instead of keeping their own text model.

## What it can do today

Latest tag: `v0.0.9`; `v0.0.10` is in preparation (the normal start can make, paste into, save and edit pages).

- **Frontends.** A plain `fs-wiki` (what the desktop file runs) opens the wiki window over the wiki of the server, in the remembered frontend (first start: Confluence). **Pages can be made and edited there** (since 0.0.10): the menu Page (German: Seite) has New page (Neue Seite) and New subpage (Neue Unterseite); the page is made at once (a subpage hangs under its parent in the page tree), opens in the editor, and you write its title and text (typing and pasting with Ctrl+V), then save with the Save button or Ctrl+S. `--read-only` makes the window read only (the menu then says why it is locked); a documentation read from folders (`--read` with sources) stays read only unless `--edit`. The old Admin window is `fs-wiki --admin`. `--read` is the same window and additionally imports `--docs-root <checkout>` or `--source` folders first (kept for compatibility). The window has a menu View > Frontend (German: Ansicht > Frontend) that switches between Classic (the plain reader), Confluence (page tree left, head with language switch, outline right) and Notion (calm sidebar, big title, blocks). The choice is remembered (a small file `freesynergy/fs-wiki/frontend` in the data folder, taken over from the `face` file of 0.0.8; a missing or broken file means the default). A new installation shows Confluence. `--frontend a|b|classic` overrides it for one run without remembering; `--edit` makes the pages of imported sources editable; `--index FILE` reads the heads of all texts from a file (listing the whole documentation takes minutes).
- **Limits of the frontends.** The history shows only the current version; the page id (UUIDv7) is not shown; living components are not drawn (their Markdown fallback is); the editor has no `/` menu yet; the page icon above the title of Notion is still a grey page symbol (it should be an emoji of the page or empty space: that is a change of fs-components); `Texts.listHeads` (fast listing) comes with fs-interfaces v0.0.20.

- Create, edit, move, delete and list texts by path; links, backlinks and search.
- Move a text or a folder and rewrite the links to it.
- Import Markdown (a changed file becomes ordinary edits) and export Markdown.
- Refuse an overwrite when someone else saved first.
- Read the documentation: `fs-wiki --read --docs-root <checkout of fs-documentation>` imports its four folders into one wiki (prefixes `en/`, `de/`, `adr/`, `research/`), orders the pages by the documentation's navigation and shows them read only, with a sidebar and search. The import is byte for byte (4199 files measured).
- Tell other programs when a text changes.
- A small server that offers all of this over a local socket.
- A first window, the reader: a tree of pages, reading with formatting, following links, back, search, editing and saving, and an explicit Export button. Unsaved drafts are backed up and offered back after a crash.
- The first start imports a documentation folder (about 30 seconds for 1,400 pages).

**Deutsch (0.0.10).** Der normale Start `fs-wiki` öffnet das Wiki-Fenster jetzt bearbeitbar: Menü Seite > Neue Seite / Neue Unterseite legt die Seite sofort an (eine Unterseite hängt im Seitenbaum unter ihrer Elternseite) und öffnet sie im Editor; Titel und Text tippen oder mit Strg+V einfügen, mit Speichern oder Strg+S sichern, später wieder öffnen und bearbeiten. `--read-only` macht das Fenster nur lesend (das Menü sagt dann, warum die Einträge gesperrt sind); eine aus Ordnern gelesene Dokumentation (`--read` mit Quellen) bleibt nur lesend, außer mit `--edit`.

## What is not there yet

- Typing in the real window has not been verified by tests; the DESIGN file lists what to try by hand.
- No other editors, no collections or shelves, no templates inside the program.
- The edit lock works inside one process. Two programs on the same store file are not covered.
- Some characters (certain emoji) show as boxes, and very wide tables can run past the edge.
- The interface is a draft with no consumer yet.

## Part of FreeSynergy

Wiki is where texts live for the groupware, project-management and democracy suites. FreeSynergy is a family of programs that are free for everyone, forever, and that keep your data yours. The documentation, the manifesto and the overall plan live in [fs-documentation](https://github.com/FreeSynergy/fs-documentation); the reading version is at <https://freesynergy.github.io/fs-documentation/>. Start with the [manifesto](https://freesynergy.github.io/fs-documentation/manifesto/) if you want to know why we do this.


## For developers

- Workspace crates: `fs-text` (library for text and layers, no storage), `fs-wiki-core` (the `Texts` trait and its store-backed implementation), `fs-wiki-server` (contract over a Unix socket), `fs-wiki-app` (the reader window, on [fs-components](https://github.com/FreeSynergy/fs-components)).
- The contract is `texts.capnp` in [fs-interfaces](https://github.com/FreeSynergy/fs-interfaces) (draft). Wiki owns it.
- Build and test: `cargo build --workspace` and `cargo test --workspace`. Try the core: `cargo run -p fs-wiki-core --example manifesto`.
- Design and the full state table: [DESIGN.md](DESIGN.md).
- Licence: MIT OR Apache-2.0.
