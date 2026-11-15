import { describe, expect, it } from "vitest";

import { matchTables } from "./search";
import type { Table } from "@/types";

function table(name: string, columns: string[], schema?: string): Table {
  return {
    name,
    schema,
    columns: columns.map((column) => ({ name: column, dataType: "int", nullable: false })),
  } as Table;
}

const tables = [
  table("users", ["id", "email"]),
  table("orders", ["id", "user_id"]),
  table("audit", ["at"], "log"),
];

describe("matchTables", () => {
  it("finds nothing to say about a blank query", () => {
    expect(matchTables(tables, "")).toBeNull();
    expect(matchTables(tables, "   ")).toBeNull();
  });

  it("matches a table by its name, without any of its columns", () => {
    const found = matchTables(tables, "ORDER");
    expect([...(found?.tables ?? [])]).toEqual(["orders"]);
    expect(found?.columns.size).toBe(0);
  });

  it("matches a table by a column, and says which", () => {
    const found = matchTables(tables, "user");
    expect([...(found?.tables ?? [])]).toEqual(["users", "orders"]);
    expect(found?.columns.get("users")).toBeUndefined();
    expect([...(found?.columns.get("orders") ?? [])]).toEqual(["user_id"]);
  });

  it("matches on the qualified name", () => {
    expect([...(matchTables(tables, "log.")?.tables ?? [])]).toEqual(["log.audit"]);
  });

  it("matches no table when nothing carries the text", () => {
    expect(matchTables(tables, "zzz")?.tables.size).toBe(0);
  });
});
