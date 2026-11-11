/**
 * Everything about the view that is worth sharing, encoded in the URL.
 *
 * A published diagram has no server to keep state in, so the URL is the only
 * place a reader can point at "this table, at this zoom". The address bar
 * follows the view as it moves, and reading it back on load is what makes a
 * link copied out of it work.
 */

import type { FitViewOptions } from "@xyflow/react";

import type { Table } from "@/types";

import { qualifiedName } from "./schema";

export type Viewport = { x: number; y: number; zoom: number };

export type ViewState = {
  /** Qualified name of the focused table, if any. */
  selected: string | null;
  /** Qualified names the reader has hidden from the canvas. */
  hidden: ReadonlySet<string>;
  /** Absent when the link carries no viewport, which means "fit the diagram". */
  viewport: Viewport | null;
  /** Name of the workflow being played back, if any. `?flow=` in the URL. */
  workflow: string | null;
  /** Index of its current step, from 0. `?step=` counts from 1, as the step
   *  bar does. */
  step: number;
};

export function readViewState(search: string = window.location.search): ViewState {
  const params = new URLSearchParams(search);
  return {
    selected: params.get("table"),
    hidden: new Set((params.get("hide") ?? "").split(",").filter(Boolean)),
    viewport: asViewport(params.get("at")),
    workflow: params.get("flow"),
    step: asStep(params.get("step")),
  };
}

/**
 * The view with every table this schema does not have dropped from it.
 *
 * A link outlives the schema it was copied from: a table gets renamed, or the
 * link is opened against another build. A selection that names no table would
 * dim the whole canvas around nothing, with no details panel to say why, so it
 * reads as no selection at all.
 */
export function forTables(state: ViewState, tables: readonly Table[]): ViewState {
  const known = new Set(tables.map(qualifiedName));
  return {
    ...state,
    selected: state.selected !== null && known.has(state.selected) ? state.selected : null,
    hidden: new Set([...state.hidden].filter((id) => known.has(id))),
  };
}

/** Everything a reader can do to the view. */
export type ViewAction =
  | { type: "select"; table: string | null }
  | { type: "toggleHidden"; table: string }
  /** Hide or show every one of `tables` — the sidebar's "all matching". */
  | { type: "setHidden"; tables: string[]; hidden: boolean }
  /** Start a workflow from its first step, or stop playing with null. */
  | { type: "play"; workflow: string | null }
  | { type: "step"; step: number }
  | { type: "recordViewport"; viewport: Viewport };

/**
 * The view after `action`.
 *
 * One function rather than a setter per field, so what each thing a reader
 * does changes — starting a workflow also rewinds it, say — is written once,
 * here, where it can be tested without a DOM.
 */
export function viewReducer(state: ViewState, action: ViewAction): ViewState {
  switch (action.type) {
    case "select":
      return { ...state, selected: action.table };
    case "toggleHidden": {
      const hidden = new Set(state.hidden);
      if (!hidden.delete(action.table)) hidden.add(action.table);
      return { ...state, hidden };
    }
    case "setHidden": {
      const hidden = new Set(state.hidden);
      for (const table of action.tables) {
        if (action.hidden) hidden.add(table);
        else hidden.delete(table);
      }
      return { ...state, hidden };
    }
    case "play":
      return { ...state, workflow: action.workflow, step: 0 };
    case "step":
      return { ...state, step: action.step };
    case "recordViewport":
      return { ...state, viewport: action.viewport };
  }
}

/** How a link that names a table frames it: close, but not nose-to-the-glass. */
export const FIT_ONE = { maxZoom: 1.2, padding: 0.4 } as const;

/**
 * How the diagram is framed on the first render.
 *
 * A link that names a table but carries no viewport means "look at this table":
 * fitting the whole diagram would drop the reader at the same place every other
 * link does. No table named means fit everything, which is what React Flow does
 * with no options at all. `initial` has been through `forTables`, so a table it
 * names is one this schema has.
 *
 * This is a prop rather than a `fitView` call in an effect. React Flow performs
 * its initial fit once it has *measured* the nodes, and an effect runs well
 * before that: on the first render every node still has `measured: undefined`,
 * so fitting to one of them has no box to fit and silently does nothing, and
 * React Flow's own fit then framed the whole diagram over the top of it.
 */
export function initialFit(initial: ViewState): FitViewOptions | undefined {
  return initial.selected ? { nodes: [{ id: initial.selected }], ...FIT_ONE } : undefined;
}

/**
 * The absolute URL for a view, as the address bar shows it.
 *
 * `at` defaults to the current location, the same way `readViewState` defaults
 * to the current query — so the pair can be exercised without a DOM.
 */
export function viewStateUrl(
  state: ViewState,
  at: { origin: string; pathname: string } = window.location,
): string {
  const params = new URLSearchParams();
  // Only what differs from the default, so the common link stays readable.
  if (state.selected) params.set("table", state.selected);
  if (state.hidden.size > 0) params.set("hide", [...state.hidden].join(","));
  if (state.viewport) {
    const { x, y, zoom } = state.viewport;
    params.set("at", [round(x), round(y), round(zoom, 3)].join(","));
  }
  if (state.workflow) {
    params.set("flow", state.workflow);
    params.set("step", String(state.step + 1));
  }
  const query = params.toString();
  const { origin, pathname } = at;
  return query ? `${origin}${pathname}?${query}` : `${origin}${pathname}`;
}

function asViewport(value: string | null): Viewport | null {
  if (!value) return null;
  const [x, y, zoom] = value.split(",").map(Number);
  if (x === undefined || y === undefined || zoom === undefined) return null;
  if (!Number.isFinite(x) || !Number.isFinite(y) || !Number.isFinite(zoom) || zoom <= 0) {
    return null;
  }
  return { x, y, zoom };
}

/** A step number that is not a positive integer starts the workflow over. */
function asStep(value: string | null): number {
  const step = Number(value);
  return Number.isInteger(step) && step >= 1 ? step - 1 : 0;
}

function round(value: number, digits = 0): number {
  const factor = 10 ** digits;
  return Math.round(value * factor) / factor;
}
