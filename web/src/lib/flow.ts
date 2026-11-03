import { resolveRef } from "./schema";
import type { Access, ColumnValue, Step, StepKind, Table } from "@/types";
import type { Relation } from "./graph";

/**
 * A workflow step → what it lights on the canvas.
 *
 * No React, like `graph.ts`. A step names tables as the SQL wrote them, so each
 * is resolved to a node with `resolveRef` — the same rule that decides which
 * edges exist — and a table the diagram does not have lights nothing.
 */

/** What one step lights: the tables it touches, how, and the rows involved. */
export type StepLit = {
  /** Per table, the strongest thing the step does to it. */
  access: ReadonlyMap<string, Access>;
  /** Per table, the columns the statement names. */
  columns: ReadonlyMap<string, ReadonlySet<string>>;
  /** Per table and column, the values the SQL writes or filters by — what it
   *  sets first, then the conditions, as a row shows them. */
  values: ReadonlyMap<string, ReadonlyMap<string, readonly ColumnValue[]>>;
  /** Relationships the step's `ON a = b` joins run along. */
  edges: ReadonlySet<string>;
};

const STRENGTH: Record<Access, number> = { read: 0, write: 1, delete: 2 };
const VALUE_ORDER: Record<ColumnValue["kind"], number> = { set: 0, filter: 1 };

/** A value as a row shows it: `← 59.80` for what is written, the condition
 *  as written (`= 100`) for a filter. */
export function valueLabel(value: ColumnValue): string {
  return value.kind === "set" ? `← ${value.text}` : value.text;
}

export function stepLit(step: Step, relations: Relation[], known: ReadonlyMap<string, Table>): StepLit {
  const access = new Map<string, Access>();
  const columns = new Map<string, Set<string>>();
  const values = new Map<string, Map<string, ColumnValue[]>>();
  for (const touch of step.touches ?? []) {
    const id = resolveRef(touch.table, known);
    if (!id) continue;
    const current = access.get(id);
    if (!current || STRENGTH[touch.access] > STRENGTH[current]) access.set(id, touch.access);
    const names = columns.get(id) ?? new Set<string>();
    for (const column of touch.columns ?? []) names.add(column);
    columns.set(id, names);
    const byColumn = values.get(id) ?? new Map<string, ColumnValue[]>();
    for (const value of touch.values ?? []) {
      byColumn.set(value.column, [...(byColumn.get(value.column) ?? []), value]);
    }
    if (byColumn.size > 0) values.set(id, byColumn);
  }
  for (const byColumn of values.values()) {
    for (const [column, list] of byColumn) {
      byColumn.set(column, [...list].sort((a, b) => VALUE_ORDER[a.kind] - VALUE_ORDER[b.kind]));
    }
  }

  // A join lights the relationship it runs along, whichever way round the
  // `ON` clause was written. A join no foreign key backs draws nothing: there
  // is no line on the diagram for it to light.
  const edges = new Set<string>();
  for (const join of step.joins ?? []) {
    const left = resolveRef(join.left.table, known);
    const right = resolveRef(join.right.table, known);
    if (!left || !right) continue;
    for (const edge of relations) {
      const along = (a: string, aColumn: string, b: string, bColumn: string) =>
        edge.source === a &&
        edge.data?.sourceColumn === aColumn &&
        edge.target === b &&
        edge.data.targetColumn === bColumn;
      if (
        along(left, join.left.column, right, join.right.column) ||
        along(right, join.right.column, left, join.left.column)
      ) {
        edges.add(edge.id);
      }
    }
  }

  return { access, columns, values, edges };
}

/** What a step does, as one word for the step bar and the picker. */
export const KIND_LABEL: Record<StepKind, string> = {
  select: "SELECT",
  insert: "INSERT",
  update: "UPDATE",
  delete: "DELETE",
};

/** The access a statement of each kind has on the table it is about. */
export const KIND_ACCESS: Record<StepKind, Access> = {
  select: "read",
  insert: "write",
  update: "write",
  delete: "delete",
};

/**
 * The state class for an access. Spelled out rather than built as
 * `is-${access}`, so `classNames.test.ts` can see each one is used.
 */
export const ACCESS_CLASS: Record<Access, string> = {
  read: "is-read",
  write: "is-write",
  delete: "is-delete",
};

/** The table a step is mainly about: the one it writes, else the first read. */
export function stepTarget(step: Step): string | null {
  const touches = step.touches ?? [];
  const main = touches.find((touch) => touch.access !== "read") ?? touches[0];
  return main ? main.table.name : null;
}
