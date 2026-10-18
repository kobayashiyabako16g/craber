import { BaseEdge, type EdgeProps } from "@xyflow/react";

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
}: EdgeProps) {
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
  return (
    <BaseEdge
      path={path}
      style={style}
      markerStart={markerStart}
      markerEnd={markerEnd}
      interactionWidth={interactionWidth}
    />
  );
}
