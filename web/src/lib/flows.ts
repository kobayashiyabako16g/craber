import { FLOWS_VERSION } from "@/generated/metrics";
import type { Flows, Workflow } from "@/types";

/**
 * The workflows published beside the diagram, or none.
 *
 * Unlike `schema.json`, a missing or unreadable `flows.json` is not a failure:
 * most diagrams have no workflows, a site built before they existed has no
 * file, and the dev server only has one after `just sample` wrote it. Each of
 * those still draws the diagram, so each comes back as an empty list — the
 * reason goes to the console, where someone expecting workflows will look.
 */
export async function loadFlows(url = "flows.json"): Promise<Workflow[]> {
  try {
    const response = await fetch(url);
    if (!response.ok) return [];
    const flows = (await response.json()) as Flows;
    if (flows.flowsVersion !== FLOWS_VERSION) {
      console.warn(
        `flows.json is version ${flows.flowsVersion}, this build reads version ${FLOWS_VERSION}`,
      );
      return [];
    }
    return flows.workflows;
  } catch (cause) {
    console.warn(`could not read ${url}:`, cause);
    return [];
  }
}
