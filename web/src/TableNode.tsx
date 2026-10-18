import { Handle, Position, type Node, type NodeProps } from "@xyflow/react";
import { Fragment } from "react";

import { ColumnGlyph } from "./ColumnGlyph";
import { handleId } from "./diagram/handles";
import { CommentIcon, TableIcon } from "./icons";
import { foreignKeyColumns, primaryKeyColumns } from "./schema";
import type { Table } from "./types";

export type TableNodeData = {
  table: Table;
  /** Column names an edge attaches to — see `anchorColumns` in `diagram/graph.ts`. */
  anchors: ReadonlySet<string>;
};
export type TableNodeType = Node<TableNodeData, "table">;

/**
 * One table on the canvas, with every column it has.
 *
 * Edges attach to a *column*, not to the node: a relationship is between
 * `orders.user_id` and `users.id`, and the diagram says so only if the line
 * starts and ends on those rows. Each column that an edge can reach therefore
 * carries handles on both of its sides, and `diagram/anchors.ts` picks the side facing the
 * table at the other end. Nothing here decides geometry.
 */
export function TableNode({ data, selected }: NodeProps<TableNodeType>) {
  const { table, anchors } = data;
  const pk = primaryKeyColumns(table);
  const fk = foreignKeyColumns(table);
  const columns = table.columns ?? [];

  return (
    <div className={`craber-node${selected ? " is-selected" : ""}`}>
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
            <div className="craber-node-column" key={column.name}>
              {anchors.has(column.name) && <ColumnHandles column={column.name} />}
              <ColumnGlyph column={column} isPk={pk.has(column.name)} isFk={fk.has(column.name)} />
              <span className="craber-node-column-name">{column.name}</span>
              <Note text={column.comment} />
              <span className="craber-node-column-type">{column.dataType}</span>
            </div>
          ))}
        </div>
      )}
    </div>
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
    <span className="craber-note" title={text}>
      <CommentIcon size={11} />
    </span>
  );
}

