import { useCallback, useEffect, useState } from "react";
import { useReactFlow, type FitViewOptions } from "@xyflow/react";

import type { TableNodeType } from "@/lib/graph";
import { viewStateUrl, type ViewState, type Viewport } from "@/lib/viewState";

/** How a link that names a table frames it: close, but not nose-to-the-glass. */
const FIT_ONE = { maxZoom: 1.2, padding: 0.4 } as const;

/**
 * How the diagram is framed on the first render.
 *
 * A link that names a table but carries no viewport means "look at this table":
 * fitting the whole diagram would drop the reader at the same place every other
 * link does. No table named means fit everything, which is what React Flow does
 * with no options at all. `initial` has been through `forTables`, so a table it
 * names is one this schema has.
 *
 * This is a prop rather than a `fitView` call in an effect. React Flow performs
 * its initial fit once it has *measured* the nodes, and an effect runs well
 * before that: on the first render every node still has `measured: undefined`,
 * so fitting to one of them has no box to fit and silently does nothing, and
 * React Flow's own fit then framed the whole diagram over the top of it.
 */
export function initialFit(initial: ViewState): FitViewOptions | undefined {
  return initial.selected ? { nodes: [{ id: initial.selected }], ...FIT_ONE } : undefined;
}

/**
 * The part of the view that belongs in the URL, and the URL kept in step with
 * it — including which workflow is playing and on which step.
 *
 * A published diagram has no server to keep state in, so the address bar is the
 * only place a reader can point at "this table, at this zoom". Selection, the
 * hidden set and the viewport live here rather than in the canvas because the
 * sidebar, the canvas and the details panel all read them — and because they
 * are exactly what a shared link has to carry.
 */
export function useViewSync(initial: ViewState) {
  const reactFlow = useReactFlow<TableNodeType>();
  const [selected, setSelected] = useState<string | null>(initial.selected);
  const [hidden, setHidden] = useState<ReadonlySet<string>>(() => new Set(initial.hidden));
  const [viewport, setViewport] = useState<Viewport | null>(initial.viewport);
  const [playingName, setPlayingName] = useState<string | null>(initial.workflow);
  const [step, setStep] = useState(initial.step);

  // The address bar follows the view, so a reload — or a link pasted straight
  // out of it — lands where the reader was.
  useEffect(() => {
    const url = viewStateUrl({ selected, hidden: [...hidden], viewport, workflow: playingName, step });
    window.history.replaceState(null, "", url);
  }, [selected, hidden, viewport, playingName, step]);

  /** Start a workflow from its first step, or stop playing one with null. */
  const playWorkflow = useCallback((name: string | null) => {
    setPlayingName(name);
    setStep(0);
  }, []);

  /** Select a table and bring it into view — what the sidebar and the details
   *  panel both mean by clicking a name. */
  const focusTable = useCallback(
    (id: string) => {
      setSelected(id);
      void reactFlow.fitView({ nodes: [{ id }], duration: 400, ...FIT_ONE });
    },
    [reactFlow],
  );

  const toggleHidden = useCallback((id: string) => {
    setHidden((current) => {
      const next = new Set(current);
      if (!next.delete(id)) next.add(id);
      return next;
    });
  }, []);

  const setAllHidden = useCallback((hideEverything: boolean, ids: string[]) => {
    setHidden((current) => {
      const next = new Set(current);
      for (const id of ids) {
        if (hideEverything) next.add(id);
        else next.delete(id);
      }
      return next;
    });
  }, []);

  return {
    selected,
    setSelected,
    hidden,
    focusTable,
    toggleHidden,
    setAllHidden,
    recordViewport: setViewport,
    playingName,
    playWorkflow,
    step,
    setStep,
  };
}
