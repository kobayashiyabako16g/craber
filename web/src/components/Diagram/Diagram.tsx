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

import { Sidebar } from "@/components/Sidebar/Sidebar";
import { StepBar } from "@/components/StepBar/StepBar";
import { TableDetails } from "@/components/TableDetails/TableDetails";
import { TableNode, type TableNodeType } from "@/components/TableNode/TableNode";
import { Toolbar } from "@/components/Toolbar/Toolbar";
import { TopBar } from "@/components/TopBar/TopBar";
import { WorkflowPicker } from "@/components/WorkflowPicker/WorkflowPicker";
import { STUB_LENGTH } from "@/lib/handles";
import { CardinalityMarkers } from "@/components/Markers/Markers";
import { RelationEdge } from "@/components/RelationEdge/RelationEdge";
import { anchorEdges, type Box } from "@/lib/anchors";
import { stepLit } from "@/lib/flow";
import {
  anchorColumns,
  bakedNodes,
  litRelations,
  neighbourhood,
  toEdges,
} from "@/lib/graph";
import { download } from "@/lib/download";
import { initialFit, useViewSync } from "@/hooks/useViewSync";
import { qualifiedName, tableIndex } from "@/lib/schema";
import { schemaSql, sqlFileName } from "@/lib/sql";
import type { Schema, Workflow } from "@/types";
import type { ViewState } from "@/lib/viewState";

import "./Diagram.css";

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
export function Diagram({
  schema,
  workflows,
  initial,
}: {
  schema: Schema;
  workflows: Workflow[];
  initial: ViewState;
}) {
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

  const {
    selected,
    setSelected,
    hidden,
    focusTable,
    toggleHidden,
    setAllHidden,
    recordViewport,
    playing: playingName,
    playFlow,
    step,
    setStep,
  } = useViewSync(initial);
  const [pickerOpen, setPickerOpen] = useState(false);

  const relations = useMemo(() => toEdges(tables, STUB_LENGTH), [tables]);
  const selectedTable = useMemo(
    () => tables.find((table) => qualifiedName(table) === selected) ?? null,
    [tables, selected],
  );
  // A selected table's neighbourhood stays lit and everything else dims, so a
  // table's relationships are readable without hiding the rest of the diagram.
  const focus = useMemo(() => neighbourhood(relations, selected), [relations, selected]);

  // The table under the pointer lights up the same way, without a click: its
  // relationships, the tables at their other ends, and the rows they join.
  // Lines are not hovered themselves — in a bundle of twenty there is no
  // telling which one the pointer is on, while the table it starts from is
  // a target nobody misses.
  const [hoveredTable, setHoveredTable] = useState<string | null>(null);
  const lit = useMemo(
    () => litRelations(relations, [selected, hoveredTable]),
    [relations, selected, hoveredTable],
  );

  // A workflow being played back takes the canvas over from selection and
  // hover: the step's tables light in the colour of what it does to them, the
  // relationships its joins run along light with them, and the rest dims.
  // Until `flows.json` has loaded, a `?flow=` from the URL matches nothing and
  // the diagram draws as usual.
  const playing = useMemo(
    () => workflows.find((workflow) => workflow.name === playingName) ?? null,
    [workflows, playingName],
  );
  const known = useMemo(() => tableIndex(tables), [tables]);
  const played = useMemo(() => {
    const current = playing?.steps[Math.min(step, playing.steps.length - 1)];
    return current ? stepLit(current, relations, known) : null;
  }, [playing, step, relations, known]);

  // Each step brings what it touches into view, so a workflow that wanders
  // across a large diagram can be followed without panning after it.
  useEffect(() => {
    if (!played) return;
    const ids = [...played.access.keys()].filter((id) => !hidden.has(id));
    if (ids.length === 0) return;
    void flow.fitView({
      nodes: ids.map((id) => ({ id })),
      duration: 400,
      maxZoom: 1.2,
      padding: 0.3,
    });
    // `hidden` is left out on purpose: hiding a table mid-playback should not
    // move the canvas.
  }, [played, flow]);

  const renderedNodes = useMemo(
    () =>
      nodes.map((node) => {
        if (played) {
          const access = played.access.get(node.id);
          return {
            ...node,
            data: access
              ? {
                  ...node.data,
                  litColumns: played.columns.get(node.id),
                  values: played.values.get(node.id),
                  access,
                }
              : node.data,
            hidden: hidden.has(node.id),
            selected: false,
            className: access ? undefined : "craber-canvas-dimmed",
          };
        }
        const columns = lit.columns.get(node.id);
        const related = lit.tables.has(node.id);
        return {
          ...node,
          // Only the tables that are lit get new data, so a hover does not
          // re-render every node on the canvas.
          data:
            columns || related ? { ...node.data, litColumns: columns, related } : node.data,
          hidden: hidden.has(node.id),
          selected: node.id === selected,
          // What the pointer lights stays readable inside a selection too.
          className: focus && !focus.has(node.id) && !related ? "craber-canvas-dimmed" : undefined,
        };
      }),
    [nodes, hidden, selected, focus, lit, played],
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
        const isLit = played ? played.edges.has(edge.id) : lit.edges.has(edge.id);
        const dimmed = played ? !isLit : Boolean(focus);
        return {
          ...edge,
          data: isLit && edge.data ? { ...edge.data, lit: true } : edge.data,
          // Above the rest of the bundle it runs in, or the lines drawn after
          // it cover the ones being asked about.
          zIndex: isLit ? 1 : undefined,
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
            isLit ? "craber-edge-focus" : dimmed ? "craber-canvas-dimmed" : undefined,
          ]
            .filter(Boolean)
            .join(" "),
        };
      }),
    [relations, anchors, hidden, focus, lit, played],
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
        workflowCount={workflows.length}
        onOpenWorkflows={() => setPickerOpen(true)}
      />
      <WorkflowPicker
        open={pickerOpen}
        workflows={workflows}
        current={playing?.name ?? null}
        onPick={(name) => {
          playFlow(name);
          setPickerOpen(false);
        }}
        onClose={() => setPickerOpen(false)}
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
            onNodeMouseEnter={(_, node) => setHoveredTable(node.id)}
            onNodeMouseLeave={() => setHoveredTable(null)}
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
          {playing && (
            <StepBar
              workflow={playing}
              step={Math.min(step, Math.max(playing.steps.length - 1, 0))}
              onStep={setStep}
              onStop={() => playFlow(null)}
            />
          )}
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
