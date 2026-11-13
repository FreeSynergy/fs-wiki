# Changelog

## Unreleased
* Wiki frontends A and B (`--frontend a|b`), start from an index file (`--index`), language switch and state in the page head, text edit of language variants (page path maps to its text).

## 0.0.7 (in progress)

### English

- **Reading wiki (new): `fs-wiki --read --docs-root <checkout of fs-documentation>`.** The four folders of the documentation (`docs/en`, `docs/de`, `conventions/decisions`, `temp/research`) go into ONE wiki and are shown read only, with a sidebar, the page and a search. The import is byte for byte: measured on the real documentation, 4199 of 4199 files come back identical. The order of the pages follows the documentation's navigation (`mkdocs.yml`, the `.pages` files).
- **German and English are one page.** A page has one id and a variant per language; the reader shows a language bar, says so when a page exists in one language only, and marks a translation as outdated when its original changed after it was translated. The origin language is set per family (default English, the decisions German). On the real documentation: 2378 pages, 1821 in two languages.
- **Component blocks (model only).** A page may hold a living component (task board, map, calendar ...) written as a comment pair around an ordinary Markdown fallback; it reads and writes back byte for byte and shows its fallback in every other Markdown viewer. Nothing draws components yet.
- **Typed change of a page.** `Page.change` is a typed edit (`Edit`) in the Admin frontend; the revision the person saw is remembered, so a write in between is a conflict. The summary field is gone.
- **Blinc only through fs-components (Q0 migration).** The keys of other layouts (`@`, `+`, `/`, AltGr) type correctly. The window checks the version of the server and replaces one of another version; every call has a time limit.
- Built on fs-components v0.0.11 and fs-interfaces v0.0.19 (Blinc fs-blinc-3.1).
- For the typing test only: the optional feature `test-access` (debug builds; fs-components refuses it in a release build) lets the test read what is typed into the fields.
- **Limits.** No history and no backlinks in the reading wiki yet. Files that are not texts (images, style sheets) are named in the import report, not stored. The sidebar names a folder by its path, not by the title of its `.pages`. The window does not open without a server (no "Try again" yet; it ends with the message). Further frontends (Wikipedia-like, Obsidian-like) come after the milestone.

### Deutsch

- **Lesewiki (neu): `fs-wiki --read --docs-root <Checkout von fs-documentation>`.** Die vier Ordner der Dokumentation (`docs/en`, `docs/de`, `conventions/decisions`, `temp/research`) kommen in EIN Wiki und werden nur lesend gezeigt, mit Seitenleiste, Seite und Suche. Der Import ist byte-genau: an der echten Dokumentation gemessen kommen 4199 von 4199 Dateien identisch zurück. Die Reihenfolge der Seiten folgt der Navigation der Dokumentation (`mkdocs.yml`, die `.pages`-Dateien).
- **Deutsch und Englisch sind eine Seite.** Eine Seite hat eine Id und je Sprache eine Variante; der Leser zeigt eine Sprachleiste, sagt es, wenn es eine Seite nur in einer Sprache gibt, und kennzeichnet eine Übersetzung als veraltet, wenn das Original nach der Übersetzung geändert wurde. Die Ursprungssprache gilt je Familie (Standard Englisch, die Entscheidungen Deutsch). In der echten Dokumentation: 2378 Seiten, davon 1821 in zwei Sprachen.
- **Component-Blöcke (nur das Modell).** Eine Seite kann eine lebendige Component enthalten (Aufgabenbrett, Karte, Kalender ...), geschrieben als Kommentarpaar um einen gewöhnlichen Markdown-Rückfall; sie wird byte-genau gelesen und zurückgeschrieben und zeigt in jedem anderen Markdown-Betrachter ihren Rückfall. Gezeichnet werden Components noch nicht.
- **Typisierte Änderung einer Seite.** `Page.change` ist im Admin-Frontend eine typisierte Änderung (`Edit`); die Revision, die die Person sah, wird gemerkt, ein Schreiben dazwischen ist also ein Konflikt. Das Zusammenfassungsfeld entfällt.
- **Blinc nur über fs-components (Q0-Migration).** Die Tasten anderer Layouts (`@`, `+`, `/`, AltGr) tippen richtig. Das Fenster prüft die Version des Servers und ersetzt einen anderer Version; jeder Aufruf hat ein Zeitlimit.
- Gebaut auf fs-components v0.0.11 und fs-interfaces v0.0.19 (Blinc fs-blinc-3.1).
- Nur für den Tipptest: das optionale Feature `test-access` (Debug-Builds; fs-components verweigert es in einem Release-Build) lässt den Test lesen, was in die Felder getippt wird.
- **Grenzen.** Im Lesewiki noch kein Verlauf und keine Rückverweise. Dateien, die keine Texte sind (Bilder, Stylesheets), werden im Importbericht genannt, nicht gespeichert. Die Seitenleiste nennt einen Ordner nach seinem Pfad, nicht nach dem Titel seiner `.pages`. Das Fenster öffnet ohne Server nicht (noch kein "Try again"; es endet mit der Meldung). Weitere Frontends (Wikipedia-artig, Obsidian-artig) kommen nach dem Meilenstein.
