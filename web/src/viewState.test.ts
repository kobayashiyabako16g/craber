import { describe, expect, it } from "vitest";

import { readViewState, viewStateUrl } from "./viewState";

/** The page a link would be copied from. Both functions take their input as an
 *  argument, so none of this needs a DOM. */
const page = { origin: "https://example.test", pathname: "/er/" };
describe("readViewState", () => {
  it("is all defaults for an empty query", () => {
    expect(readViewState("")).toEqual({ selected: null, hidden: [], viewport: null });
  });

  it("reads the selected table and the hidden list", () => {
    expect(readViewState("?table=public.users&hide=a,b")).toEqual({
      selected: "public.users",
      hidden: ["a", "b"],
      viewport: null,
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
    const url = viewStateUrl({ selected: null, hidden: [], viewport: null }, page);
    expect(new URL(url).search).toBe("");
  });

  it("round trips through readViewState", () => {
    const state = { selected: "public.users", hidden: ["a", "b"], viewport: null };
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
    };
    expect(readViewState(new URL(viewStateUrl(state, page)).search)).toEqual(state);
  });

  /** Rounded so a drifting float does not make every link unique. */
  it("rounds a viewport to whole pixels and three digits of zoom", () => {
    const url = viewStateUrl(
      { selected: null, hidden: [], viewport: { x: 10.4, y: -20.6, zoom: 1.23456 } },
      page,
    );
    expect(new URL(url).searchParams.get("at")).toBe("10,-21,1.235");
  });
});
