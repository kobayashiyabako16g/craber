import { describe, expect, it } from "vitest";

import type { ForeignKey, Table } from "@/types";

import type { Anchor } from "./anchors";
import { bakedNodes, litRelations, neighbourhood, toEdges } from "./graph";
import { decorateEdges, decorateNodes, type Highlight } from "./highlight";
import type { StepLit } from "./workflow";

const fk: ForeignKey = {
  columns: ["user_id"],
  refTable: { name: "users" },
  refColumns: ["id"],
  onDelete: "noAction",
  onUpdate: "noAction",
  inferred: false,
  cardinality: "manyToOne",
  optional: false,
};
/** orders → users, and products on its own. */
const tables = [
  { name: "users", columns: [{ name: "id" }], pk: ["id"] },
  { name: "orders", columns: [{ name: "user_id" }], fks: [fk] },
  { name: "products", columns: [{ name: "id" }], pk: ["id"] },
] as unknown as Table[];
const relations = toEdges(tables, 0);
const nodes = bakedNodes(tables, new Map());
const anchors = new Map<string, Anchor>([
  ["orders:0", { source: "s", target: "t", sides: { source: "left", target: "right" } }],
]);
const none: ReadonlySet<string> = new Set();

function focusOn(selected: string | null, hovered: string | null = null): Highlight {
  return {
    kind: "focus",
    selected,
    focus: neighbourhood(relations, selected),
    lit: litRelations(relations, [selected, hovered]),
  };
}

const step: StepLit = {
  access: new Map([["users", "write"]]),
  columns: new Map(),
  values: new Map(),
  edges: new Set(),
};

const byId = <T extends { id: string }>(items: T[]) => new Map(items.map((item) => [item.id, item]));

describe("decorateNodes", () => {
  it("dims nothing when nothing is selected or playing", () => {
    const drawn = decorateNodes(nodes, focusOn(null), none);
    expect(drawn.every((node) => node.className === undefined)).toBe(true);
  });

  it("dims what is outside a selection's neighbourhood, and marks the selection", () => {
    const drawn = byId(decorateNodes(nodes, focusOn("orders"), none));
    expect(drawn.get("orders")?.selected).toBe(true);
    expect(drawn.get("users")?.className).toBeUndefined();
    expect(drawn.get("products")?.className).toBe("craber-canvas-dimmed");
  });

  /** What the pointer lights stays readable inside a selection. */
  it("keeps a hovered table's relations lit inside another selection", () => {
    const drawn = byId(decorateNodes(nodes, focusOn("products", "orders"), none));
    expect(drawn.get("users")?.className).toBeUndefined();
    expect(drawn.get("users")?.data.related).toBe(true);
  });

  it("lets playback take over: only the step's tables are lit, and nothing is selected", () => {
    const drawn = byId(decorateNodes(nodes, { kind: "playback", step }, none));
    expect(drawn.get("users")?.data.access).toBe("write");
    expect(drawn.get("orders")?.className).toBe("craber-canvas-dimmed");
    expect([...drawn.values()].some((node) => node.selected)).toBe(false);
  });

  it("hides what is hidden, whatever else lights it", () => {
    const hidden = new Set(["users"]);
    expect(byId(decorateNodes(nodes, focusOn("users"), hidden)).get("users")?.hidden).toBe(true);
    expect(byId(decorateNodes(nodes, { kind: "playback", step }, hidden)).get("users")?.hidden).toBe(true);
  });
});

describe("decorateEdges", () => {
  it("lights a selected table's line and raises it above the bundle", () => {
    const [edge] = decorateEdges(relations, anchors, focusOn("orders"), none);
    expect(edge?.className).toContain("craber-edge-focus");
    expect(edge?.zIndex).toBe(1);
  });

  it("dims every other line while something is selected", () => {
    const [edge] = decorateEdges(relations, anchors, focusOn("products"), none);
    expect(edge?.className).toContain("craber-canvas-dimmed");
  });

  it("dims a line no step joins along during playback", () => {
    const [edge] = decorateEdges(relations, anchors, { kind: "playback", step }, none);
    expect(edge?.className).toContain("craber-canvas-dimmed");
  });

  it("names the marker for the face each end attached to", () => {
    const [edge] = decorateEdges(relations, anchors, focusOn(null), none);
    expect([edge?.markerStart, edge?.markerEnd]).toEqual(["craber-zero-many-left", "craber-one-only-right"]);
    expect([edge?.sourceHandle, edge?.targetHandle]).toEqual(["s", "t"]);
  });

  it("hides a line to a hidden table", () => {
    const [edge] = decorateEdges(relations, anchors, focusOn(null), new Set(["users"]));
    expect(edge?.hidden).toBe(true);
  });
});
