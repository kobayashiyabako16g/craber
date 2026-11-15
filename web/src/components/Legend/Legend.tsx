import { EndShape } from "@/components/Markers/Markers";
import { MARKER_LENGTH } from "@/lib/edgeGeometry";
import type { EndKind } from "@/lib/graph";

import "./Legend.css";

const ENDS: readonly { kind: EndKind; label: string }[] = [
  { kind: "zero-many", label: "zero or many" },
  { kind: "zero-one", label: "zero or one" },
  { kind: "one-only", label: "exactly one" },
];

/** A stretch of line past the glyph, so it reads as the end of one. */
const WIDTH = MARKER_LENGTH + 10;

const STROKE = {
  stroke: "currentColor",
  strokeWidth: 1.5,
  strokeLinecap: "round",
  strokeLinejoin: "round",
  fill: "none",
} as const;

/**
 * What the ends of a line mean.
 *
 * Crow's-foot notation is the whole reason the diagram can say `n:1` without
 * writing it, and it says nothing to a reader who has not met it. The glyphs
 * are `Markers.tsx`'s own shapes, so the key cannot drift from what is drawn.
 */
export function Legend() {
  return (
    <ul className="craber-legend" aria-label="Notation">
      {ENDS.map(({ kind, label }) => (
        <li className="craber-legend-item" key={kind}>
          <svg width={WIDTH} height={16} viewBox={`0 0 ${WIDTH} 16`} aria-hidden="true">
            <path d={`M0 8 H${WIDTH}`} {...STROKE} />
            <EndShape kind={kind} stroke={STROKE} />
          </svg>
          {label}
        </li>
      ))}
      <li className="craber-legend-item">
        <svg width={WIDTH} height={16} viewBox={`0 0 ${WIDTH} 16`} aria-hidden="true">
          <path d={`M0 8 H${WIDTH}`} {...STROKE} strokeDasharray="4 3" />
        </svg>
        inferred from naming
      </li>
    </ul>
  );
}
