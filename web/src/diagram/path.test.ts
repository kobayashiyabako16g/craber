import { Position } from "@xyflow/react";
import { describe, expect, it } from "vitest";

import { STUB_LENGTH } from "./handles";
import { relationPath } from "./path";

/** One SVG path command: its letter, and the numbers after it. */
type Command = { letter: string; numbers: number[] };

function commands(path: string): Command[] {
  return (path.match(/[A-Za-z][^A-Za-z]*/g) ?? []).map((command) => ({
    letter: command.charAt(0),
    numbers: (command.slice(1).match(/-?\d+(\.\d+)?/g) ?? []).map(Number),
  }));
}

/** Two tables far apart vertically — the arrangement that bent the notation. */
function stacked(): Command[] {
  const [path] = relationPath({
    sourceX: 100,
    sourceY: 0,
    sourcePosition: Position.Right,
    targetX: 140,
    targetY: 900,
    targetPosition: Position.Left,
    curvature: 0.45,
  });
  return commands(path);
}

describe("relationPath", () => {
  it("runs straight out of each node for the notation's length", () => {
    const [move, out, , back] = stacked();
    // Starts on the node, and the first thing it does is run horizontally away
    // from it: the crow's foot's whole length lies on that straight piece.
    expect(move).toEqual({ letter: "M", numbers: [100, 0] });
    expect(out).toEqual({ letter: "L", numbers: [100 + STUB_LENGTH, 0] });
    // And arrives the same way, from the far end of the other node's run.
    expect(back).toEqual({ letter: "L", numbers: [140, 900] });
  });

  it("curves only between the two straight runs", () => {
    const curve = stacked()[2];
    expect(curve?.letter).toBe("C");
    // The bezier is handed the stub ends, not the anchors themselves.
    expect(curve?.numbers.slice(4)).toEqual([140 - STUB_LENGTH, 900]);
  });

  it("takes the stub off a left face in the other direction", () => {
    const [path] = relationPath({
      sourceX: 400,
      sourceY: 10,
      sourcePosition: Position.Left,
      targetX: 0,
      targetY: 200,
      targetPosition: Position.Right,
    });
    const [, out, , back] = commands(path);
    expect(out).toEqual({ letter: "L", numbers: [400 - STUB_LENGTH, 10] });
    expect(back).toEqual({ letter: "L", numbers: [0, 200] });
  });
});
