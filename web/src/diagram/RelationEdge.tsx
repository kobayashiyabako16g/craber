import { BaseEdge, Position, type EdgeProps } from "@xyflow/react";

import type { Relation } from "./graph";
import { MARKER_LENGTH } from "./handles";
import { relationPath } from "./path";

/**
 * A relationship between two tables, drawn so that its notation sits on a
 * straight piece of line.
 *
 * Everything that makes this different from React Flow's `default` edge is in
 * `relationPath`; this is what puts it on the canvas. Registered as the
 * `relation` type in `Diagram.tsx` and asked for by name in `graph.ts`, where
 * a self-reference stays `smoothstep` — both its ends are on the one box, and
 * its orthogonal route already runs straight out of the node.
 */
export function RelationEdge({
  data,
  sourceX,
  sourceY,
  sourcePosition,
  targetX,
  targetY,
  targetPosition,
  pathOptions,
  style,
  markerStart,
  markerEnd,
  interactionWidth,
}: EdgeProps<Relation>) {
  const { curvature } = (pathOptions ?? {}) as { curvature?: number };
  const [path] = relationPath({
    sourceX,
    sourceY,
    sourcePosition,
    targetX,
    targetY,
    targetPosition,
    curvature,
  });
  // The markers are passed straight through, already wrapped in `url(#...)`.
  // They still sit on the point where the line meets the node — which is now
  // the start of its straight run rather than the start of the curve.
  // `n:1` read from the referencing end: `n` belongs at the source, `1` at
  // the target. Only on a lit line — on every line at once, the numbers would
  // be a second, illegible diagram over the first.
  const [sourceCount, targetCount] = data?.lit ? data.cardinality.split(":") : [];
  return (
    <>
      <BaseEdge
        path={path}
        style={style}
        markerStart={markerStart}
        markerEnd={markerEnd}
        interactionWidth={interactionWidth}
      />
      {sourceCount && <EndCount x={sourceX} y={sourceY} side={sourcePosition} text={sourceCount} />}
      {targetCount && <EndCount x={targetX} y={targetY} side={targetPosition} text={targetCount} />}
    </>
  );
}

/**
 * The row count at one end of a line, centred over its crow's foot so it reads
 * with the glyph it qualifies, and lifted clear of it: the glyph is 16 units
 * tall, centred on the line — see `MARKER_BOX`.
 */
function EndCount({ x, y, side, text }: { x: number; y: number; side: Position; text: string }) {
  const outward = side === Position.Left ? -1 : 1;
  return (
    <text
      className="craber-edge-end"
      x={x + (outward * MARKER_LENGTH) / 2}
      y={y - 13}
      textAnchor="middle"
    >
      {text}
    </text>
  );
}
