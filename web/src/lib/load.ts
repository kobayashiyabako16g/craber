import { FLOWS_FILE, FLOWS_VERSION, SCHEMA_FILE, SCHEMA_VERSION } from "@/generated/metrics";
import type { Flows, Schema, Workflow } from "@/types";

/**
 * The two files a published diagram fetches, each checked against the version
 * of it this bundle reads.
 *
 * The site and the binary that produced its data are versioned independently —
 * a diagram published months ago is still served by whatever bundle is
 * deployed today — so the reader checks rather than assumes. Both versions are
 * generated from `crates/craber-core`.
 */

/**
 * `url` as JSON, refused unless its `versionField` is `expected`.
 *
 * Throws on a failed request, unreadable JSON or the wrong version; what a
 * failure *means* differs between the two files, so that is left to each.
 */
export async function fetchVersioned<T>(
  url: string,
  versionField: keyof T & string,
  expected: number,
): Promise<T> {
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`could not load ${url}: ${response.status} ${response.statusText}`);
  }
  const body = (await response.json()) as T;
  const version = body[versionField];
  if (version !== expected) {
    throw new Error(`${url} is version ${String(version)}, this build reads version ${expected}`);
  }
  return body;
}

/** The diagram. Without it there is nothing to draw, so a failure is the reader's to see. */
export function loadSchema(url = SCHEMA_FILE): Promise<Schema> {
  return fetchVersioned<Schema>(url, "schemaVersion", SCHEMA_VERSION);
}

/**
 * The workflows published beside the diagram, or none.
 *
 * Unlike `schema.json`, a missing or unreadable `flows.json` is not a failure:
 * most diagrams have no workflows, a site built before they existed has no
 * file, and the dev server only has one after `just sample` wrote it. Each of
 * those still draws the diagram, so each comes back as an empty list — the
 * reason goes to the console, where someone expecting workflows will look.
 */
export async function loadWorkflows(url = FLOWS_FILE): Promise<Workflow[]> {
  try {
    return (await fetchVersioned<Flows>(url, "flowsVersion", FLOWS_VERSION)).workflows;
  } catch (cause) {
    console.warn(cause);
    return [];
  }
}
