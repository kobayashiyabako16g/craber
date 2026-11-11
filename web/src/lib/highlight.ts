import type { Edge } from "@xyflow/react";

import type { Anchor } from "./anchors";
import { cx } from "./cx";
import { markerId, type Sides } from "./edgeGeometry";
import type { Lit, Relation, TableNodeType } from "./graph";
import type { StepLit } from "./workflow";

/**
 * What the canvas lights, and what it dims, for one render.
 *
 * Three things want to light the diagram — a selected table, a hovered one,
 * and a workflow step being played back — and the rules for combining them are
 * the part of the canvas most likely to break. They live here, React-free,
 * rather than inside `Diagram.tsx`, so they can be tested.
 */

/**
 * Either a workflow step is playing, which takes the canvas over, or the
 * selection and the hover are.
 */
export type Highlight =
  | { kind: "playback"; step: StepLit }
  | {
      kind: "focus";
      selected: string | null;
      /** The selected table and its neighbours; everything else dims. Null
       *  when nothing is selected. */
      focus: ReadonlySet<string> | null;
      /** What the selected and the hovered table light. */
      lit: Lit;
    };

/**
 * The nodes as drawn under `highlight`.
 *
 * Playback lights the step's tables in the colour of what it does to them, and
 * dims the rest. Otherwise a selection dims everything outside its
 * neighbourhood, and what the pointer lights stays readable inside it. `hidden`
 * wins over both. Only a node whose highlight changed gets new data, so a hover
 * does not re-render every node on the canvas.
 */
export function decorateNodes(
  nodes: TableNodeType[],
  highlight: Highlight,
  hidden: ReadonlySet<string>,
): TableNodeType[] {
  return nodes.map((node) => {
    if (highlight.kind === "playback") {
      const { step } = highlight;
      const access = step.access.get(node.id);
      return {
        ...node,
        data: access
          ? {
              ...node.data,
              litColumns: step.columns.get(node.id),
              values: step.values.get(node.id),
              access,
            }
          : node.data,
        hidden: hidden.has(node.id),
        selected: false,
        className: access ? undefined : "craber-canvas-dimmed",
      };
    }
    const { selected, focus, lit } = highlight;
    const columns = lit.columns.get(node.id);
    const related = lit.tables.has(node.id);
    return {
      ...node,
      data: columns || related ? { ...node.data, litColumns: columns, related } : node.data,
      hidden: hidden.has(node.id),
      selected: node.id === selected,
      className: focus && !focus.has(node.id) && !related ? "craber-canvas-dimmed" : undefined,
    };
  });
}

/** Before React Flow has measured the nodes there are no sides to face, and
 *  a line runs left to right. */
const UNPLACED: Sides = { source: "right", target: "left" };

/**
 * The edges as drawn under `highlight`, attached by `anchors`.
 *
 * A lit line is coloured and raised above the bundle it runs in; while
 * anything is selected or playing, every other line dims. A line to a hidden
 * table is hidden with it.
 */
export function decorateEdges(
  relations: Relation[],
  anchors: ReadonlyMap<string, Anchor>,
  highlight: Highlight,
  hidden: ReadonlySet<string>,
): Edge[] {
  return relations.map((edge) => {
    const anchor = anchors.get(edge.id);
    const sides = anchor?.sides ?? UNPLACED;
    const isLit =
      highlight.kind === "playback"
        ? highlight.step.edges.has(edge.id)
        : highlight.lit.edges.has(edge.id);
    const dimmed = highlight.kind === "playback" ? !isLit : highlight.focus !== null;
    return {
      ...edge,
      data: isLit && edge.data ? { ...edge.data, lit: true } : edge.data,
      // Above the rest of the bundle it runs in, or the lines drawn after it
      // cover the ones being asked about.
      zIndex: isLit ? 1 : undefined,
      sourceHandle: anchor?.source,
      targetHandle: anchor?.target,
      // React Flow wraps these in `url(#...)` itself, so they are bare ids. The
      // face the end attached to picks the marker, because that is what the
      // glyph is drawn square to — see `MARKER_BOX`.
      markerStart: edge.data && markerId(edge.data.sourceEnd, sides.source),
      markerEnd: edge.data && markerId(edge.data.targetEnd, sides.target),
      hidden: hidden.has(edge.source) || hidden.has(edge.target),
      // Lit, not merely left alone: dimming the rest says which lines do not
      // matter, and colouring these says which ones do. The markers follow
      // without being told — they stroke in `context-stroke`.
      className: cx(
        edge.className,
        isLit ? "craber-edge-focus" : dimmed ? "craber-canvas-dimmed" : undefined,
      ),
    };
  });
}
