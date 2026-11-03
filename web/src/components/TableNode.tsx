import { Handle, Position, type Node, type NodeProps } from "@xyflow/react";
import { Fragment } from "react";

import { ColumnGlyph } from "./ColumnGlyph";
import { ACCESS_CLASS, valueLabel } from "../lib/flow";
import { handleId } from "../lib/handles";
import { CommentIcon, TableIcon } from "./icons";
import { foreignKeyColumns, primaryKeyColumns } from "../lib/schema";
import type { Access, ColumnValue, Table } from "../types";

import "./TableNode.css";

export type TableNodeData = {
  table: Table;
  /** Column names an edge attaches to — see `anchorColumns` in `lib/graph.ts`. */
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
 * One table on the canvas, with every column it has.
 *
 * Edges attach to a *column*, not to the node: a relationship is between
 * `orders.user_id` and `users.id`, and the diagram says so only if the line
 * starts and ends on those rows. Each column that an edge can reach therefore
 * carries handles on both of its sides, and `lib/anchors.ts` picks the side facing the
 * table at the other end. Nothing here decides geometry.
 */
export function TableNode({ data, selected }: NodeProps<TableNodeType>) {
  const { table, anchors, litColumns, related, access, values } = data;
  const pk = primaryKeyColumns(table);
  const fk = foreignKeyColumns(table);
  const columns = table.columns ?? [];

  return (
    <div
      className={[
        "craber-node",
        selected && "is-selected",
        related && "is-related",
        access && ACCESS_CLASS[access],
      ]
        .filter(Boolean)
        .join(" ")}
    >
      {/* The node's own middle, for a relationship whose column CrabER could
          not place — a foreign key on a column the parser did not read. */}
      <ColumnHandles column="" />

      <div className="craber-node-header">
        <TableIcon size={13} className="craber-node-glyph" />
        {table.schema && <span className="craber-node-schema">{table.schema}.</span>}
        <span className="craber-node-name">{table.name}</span>
        <Note text={table.comment} />
      </div>

      {columns.length > 0 && (
        <div className="craber-node-columns">
          {columns.map((column) => (
            <div
              className={`craber-node-column${litColumns?.has(column.name) ? " is-lit" : ""}`}
              key={column.name}
            >
              {anchors.has(column.name) && <ColumnHandles column={column.name} />}
              <ColumnGlyph column={column} isPk={pk.has(column.name)} isFk={fk.has(column.name)} />
              <span className="craber-node-column-name">{column.name}</span>
              <Note text={column.comment} />
              <ColumnEnd type={column.dataType} values={values?.get(column.name)} />
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

/**
 * The right end of a row: its type, or — while a workflow step names a value
 * for it — that value, with the type kept in the tooltip.
 *
 * In place of the type rather than beside it: the node is as wide as the layout
 * made room for, and a value and a type do not both fit in it.
 */
function ColumnEnd({ type, values }: { type: string; values?: readonly ColumnValue[] }) {
  const first = values?.[0];
  if (!values || !first) return <span className="craber-node-column-type">{type}</span>;
  const label = values.map(valueLabel).join("  ");
  return (
    <span
      className={`craber-node-column-value ${first.kind === "set" ? "is-set" : "is-filter"}`}
      title={`${type}\n${values.map(valueLabel).join("\n")}`}
    >
      {label}
    </span>
  );
}

/**
 * Both sides of one column row, as a source and as a target.
 *
 * Four handles per anchored column looks like a lot, and would be if every
 * column had them — only the ones a foreign key names do.
 */
function ColumnHandles({ column }: { column: string }) {
  return (
    <>
      {(["left", "right"] as const).map((side) => (
        <Fragment key={side}>
          <Handle
            type="source"
            id={handleId("s", side, column)}
            position={side === "left" ? Position.Left : Position.Right}
          />
          <Handle
            type="target"
            id={handleId("t", side, column)}
            position={side === "left" ? Position.Left : Position.Right}
          />
        </Fragment>
      ))}
    </>
  );
}

/**
 * A marker for something that has a comment, with the comment as its tooltip.
 *
 * The comment itself belongs in the details panel — prose in a node would make
 * every table as tall as its documentation. But a reader has to be able to see
 * that there *is* something to read, from the canvas.
 */
function Note({ text }: { text?: string | null }) {
  if (!text) return null;
  return (
    <span className="craber-node-note" title={text}>
      <CommentIcon size={11} />
    </span>
  );
}

