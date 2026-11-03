import { Position, getBezierPath } from "@xyflow/react";

import { STUB_LENGTH } from "./handles";

/**
 * The line one relationship is drawn as: a straight run out of each node, then
 * a bezier between the far ends of those two runs.
 *
 * React Flow's own bezier starts bending at the node. Its tangent there is
 * horizontal — which is what lets the notation be drawn square to the face at
 * all, see `MARKER_BOX` — but only for an instant. Put the two tables far
 * apart vertically and the line has turned well off horizontal before it has
 * cleared the glyph's own `MARKER_LENGTH`, so the crow's foot ends up crossed
 * by a line that no longer runs with it: the shape reads as broken rather than
 * as notation.
 *
 * Holding the line straight until the glyph has ended costs the curve nothing.
 * What it hands off to is the same bezier React Flow would have drawn, between
 * two points a stub further in, and the geometry the notation relies on is now
 * true for the whole length the notation occupies instead of for a point.
 */

/** The two ends of a line, as React Flow hands them to a custom edge. */
export type Ends = {
  sourceX: number;
  sourceY: number;
  sourcePosition: Position;
  targetX: number;
  targetY: number;
  targetPosition: Position;
  curvature?: number;
};

/** Where the straight run out of a node ends, given the face it leaves from. */
function stub(x: number, position: Position): number {
  return position === Position.Left ? x - STUB_LENGTH : x + STUB_LENGTH;
}

/** The path, and the point a label on it would sit at. */
export function relationPath({
  sourceX,
  sourceY,
  sourcePosition,
  targetX,
  targetY,
  targetPosition,
  curvature,
}: Ends): [path: string, labelX: number, labelY: number] {
  const sourceStub = stub(sourceX, sourcePosition);
  const targetStub = stub(targetX, targetPosition);
  const [curve, labelX, labelY] = getBezierPath({
    sourceX: sourceStub,
    sourceY,
    sourcePosition,
    targetX: targetStub,
    targetY,
    targetPosition,
    curvature,
  });
  // `getBezierPath` returns `M<start> C<control> <control> <end>`, and its
  // start is where the first straight run already left off — so the move is
  // dropped and only the curve itself is kept.
  const bezier = curve.slice(curve.indexOf("C"));
  return [
    `M${sourceX},${sourceY} L${sourceStub},${sourceY} ${bezier} L${targetX},${targetY}`,
    labelX,
    labelY,
  ];
}
