import { useCallback, useEffect, useReducer } from "react";
import { useReactFlow } from "@xyflow/react";

import type { TableNodeType } from "@/lib/graph";
import { FIT_ONE, viewReducer, viewStateUrl, type ViewState } from "@/lib/viewState";

/**
 * The part of the view that belongs in the URL, and the URL kept in step with
 * it — including which workflow is playing and on which step.
 *
 * A published diagram has no server to keep state in, so the address bar is the
 * only place a reader can point at "this table, at this zoom". Selection, the
 * hidden set and the viewport live here rather than in the canvas because the
 * sidebar, the canvas and the details panel all read them — and because they
 * are exactly what a shared link has to carry.
 *
 * What each action does is `viewReducer`'s; this hook adds the two things that
 * need a browser: writing the URL, and moving the camera.
 */
export function useViewSync(initial: ViewState) {
  const reactFlow = useReactFlow<TableNodeType>();
  const [view, dispatch] = useReducer(viewReducer, initial);

  // The address bar follows the view, so a reload — or a link pasted straight
  // out of it — lands where the reader was.
  useEffect(() => {
    window.history.replaceState(null, "", viewStateUrl(view));
  }, [view]);

  /** Select a table and bring it into view — what the sidebar and the details
   *  panel both mean by clicking a name. */
  const focusTable = useCallback(
    (id: string) => {
      dispatch({ type: "select", table: id });
      void reactFlow.fitView({ nodes: [{ id }], duration: 400, ...FIT_ONE });
    },
    [reactFlow],
  );

  return { view, dispatch, focusTable };
}
