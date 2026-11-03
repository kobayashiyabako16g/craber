import { describe, expect, it } from "vitest";

import { tableIndex } from "./schema";
import type { Step, Table } from "../types";
import { stepLit, stepTarget, valueLabel } from "./flow";
import { toEdges } from "./graph";

const users: Table = {
  schema: "public",
  name: "users",
  columns: [{ name: "id", dataType: "bigint", nullable: false }],
  pk: ["id"],
};
const orders: Table = {
  schema: "public",
  name: "orders",
  columns: [
    { name: "id", dataType: "bigint", nullable: false },
    { name: "user_id", dataType: "bigint", nullable: false },
  ],
  pk: ["id"],
  fks: [
    {
      columns: ["user_id"],
      refTable: { schema: "public", name: "users" },
      refColumns: ["id"],
      onDelete: "noAction",
      onUpdate: "noAction",
      inferred: false,
      cardinality: "manyToOne",
      optional: false,
    },
  ],
};
const tables = [users, orders];
const known = tableIndex(tables);
const relations = toEdges(tables, 20);

const select: Step = {
  kind: "select",
  sql: "SELECT …",
  touches: [
    { table: { name: "users" }, access: "read", columns: ["id"] },
    { table: { name: "orders" }, access: "read", columns: ["user_id"] },
  ],
  joins: [
    {
      // Written the other way round from the foreign key, as SQL often is.
      left: { table: { name: "users" }, column: "id" },
      right: { table: { name: "orders" }, column: "user_id" },
    },
  ],
};

describe("stepLit", () => {
  it("resolves unqualified tables to their nodes", () => {
    const lit = stepLit(select, relations, known);
    expect([...lit.access]).toEqual([
      ["public.users", "read"],
      ["public.orders", "read"],
    ]);
    expect([...(lit.columns.get("public.orders") ?? [])]).toEqual(["user_id"]);
  });

  it("lights the relationship a join runs along, either way round", () => {
    const lit = stepLit(select, relations, known);
    expect(lit.edges.size).toBe(1);
    expect([...lit.edges]).toEqual(relations.map((edge) => edge.id));
  });

  it("lights no relationship for a join no foreign key backs", () => {
    const step: Step = {
      ...select,
      joins: [
        {
          left: { table: { name: "users" }, column: "id" },
          right: { table: { name: "orders" }, column: "id" },
        },
      ],
    };
    expect(stepLit(step, relations, known).edges.size).toBe(0);
  });

  it("keeps the strongest access when a table is touched twice", () => {
    const step: Step = {
      kind: "update",
      sql: "UPDATE …",
      touches: [
        { table: { name: "orders" }, access: "read" },
        { table: { name: "orders" }, access: "write", columns: ["id"] },
      ],
    };
    expect(stepLit(step, relations, known).access.get("public.orders")).toBe("write");
  });

  it("lights nothing for a table the diagram does not have", () => {
    const step: Step = {
      kind: "delete",
      sql: "DELETE …",
      touches: [{ table: { name: "coupons" }, access: "delete" }],
    };
    expect(stepLit(step, relations, known).access.size).toBe(0);
  });
});

describe("stepTarget", () => {
  it("is the table written, not the first one read", () => {
    const step: Step = {
      kind: "insert",
      sql: "INSERT …",
      touches: [
        { table: { name: "orders" }, access: "read" },
        { table: { name: "archive" }, access: "write" },
      ],
    };
    expect(stepTarget(step)).toBe("archive");
  });

  it("is the first table for a read-only step", () => {
    expect(stepTarget(select)).toBe("users");
  });
});

describe("stepLit's values", () => {
  const update: Step = {
    kind: "update",
    sql: "UPDATE orders SET total = 59.80 WHERE id = 100 AND total > 0",
    touches: [
      {
        table: { name: "orders" },
        access: "write",
        columns: ["total", "id"],
        values: [
          { column: "total", kind: "filter", text: "> 0" },
          { column: "total", kind: "set", text: "59.80" },
          { column: "id", kind: "filter", text: "= 100" },
        ],
      },
    ],
  };

  it("groups values by node and column, what is set before the conditions", () => {
    const orders = stepLit(update, relations, known).values.get("public.orders");
    expect(orders?.get("total")?.map(valueLabel)).toEqual(["← 59.80", "> 0"]);
    expect(orders?.get("id")?.map(valueLabel)).toEqual(["= 100"]);
  });

  it("has no entry for a table without values", () => {
    expect(stepLit(select, relations, known).values.size).toBe(0);
  });
});
