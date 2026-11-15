import { qualifiedName } from "./schema";
import type { Table } from "@/types";

/**
 * What the search box finds.
 *
 * A reader looking for `user_id` is as often looking for the tables that carry
 * it as for a table of that name, so a table matches on its own name or on any
 * of its columns'. The sidebar lists the matches and the canvas marks them —
 * both from this, so the two cannot disagree about what was found.
 */
export type SearchMatches = {
  /** Qualified names of the tables that match, by name or by a column. */
  tables: ReadonlySet<string>;
  /** Per table, the columns whose names match. */
  columns: ReadonlyMap<string, ReadonlySet<string>>;
};

/** The tables and columns `query` finds, or null when there is no query. */
export function matchTables(tables: Table[], query: string): SearchMatches | null {
  const needle = query.trim().toLowerCase();
  if (!needle) return null;

  const matched = new Set<string>();
  const columns = new Map<string, ReadonlySet<string>>();
  for (const table of tables) {
    const id = qualifiedName(table);
    const names = (table.columns ?? [])
      .map((column) => column.name)
      .filter((name) => name.toLowerCase().includes(needle));
    if (names.length > 0) columns.set(id, new Set(names));
    if (names.length > 0 || id.toLowerCase().includes(needle)) matched.add(id);
  }
  return { tables: matched, columns };
}
