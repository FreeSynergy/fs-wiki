---
title: Synthetic wiki page
tags: [wiki, test]
---

# Wiki syntax tour ^top

A paragraph with **strong**, *emphasis*, ~~strike~~, `code` and a [link](https://example.org "Title").
Second line of the same paragraph.

See [[projects/alpha]] and [[projects/alpha|the alpha project]] or [[projects/alpha#Goals]] and
[[notes/daily#^abc123]]. Embed: ![[images/logo.png]]

- [ ] open task
- [x] done task
  - nested bullet
  - another one
- plain item

1. first
2. second

> [!note] A callout
> Body of the callout.

> [!warning]- Folded
>
> Second paragraph.

!!! info "Classic admonition"
    Body of the mkdocs admonition.

    Another paragraph.

A footnote reference[^1] here.

[^1]: The footnote text.

| Name | Value |
|------|:-----:|
| a | **b** |
| c | d \| e |

```rust
fn main() {}
```

---

Final line with a trailing block ref. ^end
