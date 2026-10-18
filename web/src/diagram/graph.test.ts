import { describe, expect, it } from "vitest";

import type { ForeignKey } from "../types";
import { cardinalityLabel, endKinds } from "./graph";

/**
 * A declared key, with the two facts that decide how it is drawn left at the
 * defaults `craber-core` gives them.
 */
function fk(over: Partial<ForeignKey> = {}): ForeignKey {
  return {
    columns: ["user_id"],
    refTable: { name: "users" },
    refColumns: ["id"],
    onDelete: "noAction",
    onUpdate: "noAction",
    inferred: false,
    cardinality: "manyToOne",
    optional: false,
    ...over,
  };
}

/**
 * `endKinds` is the TypeScript half of a rule that also exists in Rust, as
 * `End::referencing` / `End::referenced` in `crates/craber-render/src/edge.rs`.
 * The two have to agree or one schema is two different diagrams — the worst
 * failure a diagramming tool has, because it is a wrong picture rather than an
 * error.
 *
 * These cases are the four the Rust side enumerates.
 */
describe("endKinds", () => {
  it("draws many at the referencing end of a many-to-one", () => {
    expect(endKinds(fk()).source).toBe("zero-many");
  });

  it("draws at most one at the referencing end of a one-to-one", () => {
    expect(endKinds(fk({ cardinality: "oneToOne" })).source).toBe("zero-one");
  });

  it("requires a parent when the key's own columns are not nullable", () => {
    expect(endKinds(fk({ optional: false })).target).toBe("one-only");
  });

  it("allows no parent when the key is optional", () => {
    expect(endKinds(fk({ optional: true })).target).toBe("zero-one");
  });

  /**
   * The referenced end turns on `optional` alone and the referencing end on
   * `cardinality` alone. Stated because reading one of the two facts at the
   * wrong end is the mistake that still draws a plausible diagram.
   */
  it("reads each end from its own fact", () => {
    const optionalOneToOne = endKinds(fk({ cardinality: "oneToOne", optional: true }));
    expect(optionalOneToOne).toEqual({ source: "zero-one", target: "zero-one" });
  });
});

describe("cardinalityLabel", () => {
  it("reads a many-to-one from whichever end is asking", () => {
    expect(cardinalityLabel(fk(), "from")).toBe("n:1");
    expect(cardinalityLabel(fk(), "to")).toBe("1:n");
  });

  it("reads the same from both ends of a one-to-one", () => {
    const one = fk({ cardinality: "oneToOne" });
    expect(cardinalityLabel(one, "from")).toBe("1:1");
    expect(cardinalityLabel(one, "to")).toBe("1:1");
  });
});
