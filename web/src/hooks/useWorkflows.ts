import { useEffect, useState } from "react";

import { loadWorkflows } from "@/lib/load";
import type { Workflow } from "@/types";

/** The published workflows — empty until they load, and if they never do. */
export function useWorkflows(): Workflow[] {
  const [workflows, setWorkflows] = useState<Workflow[]>([]);

  useEffect(() => {
    void loadWorkflows().then(setWorkflows);
  }, []);

  return workflows;
}
