import { Fragment } from "react";

import type { EndKind } from "./graph";
import { MARKER_LENGTH } from "./handles";

/**
 * Crow's-foot notation, as SVG markers the edges reference by id.
 *
 * The id convention `craber-<kind>-<side>` is the contract with the edges that
 * name them; `endKinds` in `graph.ts` decides which kind each end gets.
 */

/**
 * Marker box: `MARKER_LENGTH`×16 diagram units, anchored on the point where the
 * line meets the node. `userSpaceOnUse` keeps the glyph a fixed size rather than
 * scaling it by the line's stroke width.
 *
 * The box is wider than it is tall because the notation reads *along* the line:
 * a glyph the height of a column row, given room to spread out, is legible at
 * the zoom a whole schema is read at. Crammed into a square it becomes a smudge.
 *
 * `refX` is what puts the glyph on the correct side of that anchor: 0 for a
 * marker on a node's right face, where the box runs off to the right and away
 * from the node behind it; `MARKER_LENGTH` for one on a left face, where the
 * box runs back to the left and so stops at the node's edge.
 */
const MARKER_BOX = {
  viewBox: `0 0 ${MARKER_LENGTH} 16`,
  refY: 8,
  markerWidth: MARKER_LENGTH,
  markerHeight: 16,
  markerUnits: "userSpaceOnUse",
  // Square to the node, not to the line — which is why there is a marker per
  // face rather than one per end of a line, and why this is not `auto`.
  //
  // `auto` turns the glyph to follow the curve's tangent where it meets the
  // node. That reads correctly only while the curve actually runs that way for
  // the glyph's whole length, and dragging a table breaks it: put two tables
  // far apart vertically but overlapping horizontally and any bezier between
  // them has to double back, so it leaves the node, turns inside 16 units, and
  // the notation is left hanging off the line at an angle.
  //
  // A crow's foot belongs to the entity rather than to the line, though, and an
  // anchor is always on a row's left or right edge — so the angle is known
  // without asking the curve, and fixing it means no arrangement of tables can
  // rotate the notation off its table.
  orient: "0",
} as const;

const END_KINDS: readonly EndKind[] = ["zero-many", "zero-one", "one-only"];

/**
 * One relationship end, drawn for the tail of a line: the node it belongs to is
 * at x=0 and the line runs away along +x.
 *
 * Crow's-foot notation reads outwards from the entity. The part nearest the
 * table says how many rows are there — a fork for many, a bar for one, two bars
 * for exactly one — and the ring further out is the zero: it may be none.
 */
function EndShape({ kind, stroke }: { kind: EndKind; stroke: object }) {
  switch (kind) {
    case "zero-many":
      return (
        <>
          {/* The fork's toes sit a unit clear of the node rather than against
              it: a foot that touches the border reads as part of the table. */}
          <path d="M1.5 1.6 L12 8 M1.5 14.4 L12 8" {...stroke} />
          <circle cx="19.5" cy="8" r="4.2" {...stroke} />
        </>
      );
    case "zero-one":
      return (
        <>
          <path d="M8 2 V14" {...stroke} />
          <circle cx="19.5" cy="8" r="4.2" {...stroke} />
        </>
      );
    case "one-only":
      return <path d="M8 2 V14 M14 2 V14" {...stroke} />;
  }
}

/**
 * Crow's-foot ends, as SVG markers the edges reference by id.
 *
 * Two things have to hold while a table is being dragged, and they pull in
 * opposite directions. The glyph has to stay square to the line — a foot
 * crossed by its own line reads as a scribble — which means turning with it,
 * `orient="auto"`. And it has to stay put against the node, which means being
 * drawn from the anchor outwards: geometry set back along the line swings like
 * a lever when the curve's tangent moves.
 *
 * Every kind is defined for both ends of a line, the head being the tail
 * mirrored, so that an edge can ask for a notation without also having to know
 * which way round it will be drawn.
 *
 * `context-stroke` takes the colour and opacity of the line the marker sits on,
 * which is what keeps a dimmed edge's ends dimmed too.
 */
export function CardinalityMarkers() {
  const stroke = {
    stroke: "context-stroke",
    strokeWidth: 1.5,
    strokeLinecap: "round",
    strokeLinejoin: "round",
    fill: "none",
  } as const;
  return (
    <svg className="craber-markers" aria-hidden="true">
      <defs>
        {END_KINDS.map((kind) => (
          <Fragment key={kind}>
            {/* On a node's right face the glyph runs off to the right, so it is
                drawn as `EndShape` has it and anchored at x=0. */}
            <marker {...MARKER_BOX} id={`craber-${kind}-right`} refX={0}>
              <EndShape kind={kind} stroke={stroke} />
            </marker>
            {/* On a left face it runs the other way: mirrored, and anchored at
                the far end of the box so the box lies to the left of the node. */}
            <marker {...MARKER_BOX} id={`craber-${kind}-left`} refX={MARKER_LENGTH}>
              <g transform={`translate(${MARKER_LENGTH} 0) scale(-1 1)`}>
                <EndShape kind={kind} stroke={stroke} />
              </g>
            </marker>
          </Fragment>
        ))}
      </defs>
    </svg>
  );
}
