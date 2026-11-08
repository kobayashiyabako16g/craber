# Agent Instructions — web/

React 19 + React Flow (`@xyflow/react`) + Vite. Reads `schema.json`; never lays out tables.

## Commands
| Task | Command |
|------|---------|
| Test file | `pnpm vitest run src/lib/graph.test.ts` |
| All tests | `pnpm test` |
| Typecheck | `pnpm typecheck` |
| Typecheck + build (regenerates bindings) | `just web-build` |
| Look at the UI | `just ui [file]`, then `just sample-reset` |

## Key Conventions
- Components live in `src/components/<Name>/` as `<Name>.tsx` with `<Name>.css` beside it.
- React-free logic goes in `src/lib/`, hooks in `src/hooks/`.
- Imports leaving their own directory use `@/` (= `src/`); within it, `./`.
- View state lives in the URL (`src/lib/viewState.ts`).
- CSS classes: `craber-<block>[-<element>]`, states as `is-<state>`.
- `src/theme.css` holds only styles shared by several components.
- Register a new stylesheet or block in `OWNERS` in `src/classNames.test.ts`.
- vitest covers only `src/lib/` and `src/classNames.test.ts`; components have no unit tests.
- Verify UI changes in Chrome via `.claude/skills/ui-check` (screenshot + console).
