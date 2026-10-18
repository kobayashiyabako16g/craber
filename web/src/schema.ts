import { SCHEMA_VERSION } from "./generated/metrics";
import type { Schema, Table, TableRef } from "./types";

/**
 * The `schema.json` shape this build reads, generated from
 * `crates/craber-core/src/ir/` — it used to be written out by hand here, with
 * only a comment asking the two to agree.
 *
 * The site and the binary that produced `schema.json` are versioned
 * independently — a diagram published months ago is still served by whatever
 * bundle is deployed today — so the reader checks rather than assumes.
 */
export { SCHEMA_VERSION };

export async function loadSchema(url = "schema.json"): Promise<Schema> {
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`could not load ${url}: ${response.status} ${response.statusText}`);
  }
  const schema = (await response.json()) as Schema;
  if (schema.schemaVersion !== SCHEMA_VERSION) {
    throw new Error(
      `schema.json is version ${schema.schemaVersion}, this build reads version ${SCHEMA_VERSION}`,
    );
  }
  return schema;
}

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
 * A reference may or may not carry a schema qualifier, while node ids always
 * use the table's own qualification — so an unqualified reference falls back to
 * a unique bare-name match. This mirrors `SchemaIndex::resolve` on the Rust
 * side, and is the rule that decides whether an edge gets drawn at all.
 */
export function resolveRef(ref: TableRef, known: ReadonlyMap<string, Table>): string | null {
  const qualified = ref.schema ? `${ref.schema}.${ref.name}` : ref.name;
  if (known.has(qualified)) return qualified;
  const matches = [...known.keys()].filter((id) => id === ref.name || id.endsWith(`.${ref.name}`));
  return matches.length === 1 ? (matches[0] ?? null) : null;
}

/** Column names that are part of the primary key. */
export function primaryKeyColumns(table: Table): ReadonlySet<string> {
  return new Set(table.pk ?? []);
}

/** Column names that participate in any foreign key. */
export function foreignKeyColumns(table: Table): ReadonlySet<string> {
  return new Set((table.fks ?? []).flatMap((fk) => fk.columns));
}
