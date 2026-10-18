import {
  Background,
  MiniMap,
  ReactFlow,
  useNodesState,
  useReactFlow,
  useViewport,
  type Edge,
} from "@xyflow/react";
import { useCallback, useEffect, useMemo, useState } from "react";

import { Sidebar } from "./Sidebar";
import { TableDetails } from "./TableDetails";
import { TableNode, type TableNodeType } from "./TableNode";
import { Toolbar } from "./Toolbar";
import { TopBar } from "./TopBar";
import { STUB_LENGTH } from "./diagram/handles";
import { CardinalityMarkers } from "./diagram/Markers";
import { RelationEdge } from "./diagram/RelationEdge";
import { anchorEdges, type Box } from "./diagram/anchors";
import { anchorColumns, bakedNodes, neighbourhood, toEdges } from "./diagram/graph";
import { download } from "./download";
import { initialFit, useViewSync } from "./hooks/useViewSync";
import { qualifiedName } from "./schema";
import { schemaSql, sqlFileName } from "./sql";
import type { Schema } from "./types";
import type { ViewState } from "./viewState";

const nodeTypes = { table: TableNode };
// `relation` is the default edge held straight out of each node for the length
// of its notation; a self-reference keeps React Flow's own `smoothstep`.
const edgeTypes = { relation: RelationEdge };

/**
 * The canvas and everything around it.
 *
 * Separate from `App` because `useReactFlow` and `useViewport` only work inside
 * a `ReactFlowProvider`, which `App` is what supplies. What this file owns is
 * the arrangement: the schema-derived graph, the live geometry, and how
 * selection dims the parts of the diagram that are not being asked about.
 */
export function Diagram({ schema, initial }: { schema: Schema; initial: ViewState }) {
  const tables = useMemo(() => schema.tables ?? [], [schema]);
  const flow = useReactFlow<TableNodeType>();
  const { zoom } = useViewport();
  const [sidebarOpen, setSidebarOpen] = useState(true);

  // Which columns an edge can land on. Fixed by the schema, so the nodes can
  // carry it and only the anchored rows pay for handles.
  const anchored = useMemo(() => anchorColumns(tables), [tables]);

  // Positions live in React Flow's state so a table can be nudged out of the
  // way; `Tidy up` puts them back. Nothing here computes a layout — the
  // coordinates always come from schema.json.
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

  const { selected, setSelected, hidden, focusTable, toggleHidden, setAllHidden, recordViewport } =
    useViewSync(initial);

  const relations = useMemo(() => toEdges(tables, STUB_LENGTH), [tables]);
  const selectedTable = useMemo(
    () => tables.find((table) => qualifiedName(table) === selected) ?? null,
    [tables, selected],
  );
  // A selected table's neighbourhood stays lit and everything else dims, so a
  // table's relationships are readable without hiding the rest of the diagram.
  const focus = useMemo(() => neighbourhood(relations, selected), [relations, selected]);

  const renderedNodes = useMemo(
    () =>
      nodes.map((node) => ({
        ...node,
        hidden: hidden.has(node.id),
        selected: node.id === selected,
        className: focus && !focus.has(node.id) ? "craber-dimmed" : undefined,
      })),
    [nodes, hidden, selected, focus],
  );

  // Where every node sits at this moment. Dragging changes `nodes`, so the
  // edges below re-pick their sides as the table moves.
  const boxes = useMemo(() => {
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
    return boxes;
  }, [nodes]);

  const anchors = useMemo(() => anchorEdges(relations, boxes), [relations, boxes]);

  const renderedEdges = useMemo<Edge[]>(
    () =>
      relations.map((edge) => {
        const anchor = anchors.get(edge.id);
        return {
          ...edge,
          sourceHandle: anchor?.source,
          targetHandle: anchor?.target,
          // React Flow wraps these in `url(#...)` itself, so they are bare ids.
          // The face the end attached to picks the marker, because that is what
          // the glyph is drawn square to — see `MARKER_BOX`.
          markerStart: `craber-${edge.data?.sourceEnd ?? "zero-many"}-${anchor?.sides.source ?? "right"}`,
          markerEnd: `craber-${edge.data?.targetEnd ?? "one-only"}-${anchor?.sides.target ?? "left"}`,
          hidden: hidden.has(edge.source) || hidden.has(edge.target),
          className: [
            edge.className,
            // Lit, not merely left alone: dimming the rest says which lines do
            // not matter, and colouring these says which ones do. The markers
            // follow without being told — they stroke in `context-stroke`.
            focus
              ? focus.has(edge.source) && focus.has(edge.target)
                ? "craber-edge-focus"
                : "craber-dimmed"
              : undefined,
          ]
            .filter(Boolean)
            .join(" "),
        };
      }),
    [relations, anchors, hidden, focus],
  );

  // The toolbar moves the canvas programmatically, so `onMoveEnd` reports it
  // with a null event and the handler below will not have recorded it. Zooming
  // with a button is as much "put the canvas here" as zooming with the wheel,
  // so the URL has to hear about it either way.
  //
  // Not used for `focusTable`: selecting a table also moves the canvas, but
  // `?table=` already says what the reader meant, and it reframes correctly on
  // a window of a different size where fixed coordinates would not.
  const moveCanvas = useCallback(
    async (move: () => Promise<boolean>) => {
      await move();
      recordViewport(flow.getViewport());
    },
    [flow, recordViewport],
  );

  // The whole schema, not what happens to be on screen: a hidden table is
  // hidden from the drawing, and the reader asked for the definitions.
  const exportSql = useCallback(() => {
    download(sqlFileName(schema), schemaSql(schema));
  }, [schema]);

  return (
    <div className="craber-app">
      <TopBar
        schemaName={schema.name}
        sidebarOpen={sidebarOpen}
        onToggleSidebar={() => setSidebarOpen((open) => !open)}
        onExportSql={exportSql}
      />
      <div className="craber-body">
        {sidebarOpen && (
          <Sidebar
            tables={tables}
            selected={selected}
            hidden={hidden}
            onSelect={focusTable}
            onToggleHidden={toggleHidden}
            onSetAllHidden={setAllHidden}
          />
        )}
        <div className="craber-canvas">
          <CardinalityMarkers />
          <ReactFlow
            nodes={renderedNodes}
            edges={renderedEdges}
            onNodesChange={onNodesChange}
            nodeTypes={nodeTypes}
            edgeTypes={edgeTypes}
            colorMode="dark"
            minZoom={0.05}
            fitView={!initial.viewport}
            fitViewOptions={initialFit(initial, tables)}
            defaultViewport={initial.viewport ?? undefined}
            proOptions={{ hideAttribution: false }}
            onNodeClick={(_, node) => setSelected(node.id)}
            onPaneClick={() => setSelected(null)}
            // The only writer of `?at=`. React Flow reports the viewport when a
            // pan or zoom settles rather than per frame, which is the rate the
            // address bar wants to be rewritten at.
            //
            // `event` is null when the move was not a gesture — React Flow's
            // own opening fit, and the toolbar's animated zooms. Recording the
            // opening fit would put coordinates in every link the moment it
            // loaded, and coordinates framed for one window are the wrong
            // region in another; a link nobody has panned should still arrive
            // fitted. So the URL gains `?at=` when, and only when, the reader
            // has put the canvas somewhere themselves.
            onMoveEnd={(event, viewport) => {
              if (event) recordViewport(viewport);
            }}
          >
            <Background color="var(--craber-grid)" gap={20} size={1.5} />
            <MiniMap pannable zoomable />
          </ReactFlow>
          <Toolbar
            zoom={zoom}
            onZoomIn={() => void moveCanvas(() => flow.zoomIn({ duration: 150 }))}
            onZoomOut={() => void moveCanvas(() => flow.zoomOut({ duration: 150 }))}
            onFitView={() => void moveCanvas(() => flow.fitView({ duration: 300 }))}
            onTidyUp={tidyUp}
          />
        </div>
        {selectedTable && (
          <TableDetails
            table={selectedTable}
            tables={tables}
            onSelect={focusTable}
            onClose={() => setSelected(null)}
          />
        )}
      </div>
    </div>
  );
}
