import { useNodesState } from "@xyflow/react";
import { useCallback, useEffect, useMemo } from "react";

import { anchorEdges, type Box } from "@/lib/anchors";
import { STUB_LENGTH } from "@/lib/edgeGeometry";
import { anchorColumns, bakedNodes, toEdges, type TableNodeType } from "@/lib/graph";
import { tableIndex } from "@/lib/schema";
import type { Table } from "@/types";

/**
 * The schema as a graph on the canvas: the tables by id, the relationships,
 * the nodes where they currently sit, and which handle each line uses.
 *
 * Everything but the node positions is a fact about the schema and computed
 * once; the positions live in React Flow's state so a table can be nudged out
 * of the way, and the anchors follow them. Nothing here computes a layout —
 * the coordinates always come from `schema.json`.
 */
export function useDiagramGraph(tables: Table[]) {
  const known = useMemo(() => tableIndex(tables), [tables]);
  const relations = useMemo(() => toEdges(tables, STUB_LENGTH), [tables]);
  // Which columns an edge can land on. Fixed by the schema, so the nodes can
  // carry it and only the anchored rows pay for handles.
  const anchored = useMemo(() => anchorColumns(tables, relations), [tables, relations]);

  const [nodes, setNodes, onNodesChange] = useNodesState<TableNodeType>(
    useMemo(() => bakedNodes(tables, anchored), [tables, anchored]),
  );
  // `Tidy up`, and what happens when the schema itself changes: both mean
  // "put every table back where craber-layout placed it".
  const tidyUp = useCallback(() => {
    setNodes(bakedNodes(tables, anchored));
  }, [setNodes, tables, anchored]);
  useEffect(() => {
    tidyUp();
  }, [tidyUp]);

  // Where every node sits at this moment. Dragging changes `nodes`, so the
  // edges re-pick their sides as the table moves.
  const anchors = useMemo(() => {
    const boxes = new Map<string, Box>();
    for (const node of nodes) {
      boxes.set(node.id, {
        x: node.position.x,
        y: node.position.y,
        // Before React Flow has measured a node, its own coordinates are all
        // there is; the sides come out right as soon as it has.
        width: node.measured?.width ?? 0,
      });
    }
    return anchorEdges(relations, boxes);
  }, [relations, nodes]);

  return { known, relations, nodes, onNodesChange, tidyUp, anchors };
}
