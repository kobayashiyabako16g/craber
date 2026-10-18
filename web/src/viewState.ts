/**
 * Everything about the view that is worth sharing, encoded in the URL.
 *
 * A published diagram has no server to keep state in, so the URL is the only
 * place a reader can point at "this table, at this zoom". The address bar
 * follows the view as it moves, and reading it back on load is what makes a
 * link copied out of it work.
 */

export type Viewport = { x: number; y: number; zoom: number };

export type ViewState = {
  /** Qualified name of the focused table, if any. */
  selected: string | null;
  /** Qualified names the reader has hidden from the canvas. */
  hidden: string[];
  /** Absent when the link carries no viewport, which means "fit the diagram". */
  viewport: Viewport | null;
};

export function readViewState(search: string = window.location.search): ViewState {
  const params = new URLSearchParams(search);
  return {
    selected: params.get("table"),
    hidden: (params.get("hide") ?? "").split(",").filter(Boolean),
    viewport: asViewport(params.get("at")),
  };
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
  if (state.hidden.length > 0) params.set("hide", state.hidden.join(","));
  if (state.viewport) {
    const { x, y, zoom } = state.viewport;
    params.set("at", [round(x), round(y), round(zoom, 3)].join(","));
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

function round(value: number, digits = 0): number {
  const factor = 10 ** digits;
  return Math.round(value * factor) / factor;
}
