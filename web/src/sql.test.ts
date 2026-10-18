import { describe, expect, it } from "vitest";

import { schemaSql, sqlFileName } from "./sql";
import type { Schema } from "./types";

const shop: Schema = {
  schemaVersion: 2,
  name: "shop",
  tables: [
    {
      name: "users",
      columns: [
        { name: "id", dataType: "bigint", nullable: false },
        { name: "email", dataType: "text", nullable: false },
      ],
      pk: ["id"],
      indexes: [{ name: "users_email_key", columns: ["email"], unique: true }],
    },
    {
      name: "orders",
      columns: [
        { name: "id", dataType: "bigint", nullable: false },
        { name: "user_id", dataType: "bigint", nullable: true },
      ],
      pk: ["id"],
      fks: [
        {
          name: "orders_user_id_fkey",
          columns: ["user_id"],
          refTable: { name: "users" },
          refColumns: ["id"],
          onDelete: "cascade",
          onUpdate: "noAction",
          inferred: false,
          cardinality: "manyToOne",
          optional: true,
        },
      ],
    },
  ],
};

describe("schemaSql", () => {
  /**
   * A schema is a graph, so no table order puts every reference after its
   * target. Foreign keys go in a pass of their own at the end for that reason,
   * and this is the property that depends on it.
   */
  it("adds foreign keys after every table exists", () => {
    const sql = schemaSql(shop);
    const lastCreate = Math.max(sql.lastIndexOf("CREATE TABLE"), 0);
    expect(sql.indexOf("ADD CONSTRAINT")).toBeGreaterThan(lastCreate);
  });

  it("states the referential action the schema carries", () => {
    expect(schemaSql(shop)).toContain("ON DELETE CASCADE");
  });

  /**
   * A dashed edge is CrabER's guess, not a constraint anybody declared.
   * Emitting it as DDL would put a constraint into a database that never had
   * one, so it is counted in the header instead.
   */
  it("leaves an inferred key out of the DDL and says so", () => {
    const guessed: Schema = {
      ...shop,
      tables: [
        shop.tables![0]!,
        {
          ...shop.tables![1]!,
          fks: [{ ...shop.tables![1]!.fks![0]!, inferred: true }],
        },
      ],
    };
    const sql = schemaSql(guessed);
    expect(sql).not.toContain("ADD CONSTRAINT");
    expect(sql).toMatch(/--.*inferred/i);
  });

  it("quotes identifiers the standard way whatever dialect they came from", () => {
    expect(schemaSql(shop)).toContain('"users"');
  });

  it("ends with exactly one newline", () => {
    const sql = schemaSql(shop);
    expect(sql.endsWith("\n")).toBe(true);
    expect(sql.endsWith("\n\n")).toBe(false);
  });

  it("writes something for a schema with no tables at all", () => {
    expect(schemaSql({ schemaVersion: 2, name: "empty" })).toContain("empty");
  });
});

describe("sqlFileName", () => {
  it("names the file after the schema", () => {
    expect(sqlFileName({ schemaVersion: 2, name: "shop" })).toBe("shop.sql");
  });

  /** The name comes from a filename or a database name, so it is not trusted. */
  it("strips what does not belong in a filename", () => {
    expect(sqlFileName({ schemaVersion: 2, name: "my shop/../db" })).toBe("my-shop-..-db.sql");
  });

  it("falls back rather than producing a bare extension", () => {
    expect(sqlFileName({ schemaVersion: 2, name: "///" })).toBe("schema.sql");
  });
});
