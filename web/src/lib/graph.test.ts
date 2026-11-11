import { describe, expect, it } from "vitest";

import type { ForeignKey, Table } from "@/types";
import {
  anchorColumns,
  cardinalityLabel,
  endCounts,
  endKinds,
  litRelations,
  neighbourhood,
  relationshipsOf,
  toEdges,
} from "./graph";

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

/** Every combination of the two facts `endKinds` reads. */
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
    expect(cardinalityLabel("manyToOne", "from")).toBe("n:1");
    expect(cardinalityLabel("manyToOne", "to")).toBe("1:n");
  });

  it("reads the same from both ends of a one-to-one", () => {
    expect(cardinalityLabel("oneToOne", "from")).toBe("1:1");
    expect(cardinalityLabel("oneToOne", "to")).toBe("1:1");
  });
});

describe("toEdges", () => {
  /** `orders.user_id` → `users.id`, with the key's own facts overridable. */
  function edge(over: Partial<ForeignKey> = {}) {
    const tables = [
      { name: "users", columns: [{ name: "id" }], pk: ["id"] },
      { name: "orders", columns: [{ name: "id" }, { name: "user_id" }], fks: [fk(over)] },
    ] as unknown as Table[];
    const [relation] = toEdges(tables, 0);
    if (!relation) throw new Error("orders.user_id did not resolve to users");
    return relation;
  }

  it("carries the key's cardinality and whether it was inferred", () => {
    expect(edge().data?.cardinality).toBe("manyToOne");
    expect(edge({ cardinality: "oneToOne" }).data?.cardinality).toBe("oneToOne");
    expect(edge({ inferred: true }).data?.inferred).toBe(true);
    expect(edge({ inferred: true }).className).toBe("craber-edge-inferred");
  });

  it("is keyed by table and key index, joining the rows the key names", () => {
    const relation = edge();
    expect(relation.id).toBe("orders:0");
    expect([relation.data?.sourceColumn, relation.data?.targetColumn]).toEqual(["user_id", "id"]);
  });

  it("lands on the referenced primary key when the key names no column", () => {
    expect(edge({ refColumns: [] }).data?.targetColumn).toBe("id");
  });

  it("attaches to the node's middle when a column is not one the node draws", () => {
    expect(edge({ columns: ["ghost"] }).data?.sourceColumn).toBe("");
  });

  it("skips a key whose table is not in the schema", () => {
    const tables = [{ name: "orders", columns: [], fks: [fk({ refTable: { name: "ghost" } })] }];
    expect(toEdges(tables as unknown as Table[], 0)).toEqual([]);
  });

  it("routes a self-reference orthogonally, and everything else as a curve", () => {
    const tables = [{ name: "users", columns: [{ name: "id" }, { name: "user_id" }], pk: ["id"], fks: [fk()] }];
    const [loop] = toEdges(tables as unknown as Table[], 30);
    expect(loop?.type).toBe("smoothstep");
    expect(loop?.pathOptions).toEqual({ offset: 30 });
    expect(edge().type).toBe("relation");
  });
});

describe("litRelations", () => {
  /** users ← orders, users ← sessions, and products unrelated to either. */
  const tables = [
    { name: "users", columns: [{ name: "id" }], pk: ["id"] },
    { name: "orders", columns: [{ name: "user_id" }], fks: [fk()] },
    { name: "sessions", columns: [{ name: "user_id" }], fks: [fk()] },
    { name: "products", columns: [{ name: "id" }], pk: ["id"] },
  ] as unknown as Table[];
  const edges = toEdges(tables, 0);

  it("lights nothing when no table is selected or hovered", () => {
    const lit = litRelations(edges, [null, null]);
    expect(lit.edges.size).toBe(0);
    expect(lit.tables.size).toBe(0);
  });

  it("lights a table's relationships, their far ends, and the rows they join", () => {
    const lit = litRelations(edges, [null, "orders"]);
    expect([...lit.edges]).toEqual(["orders:0"]);
    expect([...lit.tables].sort()).toEqual(["orders", "users"]);
    expect([...(lit.columns.get("orders") ?? [])]).toEqual(["user_id"]);
    expect([...(lit.columns.get("users") ?? [])]).toEqual(["id"]);
  });

  it("lights the selected and the hovered table together", () => {
    const lit = litRelations(edges, ["orders", "sessions"]);
    expect([...lit.edges].sort()).toEqual(["orders:0", "sessions:0"]);
    expect(lit.tables.has("products")).toBe(false);
  });
});

describe("relationships, read off the edges", () => {
  /** users ← orders, users ← users (a self-reference), products alone. */
  const tables = [
    { name: "users", columns: [{ name: "id" }, { name: "user_id" }], pk: ["id"], fks: [fk()] },
    { name: "orders", columns: [{ name: "user_id" }], fks: [fk()] },
    { name: "products", columns: [{ name: "id" }], pk: ["id"] },
  ] as unknown as Table[];
  const edges = toEdges(tables, 0);

  it("anchors only the rows a drawn relationship joins", () => {
    const anchors = anchorColumns(tables, edges);
    expect([...(anchors.get("users") ?? [])].sort()).toEqual(["id", "user_id"]);
    expect([...(anchors.get("orders") ?? [])]).toEqual(["user_id"]);
    expect(anchors.get("products")?.size).toBe(0);
  });

  it("lists a table's keys and the keys pointing at it, the self-reference once", () => {
    const listed = relationshipsOf("users", edges);
    expect(listed.map((r) => [r.other, r.from, r.to, r.cardinality])).toEqual([
      ["users", "users.user_id", "users.id", "n:1"],
      ["orders", "orders.user_id", "users.id", "1:n"],
    ]);
    expect(new Set(listed.map((r) => r.id)).size).toBe(listed.length);
  });

  it("takes a table and its neighbours as the neighbourhood", () => {
    expect([...(neighbourhood(edges, "orders") ?? [])].sort()).toEqual(["orders", "users"]);
    expect(neighbourhood(edges, null)).toBeNull();
  });

  it("counts rows from the referencing end", () => {
    expect(endCounts("manyToOne")).toEqual(["n", "1"]);
    expect(endCounts("oneToOne")).toEqual(["1", "1"]);
  });
});
