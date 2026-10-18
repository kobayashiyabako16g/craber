import { describe, expect, it } from "vitest";

import { foreignKeyColumns, primaryKeyColumns, qualifiedName, resolveRef, tableIndex } from "./schema";
import type { Table } from "./types";

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
 * resolve is a relationship the reader never sees.
 */
describe("resolveRef", () => {
  it("prefers an exact qualified match", () => {
    const known = tableIndex([table("users", "public"), table("users", "audit")]);
    expect(resolveRef({ name: "users", schema: "audit" }, known)).toBe("audit.users");
  });

  it("falls back to a unique bare-name match", () => {
    const known = tableIndex([table("users", "public")]);
    expect(resolveRef({ name: "users" }, known)).toBe("public.users");
  });

  /**
   * Two schemas with the same table name is the case the fallback must not
   * guess at: picking either one draws an edge to a table the DDL did not name.
   */
  it("refuses an ambiguous bare name rather than picking one", () => {
    const known = tableIndex([table("users", "public"), table("users", "audit")]);
    expect(resolveRef({ name: "users" }, known)).toBeNull();
  });

  it("is null for a table the schema does not contain", () => {
    expect(resolveRef({ name: "ghosts" }, tableIndex([table("users")]))).toBeNull();
  });

  /** `.endsWith(".users")` must not match `power_users`. */
  it("does not match a name that merely ends with the reference", () => {
    const known = tableIndex([table("power_users")]);
    expect(resolveRef({ name: "users" }, known)).toBeNull();
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
});
