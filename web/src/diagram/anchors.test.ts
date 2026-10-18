import { describe, expect, it } from "vitest";

import { anchorEdges, type Box } from "./anchors";
import type { Relation } from "./graph";
import { STUB_LENGTH } from "./handles";

const NODE_WIDTH = 260;

function box(x: number, y = 0): Box {
  return { x, y, width: NODE_WIDTH };
}

/** One relationship: `orders.user_id` → `users.id`. */
function relation(over: Partial<Relation> = {}): Relation {
  return {
    id: "orders.user_id->users.id",
    source: "orders",
    target: "users",
    data: {
      sourceEnd: "zero-many",
      targetEnd: "one-only",
      sourceColumn: "user_id",
      targetColumn: "id",
    },
    ...over,
  } as Relation;
}

function sidesOf(boxes: Record<string, Box>, over: Partial<Relation> = {}) {
  const edge = relation(over);
  const anchors = anchorEdges([edge], new Map(Object.entries(boxes)));
  return anchors.get(edge.id)?.sides;
}

/**
 * React Flow offsets each control point by half the gap between the curve's
 * two ends, so a line between two faces that nearly touch doubles back in
 * almost no space. Which pair of faces is right therefore depends on where the
 * tables currently are — these are the cases that decision turns on.
 */
describe("anchorEdges picks the faces by where the tables are", () => {
  it("uses the faces that look at each other when the tables are clear", () => {
    // The layout's own column spacing: target to the left of source.
    expect(sidesOf({ orders: box(400), users: box(0) })).toEqual({
      source: "left",
      target: "right",
    });
  });

  it("mirrors that when the target is to the right", () => {
    expect(sidesOf({ orders: box(0), users: box(400) })).toEqual({
      source: "right",
      target: "left",
    });
  });

  /**
   * Dragged into the same column, the facing pair's gap collapses and the line
   * would turn round inside a few pixels. The away-facing pair has a gap wider
   * than a node, so the curve swings out before it comes back.
   *
   * Stacked exactly, `leftward` is false — the target counts as rightward — so
   * the facing pair would be source-right/target-left and the away pair is the
   * opposite of that.
   */
  it("uses the away-facing pair when the tables are stacked", () => {
    expect(sidesOf({ orders: box(0, 0), users: box(0, 500) })).toEqual({
      source: "left",
      target: "right",
    });
  });

  /** The threshold is stated in `facingSides` as `2 × STUB_LENGTH`: room for
   *  the straight run each end makes before the curve between them starts. */
  it("switches pairs at twice the straight run", () => {
    const justWide = NODE_WIDTH + 2 * STUB_LENGTH;
    expect(sidesOf({ orders: box(0), users: box(justWide) })?.source).toBe("right");
    expect(sidesOf({ orders: box(0), users: box(justWide - 1) })?.source).toBe("left");
  });

  /**
   * A table referencing itself has no other table to face, and the two sides
   * the fallback would pick send the line across the node's own width.
   */
  it("puts both ends of a self-reference on one side", () => {
    const sides = sidesOf(
      { orders: box(0) },
      { id: "self", source: "orders", target: "orders" },
    );
    expect(sides).toEqual({ source: "right", target: "right" });
  });
});

describe("anchorEdges", () => {
  it("names the handle for the column the relationship joins", () => {
    const edge = relation();
    const anchors = anchorEdges([edge], new Map([["orders", box(400)], ["users", box(0)]]));
    expect(anchors.get(edge.id)).toMatchObject({
      source: "s-left-user_id",
      target: "t-right-id",
    });
  });

  /** A node that is not on the canvas — hidden, or filtered out — has no box. */
  it("skips an edge whose end has no box", () => {
    const edge = relation();
    expect(anchorEdges([edge], new Map([["orders", box(0)]])).size).toBe(0);
  });
});
