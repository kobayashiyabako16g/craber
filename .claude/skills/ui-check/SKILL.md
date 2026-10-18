---
name: ui-check
description: Look at CrabER's diagram in a real browser before calling a frontend change done. Use when asked to check the UI, see the screen, take a screenshot, start the app, or when something about the rendering looks wrong. Trigger keywords - "画面を見て", "見た目", "スクショ", "UI を確認", "描画がおかしい", "アプリを起動して", "screenshot", "visual check", "render", "React Flow", "web/".
version: 0.1.0
tags: [craber, web, ui, browser, screenshot, visual]
keywords: [ui, screenshot, browser, chrome, react-flow, vite, visual, 画面, 見た目, スクショ]
updated: 2026-08-30
---

# Looking at CrabER on screen

`web/` renders `schema.json` with React Flow. Tests and `tsc` say nothing about
whether the diagram is legible, so a frontend change is not finished until it
has been looked at. This is how to look at it — via the `claude-in-chrome`
tools, against the ordinary Vite dev server. Nothing else is installed for it.

## The loop

1. **Start the app** in the background:

   ```console
   $ just ui                                    # fixtures/tiny.sql
   $ just ui examples/01-quickstart/blog.sql    # or any example schema
   ```

   This exports the schema to `web/public/schema.json` and starts Vite. Read the
   URL out of the output — usually `http://localhost:5173`, but Vite moves up a
   port when that one is taken, so do not assume it.

2. **Open a tab.** `tabs_context_mcp` first, then `tabs_create_mcp` and
   `navigate`. Never take over a tab the user is already using.

3. **Screenshot with `computer` and actually look at it.** The point of this
   skill is to replace guessing. Do not adjust CSS from a description of the
   problem when a screenshot is one call away.

4. **Read the console** with `read_console_messages` every time. The diagram can
   look right while React is throwing — `App.tsx` catches schema load failures
   itself, so a silent-looking page is not evidence of a clean one. That tool
   only records from its first call onward, so call it once and then reload, or
   the page-load messages are simply not there to find.

5. **Iterate.** Vite's HMR picks up edits under `web/src`; just reload. `.sql`
   files are not watched, so re-run `just sample <file>` and reload after
   changing which schema is on screen.

6. **Clean up**: `just sample-reset`, close the tab, stop the dev server.

## Check the count, not just the picture

When a screenshot is ambiguous — nodes overlapping, an edge you cannot tell is
there — `javascript_tool` settles it against the DOM:

```js
const s = await fetch("schema.json").then((r) => r.json());
({
  nodes: document.querySelectorAll(".react-flow__node").length,
  edges: document.querySelectorAll(".react-flow__edge").length,
  inferred: document.querySelectorAll(".craber-edge-inferred").length,
  tables: s.tables.length,
  fks: s.tables.reduce((n, t) => n + (t.fks ?? []).length, 0),
})
```

`nodes` must equal `tables`. `edges` is allowed to come out lower than `fks` —
`toEdges()` in `diagram/graph.ts` drops a foreign key whose target it
cannot resolve — but a gap there
is a finding, not noise.

## The error screens count too

`App.tsx` and `schema.ts` have failure paths that no test renders. If you touch
them, put them on screen: break `web/public/schema.json`, reload, then
`just sample-reset`.

| To see | Do this to web/public/schema.json |
|---|---|
| The load-failure message | `rm` it |
| The version-mismatch message | set `schemaVersion` to `2` |
| An empty canvas | set `tables` to `[]` |

Deleting the file does not produce the 404 branch under the dev server: Vite
falls back to `index.html` with a 200, so `loadSchema()` gets past `response.ok`
and fails in `response.json()` instead. The screen you get is the raw
`SyntaxError`, not the message `schema.ts` writes for a failed fetch.

## What to look for

- **Overlapping nodes are not a CSS bug.** Positions come from `craber-layout`
  and the frontend never re-runs a layout. Fix it in `crates/craber-layout`.
- **Inferred foreign keys are dashed.** `fk.inferred` maps to
  `craber-edge-inferred`; a relationship CrabER guessed must never look like one
  it read from a constraint.
- **The wordmark is `CrabER`.** See the naming table in `CONTRIBUTING.md`.

## Do not

- Leave `web/public/schema.json` dirty. It is a committed file; finish with
  `just sample-reset`.
- Hand-edit `web/src/types` — it is generated (`just bindings`).
- Make nodes draggable or compute a layout in the frontend.
- Add Playwright or any other screenshot dependency to `web/package.json` or
  `flake.nix`. The browser is already there; keeping it out of the project's
  dependencies is the point of this setup.

## If the extension blocks you

`claude-in-chrome` needs per-site permission. If `localhost` is refused, stop
and ask the user to allow it in the extension rather than looking for a way
around it.
