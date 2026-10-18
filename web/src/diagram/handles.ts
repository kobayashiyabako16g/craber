/**
 * What a node's handles and the edges attaching to them have to agree on.
 *
 * React Flow joins an edge to a handle by id, so the string `TableNode` renders
 * and the string `anchors.ts` asks for have to be the same string. It is built
 * here so that neither side writes the format out.
 *
 * In `diagram/` rather than beside the component because `anchors.ts` is pure
 * geometry and used to reach into `TableNode.tsx` for this — which made the
 * geometry, and anything testing it, depend on React.
 */

/** The side of a row an edge attaches to. */
export type Side = "left" | "right";

/** The handle id for one anchor: `s-` to start an edge, `t-` to end one. */
export function handleId(role: "s" | "t", side: Side, column: string): string {
  return `${role}-${side}-${column}`;
}

/** Which side each end of one edge attaches to. */
export type Sides = { source: Side; target: Side };

/**
 * How far the crow's-foot notation reaches along the line from the face it
 * sits on, in diagram units.
 *
 * Here rather than in `Markers.tsx` because the glyph's length is what every
 * line has to keep straight for, and that is geometry: `STUB_LENGTH` below is
 * built from it, and the glyph's own box in `Markers.tsx` is built from it
 * too.
 */
export const MARKER_LENGTH = 26;

/**
 * How far a line runs straight out of a node before it is allowed to bend.
 *
 * The notation's own length plus a little clearance, so that the glyph is
 * never crossed by the part of the line that has started to turn — see
 * `relationPath`. `anchors.ts` reads it to decide whether two faces have room
 * for both of their straight runs, and a self-reference's orthogonal route
 * uses it as the distance before its first corner.
 */
export const STUB_LENGTH = MARKER_LENGTH + 4;
