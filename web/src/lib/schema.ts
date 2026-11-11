import type { Table, TableRef } from "@/types";

export function qualifiedName(table: Table | TableRef): string {
  return table.schema ? `${table.schema}.${table.name}` : table.name;
}

/** Every table by its qualified name, which is also its node id on the canvas. */
export function tableIndex(tables: Table[]): ReadonlyMap<string, Table> {
  return new Map(tables.map((table) => [qualifiedName(table), table]));
}

/**
 * The table a reference points at, or null when it is missing or ambiguous.
 *
 * A qualified reference matches its qualified name, and failing that the
 * unqualified table of that name: an unqualified table lives in whatever the
 * default schema is. A table in another named schema never matches. An
 * unqualified reference matches by bare name, but only when exactly one table
 * in the whole schema has that name. This is `SchemaIndex::resolve` on the
 * Rust side, and it is the rule that decides whether an edge gets drawn at
 * all — both are held to `crates/craber-core/tests/cases/resolve.json`.
 */
export function resolveRef(ref: TableRef, known: ReadonlyMap<string, Table>): string | null {
  if (ref.schema) {
    const qualified = `${ref.schema}.${ref.name}`;
    if (known.has(qualified)) return qualified;
    return known.has(ref.name) ? ref.name : null;
  }
  const matches = [...known].filter(([, table]) => table.name === ref.name);
  return matches.length === 1 ? (matches[0]?.[0] ?? null) : null;
}

/** Column names that are part of the primary key. */
export function primaryKeyColumns(table: Table): ReadonlySet<string> {
  return new Set(table.pk ?? []);
}

/** Column names that participate in any foreign key. */
export function foreignKeyColumns(table: Table): ReadonlySet<string> {
  return new Set((table.fks ?? []).flatMap((fk) => fk.columns));
}

/**
 * Column names that a unique index covers on its own. A composite unique index
 * `(tenant_id, email)` makes neither column unique, so it is not counted.
 */
export function uniqueColumns(table: Table): ReadonlySet<string> {
  return new Set(
    (table.indexes ?? []).flatMap((index) => (index.unique && index.columns.length === 1 ? index.columns : [])),
  );
}
