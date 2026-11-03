import { resolveRef } from "../schema";
import type { Access, Step, StepKind, Table } from "../types";
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
  /** Relationships the step's `ON a = b` joins run along. */
  edges: ReadonlySet<string>;
};

const STRENGTH: Record<Access, number> = { read: 0, write: 1, delete: 2 };

export function stepLit(step: Step, relations: Relation[], known: ReadonlyMap<string, Table>): StepLit {
  const access = new Map<string, Access>();
  const columns = new Map<string, Set<string>>();
  for (const touch of step.touches ?? []) {
    const id = resolveRef(touch.table, known);
    if (!id) continue;
    const current = access.get(id);
    if (!current || STRENGTH[touch.access] > STRENGTH[current]) access.set(id, touch.access);
    const names = columns.get(id) ?? new Set<string>();
    for (const column of touch.columns ?? []) names.add(column);
    columns.set(id, names);
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

  return { access, columns, edges };
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
