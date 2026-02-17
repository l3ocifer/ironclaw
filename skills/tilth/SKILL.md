---
name: tilth
description: AST-aware code intelligence — smart file reading, symbol search, and structural navigation via tree-sitter. Reduces agent code navigation cost by 22-29%.
homepage: https://github.com/jahala/tilth
metadata:
  category: development
  requires: tilth binary (cargo install tilth)
---

# tilth — Code Intelligence for Agents

Three core tools: `tilth_search`, `tilth_read`, `tilth_files`.

## Key Principles

- **Use tilth tools for ALL code navigation** — replaces grep, cat, find, ls with AST-aware equivalents.
- **Expanded search results include full source** — do NOT re-read files already shown in output.
- **Always pass `context`** (the file you're editing) to boost nearby results.
- **Multi-symbol search**: comma-separated names for cross-file tracing in one call.

## tilth_read

Read a file. Small files → full content. Large files → structural outline.

- `path`: single file
- `paths`: array for batch reads
- `section`: line range (`"45-89"`) or markdown heading (`"## Architecture"`)
- `full`: true to force full content on large files

## tilth_search

AST-aware code search. Returns ranked results with structural context.

- `query`: symbol name, text, `/regex/`, or comma-separated symbols (max 5)
- `kind`: `"symbol"` (default) | `"content"` | `"regex"` | `"callers"`
- `expand`: number of results to show with full source (default 2)
- `context`: path of file being edited (boosts nearby results)
- `scope`: directory to search within

Expanded definitions include `── calls ──` footer with resolved callees — follow these instead of manually searching.

## tilth_files

Find files by glob pattern with token estimates. Respects .gitignore.

- `pattern`: glob e.g. `"*.test.ts"`, `"src/**/*.rs"`
- `scope`: directory to search within

## Installation

```bash
cargo install tilth
tilth install claude-code
```
