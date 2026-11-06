# MediaWiki - Functions

## Overview

MediaWiki offers two main editors: the classic **source-code editor** with wiki
syntax and the **Visual Editor (VisualEditor)**. Availability depends on the
server configuration.

## Available editors

### 1. Visual Editor (VisualEditor)

#### Functions:
- **WYSIWYG**: what you see is what you edit
- **Formatting**: headings (H1-H6), bold, italic, underline
- **Lists**: numbered lists, bullet lists
- **Tables**: create/edit tables (columns/rows)
- **Media**: insert images and files
- **Links**: add external and internal links
- **Templates**: insert templates (visually)
- **Sources**: citations and references
- **Code blocks**: syntax highlighting (optional)
- **Math formulas**: LaTeX support (optional)
- **Diagrams**: with extensions (e.g. Graphviz)

#### What the editor does NOT do:
- no direct wiki-syntax editing in visual mode
- complex template parameters are not directly visible
- no direct editing of wikitext structures
- limited support for advanced functions

---

### 2. Source-code editor (WikiText)

#### Functions:
- **Complete wiki syntax**: the MediaWiki syntax (wikitext)
- **Complete control**: control every character
- **Templates**: `{{Template}}` syntax
- **Links**: `[[PageName]]` internal links
- **Images**: `[[File:Image.jpg|Thumb|Description]]`
- **Categories**: `[[Category:Name]]`
- **Formatting**: `'''bold'''`, `''italic''`, `===Heading===`
- **Lists**: `* bullet`, `# numbered`
- **Tables**: `{| ... |}` syntax
- **HTML**: limited HTML support

#### Wiki syntax examples:
```wikitext
== Heading ==
'''Bold text''' and ''italic text''

* Bullet list
* Item 2

1. Numbered
2. List

[[Internal Page]]
[[Internal Page|Displayed text]]

[[File:Image.jpg|mini|right|Description]]

{{Template|parameter=value}}
```

#### What the editor does NOT do:
- no visual feedback (you only see the code)
- the wiki syntax has to be learned
- no drag & drop for images
- no real-time preview (except with special extensions)

---

### 3. Edit source code (alternative mode)

#### Functions:
- **Code editor**: for developers
- **Syntax highlighting**: coloured marking
- **Autocompletion**: optional
- **Line numbers**: optional

#### What the editor does NOT do:
- no WYSIWYG
- no live preview (without an extension)

---

## Comparing the editors

| Function | Visual Editor | Source-code editor |
|----------|---------------|--------------------|
| Visual editing | ✅ yes | ❌ no |
| Wiki syntax | ❌ (invisible) | ✅ directly visible |
| Templates | ✅ visual (limited) | ✅ complete |
| Tables | ✅ visual | ✅ syntax (complex) |
| Insert images | ✅ drag & drop | ✅ syntax |
| Live preview | ✅ yes | ❌ no (optional) |
| Learning curve | ✅ low | ❌ medium-high |
| Advanced functions | ❌ limited | ✅ full control |

---

## Recommendation

- **For beginners**: the visual editor (intuitive, no syntax needed)
- **For advanced users**: the source-code editor (more control, fast)
- **For template users**: the source-code editor (complete template support)

---

## Shortcuts

See [shortcuts.md](shortcuts.md) for a complete list of all keyboard shortcuts.

---

## The MediaWiki "Source Editor" variant (web UI)

Many MediaWiki installations also offer an improved source-code editor with:

- **Syntax highlighting**: coloured syntax marking
- **Line numbers**: for better orientation
- **Autocompletion**: for common syntax
- **Diff view**: the changes in context

---

## Extensions for the editor

### VisualEditor extensions:
- **Citoid**: automatic citation
- **Math**: LaTeX formulas
- **Graph**: diagrams
- **Kartographer**: maps
- **Score**: musical notation
- **Linter**: error detection

### Source-code extensions:
- **CodeEditor**: improved source-code editor
- **WikiEditor**: extended toolbar
- **SyntaxHighlight**: code highlighting
