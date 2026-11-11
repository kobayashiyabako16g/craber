import { useReactFlow } from "@xyflow/react";
import { useEffect, useMemo } from "react";

import type { Relation, TableNodeType } from "@/lib/graph";
import { stepLit } from "@/lib/workflow";
import type { Table, Workflow } from "@/types";

/**
 * The workflow being played back, the step it is on, and what that step
 * lights.
 *
 * Until `flows.json` has loaded, a `?flow=` from the URL matches nothing and
 * the diagram draws as usual. A step past the end — a link to a workflow that
 * has since lost steps — shows the last one.
 */
export function usePlayback(
  workflows: Workflow[],
  name: string | null,
  step: number,
  relations: Relation[],
  known: ReadonlyMap<string, Table>,
  hidden: ReadonlySet<string>,
) {
  const reactFlow = useReactFlow<TableNodeType>();
  const playing = useMemo(
    () => workflows.find((workflow) => workflow.name === name) ?? null,
    [workflows, name],
  );
  const current = playing ? Math.min(step, Math.max(playing.steps.length - 1, 0)) : 0;
  const played = useMemo(() => {
    const shown = playing?.steps[current];
    return shown ? stepLit(shown, relations, known) : null;
  }, [playing, current, relations, known]);

  // Each step brings what it touches into view, so a workflow that wanders
  // across a large diagram can be followed without panning after it.
  useEffect(() => {
    if (!played) return;
    const ids = [...played.access.keys()].filter((id) => !hidden.has(id));
    if (ids.length === 0) return;
    void reactFlow.fitView({
      nodes: ids.map((id) => ({ id })),
      duration: 400,
      maxZoom: 1.2,
      padding: 0.3,
    });
    // `hidden` is left out on purpose: hiding a table mid-playback should not
    // move the canvas.
  }, [played, reactFlow]);

  return { playing, step: current, played };
}
