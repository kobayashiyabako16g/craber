import { describe, expect, it } from "vitest";

import { forTables, readViewState, viewStateUrl } from "./viewState";

/** The page a link would be copied from. Both functions take their input as an
 *  argument, so none of this needs a DOM. */
const page = { origin: "https://example.test", pathname: "/er/" };
/** No workflow playing — what every link without `?flow=` reads as. */
const noFlow = { flow: null, step: 0 };
describe("readViewState", () => {
  it("is all defaults for an empty query", () => {
    expect(readViewState("")).toEqual({ selected: null, hidden: [], viewport: null, ...noFlow });
  });

  it("reads the selected table and the hidden list", () => {
    expect(readViewState("?table=public.users&hide=a,b")).toEqual({
      selected: "public.users",
      hidden: ["a", "b"],
      viewport: null,
      ...noFlow,
    });
  });

  /** `"".split(",")` is `[""]`, so the filter is what keeps `?hide=` empty. */
  it("reads an empty hide parameter as nothing hidden", () => {
    expect(readViewState("?hide=").hidden).toEqual([]);
  });

  it("reads a viewport", () => {
    expect(readViewState("?at=10,20,1.5").viewport).toEqual({ x: 10, y: 20, zoom: 1.5 });
  });

  /**
   * A hand-edited or truncated link should fit the diagram rather than put the
   * canvas somewhere unusable — a zero or negative zoom especially.
   */
  it.each(["?at=10,20", "?at=a,b,c", "?at=10,20,0", "?at=10,20,-1"])(
    "ignores an unusable viewport in %s",
    (search) => {
      expect(readViewState(search).viewport).toBeNull();
    },
  );
});

describe("viewStateUrl", () => {
  /**
   * Only what differs from the default is encoded, so the common link — one
   * table selected, nothing hidden — stays short enough to paste in a message.
   */
  it("omits the defaults", () => {
    const url = viewStateUrl({ selected: null, hidden: [], viewport: null, ...noFlow }, page);
    expect(new URL(url).search).toBe("");
  });

  it("round trips through readViewState", () => {
    const state = { selected: "public.users", hidden: ["a", "b"], viewport: null, ...noFlow };
    const url = viewStateUrl(state, page);
    expect(readViewState(new URL(url).search)).toEqual(state);
  });

  /**
   * The whole point of `?at=`: a link carries where the reader was looking, not
   * just what they had selected. `useViewSync` passed a literal `null` here for
   * a while, which left the reading half with nothing to read.
   */
  it("round trips a viewport too", () => {
    const state = {
      selected: "public.orders",
      hidden: [],
      viewport: { x: 120, y: -40, zoom: 0.85 },
      ...noFlow,
    };
    expect(readViewState(new URL(viewStateUrl(state, page)).search)).toEqual(state);
  });

  /** Rounded so a drifting float does not make every link unique. */
  it("rounds a viewport to whole pixels and three digits of zoom", () => {
    const url = viewStateUrl(
      { selected: null, hidden: [], viewport: { x: 10.4, y: -20.6, zoom: 1.23456 }, ...noFlow },
      page,
    );
    expect(new URL(url).searchParams.get("at")).toBe("10,-21,1.235");
  });

  /** A link into a playback opens on the same step, counted from 1 as shown. */
  it("round trips a workflow and its step", () => {
    const state = { selected: null, hidden: [], viewport: null, flow: "checkout", step: 2 };
    const url = viewStateUrl(state, page);
    expect(new URL(url).searchParams.get("step")).toBe("3");
    expect(readViewState(new URL(url).search)).toEqual(state);
  });
});

describe("readViewState's step", () => {
  it.each(["?flow=a", "?flow=a&step=0", "?flow=a&step=-2", "?flow=a&step=x", "?flow=a&step=1.5"])(
    "starts the workflow over for %s",
    (search) => {
      expect(readViewState(search).step).toBe(0);
    },
  );
});

describe("forTables", () => {
  const tables = [{ name: "users", schema: "public", columns: [] }, { name: "orders", columns: [] }];

  it("keeps a selection and hidden tables the schema has", () => {
    const state = { ...readViewState("?table=public.users&hide=orders") };
    expect(forTables(state, tables)).toEqual(state);
  });

  /** A stale `?table=` used to dim every node around a table that was not there. */
  it("drops a selection the schema does not have", () => {
    expect(forTables(readViewState("?table=ghost"), tables).selected).toBeNull();
  });

  it("drops hidden names the schema does not have", () => {
    expect(forTables(readViewState("?hide=ghost,orders"), tables).hidden).toEqual(["orders"]);
  });
});
