import type { Edge } from "@xyflow/react";

import { qualifiedName, resolveRef, tableIndex } from "../schema";
import type { TableNodeType } from "../TableNode";
import type { ForeignKey, Table } from "../types";

/**
 * Schema IR → what React Flow draws.
 *
 * No React and no geometry. Which side of a node an edge attaches to depends on
 * where the tables currently are and is decided per render in `anchors.ts`;
 * everything here is a fact about the schema, so it is computed once.
 */

/**
 * The crow's-foot ends CrabER can derive from DDL.
 *
 * `one or many` and a bare `one` are absent because nothing in a schema states
 * them: no dialect can say that a row must have at least one child.
 */
export type EndKind = "zero-many" | "zero-one" | "one-only";

/** An edge, the columns it joins, and how each of its ends is drawn. */
export type Relation = Edge<{
  sourceEnd: EndKind;
  targetEnd: EndKind;
  sourceColumn: string;
  targetColumn: string;
}> & {
  /** Curvature for a bezier, straight-run offset for a self-reference's
   *  orthogonal route — see where `toEdges` sets them. */
  pathOptions?: { curvature: number } | { offset: number };
};

/**
 * How each end of a relationship is drawn, from the two facts the schema
 * carries.
 *
 * Both come from the schema, not from the picture: a unique foreign key means
 * at most one row at the referencing end, and a nullable one means the row it
 * points at may not be there. `craber-render`'s `End::referencing` /
 * `End::referenced` decide the same thing for an exported SVG, and the two have
 * to agree or one schema is two different diagrams.
 */
export function endKinds(fk: ForeignKey): { source: EndKind; target: EndKind } {
  return {
    source: fk.cardinality === "oneToOne" ? "zero-one" : "zero-many",
    target: fk.optional ? "zero-one" : "one-only",
  };
}

/**
 * The same relationship as a label, read from one end of it.
 *
 * `from` is the table that holds the foreign key looking out; `to` is the table
 * it points at looking back.
 */
export function cardinalityLabel(fk: ForeignKey, side: "from" | "to"): string {
  if (fk.cardinality === "oneToOne") return "1:1";
  return side === "from" ? "n:1" : "1:n";
}

/** Nodes at the coordinates `craber-layout` assigned, before any dragging. */
export function bakedNodes(
  tables: Table[],
  anchored: ReadonlyMap<string, ReadonlySet<string>>,
): TableNodeType[] {
  return tables.map((table) => {
    const id = qualifiedName(table);
    // Every column is drawn, so the coordinates are the ones laid out for a
    // full-height node.
    const position = table.positions?.allFields;
    return {
      id,
      type: "table" as const,
      position: { x: position?.x ?? 0, y: position?.y ?? 0 },
      data: { table, anchors: anchored.get(id) ?? EMPTY_ANCHORS },
    };
  });
}

const EMPTY_ANCHORS: ReadonlySet<string> = new Set();

/**
 * The columns each table's relationships attach to: the foreign key's own
 * columns, and the columns it points at in the other table.
 *
 * Only these get handles. A table of forty columns with one foreign key needs
 * two anchors, not eighty.
 */
export function anchorColumns(tables: Table[]): Map<string, ReadonlySet<string>> {
  const byId = tableIndex(tables);
  const anchors = new Map<string, Set<string>>(
    tables.map((table) => [qualifiedName(table), new Set<string>()]),
  );
  const add = (tableId: string, column: string | undefined) => {
    if (!columnOf(byId.get(tableId), column)) return;
    anchors.get(tableId)?.add(column as string);
  };

  for (const table of tables) {
    const id = qualifiedName(table);
    for (const fk of table.fks ?? []) {
      const targetId = resolveRef(fk.refTable, byId);
      add(id, fk.columns[0]);
      if (targetId) add(targetId, referencedColumn(fk.refColumns, byId.get(targetId)));
    }
  }
  return anchors;
}

/** The column's name if the table has it, and the empty string if not. */
function columnOf(table: Table | undefined, column: string | undefined): string {
  return column && table?.columns?.some((c) => c.name === column) ? column : "";
}

/** The column an edge lands on in the referenced table. */
function referencedColumn(refColumns: string[] | undefined, target: Table | undefined) {
  return refColumns?.[0] ?? target?.pk?.[0];
}

/**
 * Schema IR → React Flow edges. No layout happens here, and no geometry: which
 * side of each node an edge attaches to is decided per render from where the
 * tables currently are, by `facingSides`.
 */
export function toEdges(tables: Table[], stubLength: number): Relation[] {
  const byId = tableIndex(tables);
  const edges: Relation[] = [];

  for (const table of tables) {
    for (const [i, fk] of (table.fks ?? []).entries()) {
      const source = qualifiedName(table);
      const targetId = resolveRef(fk.refTable, byId);
      if (!targetId) continue;
      const selfReference = source === targetId;
      const ends = endKinds(fk);
      edges.push({
        id: `${source}:${i}`,
        source,
        target: targetId,
        sourceHandle: null,
        targetHandle: null,
        // Relationships CrabER inferred from naming rather than read from a
        // constraint are drawn dashed, so the two are never confused.
        className: fk.inferred ? "craber-edge-inferred" : "",
        // The referencing end carries the cardinality: many rows here point at
        // one row there, unless the FK is unique. The referenced end is always
        // the "one" side. Which *marker* draws it depends on the side the edge
        // ends up attached to, so it is chosen at render time.
        data: {
          sourceEnd: ends.source,
          targetEnd: ends.target,
          // The rows the relationship actually joins. A line between two
          // tables says less than a line between two columns. Empty when the
          // column is not one the node draws, which falls the edge back to the
          // node's own middle rather than dropping it.
          sourceColumn: columnOf(table, fk.columns[0]),
          targetColumn: columnOf(
            byId.get(targetId),
            referencedColumn(fk.refColumns, byId.get(targetId)),
          ),
        },
        // A bezier, held straight out of each node for the length of the
        // notation first: see `relationPath`, which is what `relation` draws.
        // The crow's foot has to stay square to the node it touches at every
        // arrangement a reader can drag the tables into, and that straight run
        // is what makes it so for the glyph's whole length rather than for the
        // instant the curve leaves the face.
        //
        // A self-reference is the exception, and keeps the orthogonal route.
        // Both of its ends are on the one box, so there is no gap between two
        // tables for a curve to cross: a bezier between them is a shortcut
        // straight through the node, drawn entirely behind it. `anchorEdges`
        // puts both ends on the same side and this brackets them around.
        type: selfReference ? "smoothstep" : "relation",
        // Both of these buy the same thing: a line that still runs with its
        // crow's foot for the glyph's whole length instead of turning inside
        // it. The orthogonal route runs straight out of the node for `offset`
        // before its first corner, which defaults to 20 — just short of the
        // glyph, so it is given the same stub the curves take. And a curvature
        // above the 0.25 default only ever reaches a line that has to double
        // back, because React Flow takes the control offset from `0.5 × gap`
        // and ignores the curvature entirely whenever the two ends have a gap
        // between them, falling back to `curvature × 25 × √-gap` only when
        // they do not.
        pathOptions: selfReference ? { offset: stubLength } : { curvature: 0.45 },
      });
    }
  }
  return edges;
}

/** The selected table plus everything one relationship away, or null. */
export function neighbourhood(
  edges: Edge[],
  selected: string | null,
): ReadonlySet<string> | null {
  if (!selected) return null;
  const lit = new Set([selected]);
  for (const edge of edges) {
    if (edge.source === selected) lit.add(edge.target);
    if (edge.target === selected) lit.add(edge.source);
  }
  return lit;
}
