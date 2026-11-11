import type { Edge, Node } from "@xyflow/react";

import { qualifiedName, resolveRef, tableIndex } from "./schema";
import type { Access, Cardinality, ColumnValue, ForeignKey, Table } from "@/types";

/**
 * Schema IR → what React Flow draws.
 *
 * No React and no geometry. Which side of a node an edge attaches to depends on
 * where the tables currently are and is decided per render in `anchors.ts`;
 * everything here is a fact about the schema, so it is computed once.
 *
 * The foreign keys are walked once, by `toEdges`. Everything else that is
 * about relationships — which rows get handles, what the details panel lists —
 * is read off the `Relation`s it returns, so the canvas and the panel cannot
 * disagree about what a key joins.
 */

/** What a table node carries. Defined here, not beside the component, so the
 *  code that builds nodes does not depend on the one that draws them. */
export type TableNodeData = {
  table: Table;
  /** Column names an edge attaches to — see `anchorColumns`. */
  anchors: ReadonlySet<string>;
  /** Rows joined by a relationship of the selected or hovered table. */
  litColumns?: ReadonlySet<string>;
  /** This table is the selected or hovered one, or one relationship away. */
  related?: boolean;
  /** What the workflow step being played does to this table. */
  access?: Access;
  /** Per column, the values that step writes or filters by. */
  values?: ReadonlyMap<string, readonly ColumnValue[]>;
};
export type TableNodeType = Node<TableNodeData, "table">;

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
  cardinality: Cardinality;
  /** Read from naming rather than from a constraint; drawn dashed. */
  inferred: boolean;
  /** Set per render while one of its tables is selected or pointed at. */
  lit?: boolean;
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
 * points at may not be there.
 */
export function endKinds(fk: ForeignKey): { source: EndKind; target: EndKind } {
  return {
    source: fk.cardinality === "oneToOne" ? "zero-one" : "zero-many",
    target: fk.optional ? "zero-one" : "one-only",
  };
}

/**
 * The row count at each end of a relationship: the referencing end first.
 * Many rows point at one, unless the key is unique.
 */
export function endCounts(cardinality: Cardinality): [source: string, target: string] {
  return cardinality === "oneToOne" ? ["1", "1"] : ["n", "1"];
}

/**
 * The same relationship as a label, read from one end of it.
 *
 * `from` is the table that holds the foreign key looking out; `to` is the table
 * it points at looking back.
 */
export function cardinalityLabel(cardinality: Cardinality, side: "from" | "to"): string {
  const [source, target] = endCounts(cardinality);
  return side === "from" ? `${source}:${target}` : `${target}:${source}`;
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
 * two anchors, not eighty. A relationship whose column the node does not draw
 * attaches to the node's middle instead, so it adds nothing here.
 */
export function anchorColumns(
  tables: Table[],
  relations: Relation[],
): Map<string, ReadonlySet<string>> {
  const anchors = new Map<string, Set<string>>(
    tables.map((table) => [qualifiedName(table), new Set<string>()]),
  );
  for (const { source, target, data } of relations) {
    if (data?.sourceColumn) anchors.get(source)?.add(data.sourceColumn);
    if (data?.targetColumn) anchors.get(target)?.add(data.targetColumn);
  }
  return anchors;
}

/** The column's name if the table has it, and the empty string if not. */
function columnOf(table: Table | undefined, column: string | undefined): string {
  return column && table?.columns?.some((c) => c.name === column) ? column : "";
}

/**
 * The column an edge lands on in the referenced table: the one the key names,
 * or the primary key's when it names none. `craber-core` fills that in, so the
 * fallback is for a `schema.json` written before it did.
 */
function referencedColumn(refColumns: string[] | undefined, target: Table | undefined) {
  return refColumns?.[0] ?? target?.pk?.[0];
}

/**
 * Schema IR → React Flow edges, one per foreign key whose target resolves.
 *
 * An edge's id is `<table>:<index of the key>`, which `workflow.ts` and the
 * details panel rely on to refer back to it.
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
          cardinality: fk.cardinality,
          inferred: fk.inferred,
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

/** What lighting up some tables lights with them — see `litRelations`. */
export type Lit = {
  /** Every relationship that touches one of the tables. */
  edges: ReadonlySet<string>;
  /** The tables themselves and everything one relationship away. */
  tables: ReadonlySet<string>;
  /** Per table, the rows those relationships join. */
  columns: ReadonlyMap<string, ReadonlySet<string>>;
};

/**
 * The relationships of the given tables, and what they reach.
 *
 * Takes the selected and the hovered table together, since both light the
 * diagram the same way and either may be absent. A column the node does not
 * draw (an empty `sourceColumn`) lights nothing, as there is no row to light.
 */
export function litRelations(edges: Relation[], tables: (string | null)[]): Lit {
  const active = new Set(tables.filter((table): table is string => table !== null));
  const lit = { edges: new Set<string>(), tables: new Set(active), columns: new Map() };
  if (active.size === 0) return lit;

  const light = (table: string, column: string | undefined) => {
    if (!column) return;
    const columns = lit.columns.get(table) ?? new Set<string>();
    lit.columns.set(table, columns.add(column));
  };
  for (const edge of edges) {
    if (!active.has(edge.source) && !active.has(edge.target)) continue;
    lit.edges.add(edge.id);
    lit.tables.add(edge.source).add(edge.target);
    light(edge.source, edge.data?.sourceColumn);
    light(edge.target, edge.data?.targetColumn);
  }
  return lit;
}

/** One relationship, seen from a table the details panel is describing. */
export type Relationship = {
  /** The edge it is drawn as, which also keys it. */
  id: string;
  /** The table at the other end. */
  other: string;
  /** The referencing column, `orders.user_id`, whichever end this table is. */
  from: string;
  /** The column it points at, `users.id`. */
  to: string;
  /** `n:1` or `1:1`, written from this table's side. */
  cardinality: string;
  inferred: boolean;
};

/**
 * Both directions: the foreign keys `table` declares, and the ones other
 * tables point back at it with. A reader asking about `users` wants to know
 * what refers to it at least as much as what it refers to.
 *
 * Read off the same relations the canvas draws, so the panel names the rows
 * the line actually joins.
 */
export function relationshipsOf(table: string, relations: Relation[]): Relationship[] {
  const end = (id: string, column: string | undefined) => `${id}.${column || "?"}`;
  const outgoing = relations
    .filter((edge) => edge.source === table)
    .map((edge) => ({
      id: edge.id,
      other: edge.target,
      from: end(edge.source, edge.data?.sourceColumn),
      to: end(edge.target, edge.data?.targetColumn),
      cardinality: cardinalityLabel(edge.data?.cardinality ?? "manyToOne", "from"),
      inferred: edge.data?.inferred ?? false,
    }));
  // A self-reference is listed once, as the key this table declares.
  const incoming = relations
    .filter((edge) => edge.target === table && edge.source !== table)
    .map((edge) => ({
      id: edge.id,
      other: edge.source,
      from: end(edge.source, edge.data?.sourceColumn),
      to: end(edge.target, edge.data?.targetColumn),
      // Read from this table's side, so the "one" end is this one.
      cardinality: cardinalityLabel(edge.data?.cardinality ?? "manyToOne", "to"),
      inferred: edge.data?.inferred ?? false,
    }));
  return [...outgoing, ...incoming];
}
