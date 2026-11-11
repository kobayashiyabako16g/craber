import { describe, expect, it } from "vitest";

import { foreignKeyColumns, primaryKeyColumns, qualifiedName, resolveRef, tableIndex, uniqueColumns } from "./schema";
import type { Table, TableRef } from "@/types";
// Read as text, like the stylesheets in cssConventions.test.ts, so the file stays
// outside the bundle's module graph and needs no JSON-module typings.
import resolveCases from "../../../crates/craber-core/tests/cases/resolve.json?raw";

type ResolveCase = { case: string; tables: TableRef[]; ref: TableRef; expect: string | null };

function table(name: string, schema?: string): Table {
  return { name, ...(schema ? { schema } : {}), columns: [] };
}

describe("qualifiedName", () => {
  it("qualifies a namespaced table and leaves a bare one alone", () => {
    expect(qualifiedName(table("users", "public"))).toBe("public.users");
    expect(qualifiedName(table("users"))).toBe("users");
  });
});

/**
 * `resolveRef` mirrors `SchemaIndex::resolve` on the Rust side, and it is the
 * rule that decides whether an edge is drawn at all: a reference that does not
 * resolve is a relationship the reader never sees. The cases are the ones
 * `crates/craber-core/tests/resolve.rs` checks the Rust side against, so the
 * two cannot drift apart without one of the suites failing.
 */
describe("resolveRef", () => {
  const cases = JSON.parse(resolveCases) as ResolveCase[];

  it("has cases to check", () => {
    expect(cases.length).toBeGreaterThan(0);
  });

  it.each(cases)("$case", ({ tables, ref, expect: expected }) => {
    const known = tableIndex(tables.map(({ name, schema }) => table(name, schema ?? undefined)));
    expect(resolveRef(ref, known)).toBe(expected);
  });
});

describe("key columns", () => {
  it("reads the primary key, and tolerates a table without one", () => {
    expect([...primaryKeyColumns({ ...table("users"), pk: ["id"] })]).toEqual(["id"]);
    expect([...primaryKeyColumns(table("users"))]).toEqual([]);
  });

  it("collects every column of every foreign key, composite ones included", () => {
    const orders: Table = {
      ...table("orders"),
      fks: [
        {
          columns: ["tenant_id", "user_id"],
          refTable: { name: "users" },
          refColumns: ["tenant_id", "id"],
          onDelete: "noAction",
          onUpdate: "noAction",
          inferred: false,
          cardinality: "manyToOne",
          optional: false,
        },
      ],
    };
    expect([...foreignKeyColumns(orders)].sort()).toEqual(["tenant_id", "user_id"]);
  });

  it("counts a column as unique only when a unique index covers it alone", () => {
    const users: Table = {
      ...table("users"),
      indexes: [
        { columns: ["email"], unique: true },
        { columns: ["tenant_id", "handle"], unique: true },
        { name: "users_name_idx", columns: ["name"], unique: false },
      ],
    };
    expect([...uniqueColumns(users)]).toEqual(["email"]);
    expect([...uniqueColumns(table("users"))]).toEqual([]);
  });
});

describe("tableIndex", () => {
  /** The rule `normalize` applies on the Rust side, so a reference resolves to
   *  the same table in both. */
  it("keeps the first of two tables with one name", () => {
    const first = { ...table("users"), comment: "first" };
    const known = tableIndex([first, table("users")]);
    expect(known.size).toBe(1);
    expect(known.get("users")).toBe(first);
  });
});
