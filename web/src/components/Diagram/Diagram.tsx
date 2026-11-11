import { Background, MiniMap, ReactFlow, useReactFlow, useViewport } from "@xyflow/react";
import { useCallback, useMemo, useState } from "react";

import { CardinalityMarkers } from "@/components/Markers/Markers";
import { RelationEdge } from "@/components/RelationEdge/RelationEdge";
import { Sidebar } from "@/components/Sidebar/Sidebar";
import { StepBar } from "@/components/StepBar/StepBar";
import { TableDetails } from "@/components/TableDetails/TableDetails";
import { TableNode } from "@/components/TableNode/TableNode";
import { Toolbar } from "@/components/Toolbar/Toolbar";
import { TopBar } from "@/components/TopBar/TopBar";
import { WorkflowPicker } from "@/components/WorkflowPicker/WorkflowPicker";
import { useDiagramGraph } from "@/hooks/useDiagramGraph";
import { usePlayback } from "@/hooks/usePlayback";
import { useViewSync } from "@/hooks/useViewSync";
import { download } from "@/lib/download";
import { litRelations, neighbourhood, type TableNodeType } from "@/lib/graph";
import { decorateEdges, decorateNodes, type Highlight } from "@/lib/highlight";
import { schemaSql, sqlFileName } from "@/lib/sql";
import { initialFit, type ViewState } from "@/lib/viewState";
import type { Schema, Workflow } from "@/types";

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
 * the arrangement; the graph is `useDiagramGraph`'s, playback is
 * `usePlayback`'s, and what lights and dims is `lib/highlight.ts`'s.
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
  const reactFlow = useReactFlow<TableNodeType>();
  const { zoom } = useViewport();
  const [sidebarOpen, setSidebarOpen] = useState(true);
  const [pickerOpen, setPickerOpen] = useState(false);

  const { known, relations, nodes, onNodesChange, tidyUp, anchors } = useDiagramGraph(tables);
  const { view, dispatch, focusTable } = useViewSync(initial);
  const { selected, hidden } = view;
  const select = useCallback((table: string | null) => dispatch({ type: "select", table }), [dispatch]);
  const selectedTable = selected ? (known.get(selected) ?? null) : null;

  // The table under the pointer lights up the way a selected one does, without
  // a click: its relationships, the tables at their other ends, and the rows
  // they join. Lines are not hovered themselves — in a bundle of twenty there
  // is no telling which one the pointer is on, while the table it starts from
  // is a target nobody misses.
  const [hoveredTable, setHoveredTable] = useState<string | null>(null);

  const { playing, step, played } = usePlayback(
    workflows,
    view.workflow,
    view.step,
    relations,
    known,
    hidden,
  );

  // A workflow being played back takes the canvas over from selection and
  // hover. Otherwise a selected table's neighbourhood stays lit and everything
  // else dims, so its relationships are readable without hiding the rest.
  const highlight = useMemo<Highlight>(
    () =>
      played
        ? { kind: "playback", step: played }
        : {
            kind: "focus",
            selected,
            focus: neighbourhood(relations, selected),
            lit: litRelations(relations, [selected, hoveredTable]),
          },
    [played, relations, selected, hoveredTable],
  );
  const renderedNodes = useMemo(
    () => decorateNodes(nodes, highlight, hidden),
    [nodes, highlight, hidden],
  );
  const renderedEdges = useMemo(
    () => decorateEdges(relations, anchors, highlight, hidden),
    [relations, anchors, highlight, hidden],
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
      dispatch({ type: "recordViewport", viewport: reactFlow.getViewport() });
    },
    [reactFlow, dispatch],
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
          dispatch({ type: "play", workflow: name });
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
            onToggleHidden={(table) => dispatch({ type: "toggleHidden", table })}
            onSetAllHidden={(hide, tables) => dispatch({ type: "setHidden", tables, hidden: hide })}
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
            fitViewOptions={initialFit(initial)}
            defaultViewport={initial.viewport ?? undefined}
            proOptions={{ hideAttribution: false }}
            onNodeClick={(_, node) => select(node.id)}
            onPaneClick={() => select(null)}
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
              if (event) dispatch({ type: "recordViewport", viewport });
            }}
          >
            <Background color="var(--craber-grid)" gap={20} size={1.5} />
            <MiniMap pannable zoomable />
          </ReactFlow>
          <Toolbar
            zoom={zoom}
            onZoomIn={() => void moveCanvas(() => reactFlow.zoomIn({ duration: 150 }))}
            onZoomOut={() => void moveCanvas(() => reactFlow.zoomOut({ duration: 150 }))}
            onFitView={() => void moveCanvas(() => reactFlow.fitView({ duration: 300 }))}
            onTidyUp={tidyUp}
          />
          {playing && (
            <StepBar
              workflow={playing}
              step={step}
              onStep={(next) => dispatch({ type: "step", step: next })}
              onStop={() => dispatch({ type: "play", workflow: null })}
            />
          )}
        </div>
        {selectedTable && (
          <TableDetails
            table={selectedTable}
            relations={relations}
            known={known}
            onSelect={focusTable}
            onClose={() => select(null)}
          />
        )}
      </div>
    </div>
  );
}
