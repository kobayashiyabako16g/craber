import { STUB_LENGTH, handleId, type Sides } from "./handles";
import type { Relation } from "./graph";

/**
 * Which handle each edge uses at each end.
 *
 * Pure geometry, recomputed as tables move. The *row* an edge lands on is fixed
 * by the relationship — `orders.user_id` to `users.id` — so only the side is
 * decided here.
 */

/**
 * Where a node sits on the canvas right now, dragging included.
 *
 * Only the horizontal extent: an anchor is always on a row's left or right
 * edge, so nothing here ever has to know how tall a table is.
 */
export type Box = { x: number; y: number; width: number };

/** The handles an edge uses, and the faces they sit on. */
export type Anchor = { source: string; target: string; sides: Sides };

/**
 * The sides of two nodes an edge leaves and arrives on.
 *
 * An anchor sits on a column row, so it can only be on that row's left or right
 * edge — there is no top or bottom of a row to attach to. The two ends always
 * take opposing faces; which pair is a question of how the line will run.
 *
 * React Flow offsets each end's control point horizontally by `0.5 × gap`
 * between the two ends of the curve, or by `curvature × 25 × √-gap` when that
 * gap is negative (`getControlWithCurvature` in @xyflow/system). A positive gap
 * is therefore a line that runs straight across; a negative one is a line that
 * has to leave the node, turn round and come back. The faces that look at each
 * other are the right pair while the tables are clear of one another, and the
 * layout's 350px columns keep them that way.
 *
 * Drag a table above or below the one it points at, though, and those two faces
 * overlap: their gap collapses towards zero, and with it the control offset, so
 * the line doubles back in barely any space at all. The away-facing pair is the
 * better one there — its gap is more than a node's width negative, so the line
 * swings out properly before it turns. The curve starts a `STUB_LENGTH` in from
 * each face (`relationPath`), so the gap it is given is the one between the
 * faces less both stubs, and that is the gap measured here. The notation stays
 * put either way; it is drawn square to the face rather than to the curve (see
 * `MARKER_BOX`).
 */
function facingSides(source: Box, target: Box): Sides {
  const leftward = target.x + target.width / 2 < source.x + source.width / 2;
  const gap = leftward
    ? source.x - (target.x + target.width)
    : target.x - (source.x + source.width);

  // Positive, and wide enough for both ends' straight runs — below that the
  // curve between them is gone and the line turns inside its own notation.
  const facing = gap >= 2 * STUB_LENGTH;
  return leftward === facing
    ? { source: "left", target: "right" }
    : { source: "right", target: "left" };
}

/**
 * Which handle each edge uses at each end.
 *
 * The row is fixed by the relationship — `orders.user_id` to `users.id` — and
 * only the side is a matter of where the tables happen to be, so that is all
 * this decides. Dragging a table moves its rows, and the anchors move with
 * them; drag it past the table at the other end and the sides swap, which is
 * what keeps both ends square to the node they touch.
 */
export function anchorEdges(
  relations: Relation[],
  boxes: ReadonlyMap<string, Box>,
): Map<string, Anchor> {
  const anchors = new Map<string, Anchor>();
  for (const edge of relations) {
    const source = boxes.get(edge.source);
    const target = boxes.get(edge.target);
    if (!source || !target) continue;
    // A table that references itself has no other table to face, and the two
    // sides `facingSides` would fall back to send the line across the node's
    // own width. One side for both ends keeps the whole loop outside the box.
    const sides: Sides =
      edge.source === edge.target
        ? { source: "right", target: "right" }
        : facingSides(source, target);
    anchors.set(edge.id, {
      source: handleId("s", sides.source, edge.data?.sourceColumn ?? ""),
      target: handleId("t", sides.target, edge.data?.targetColumn ?? ""),
      sides,
    });
  }
  return anchors;
}
