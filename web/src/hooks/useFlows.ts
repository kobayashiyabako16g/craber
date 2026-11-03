import { useEffect, useState } from "react";

import { loadFlows } from "../flows";
import type { Workflow } from "../types";

/** The published workflows — empty until they load, and if they never do. */
export function useFlows(): Workflow[] {
  const [workflows, setWorkflows] = useState<Workflow[]>([]);

  useEffect(() => {
    void loadFlows().then(setWorkflows);
  }, []);

  return workflows;
}
