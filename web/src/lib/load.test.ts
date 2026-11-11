import { afterEach, describe, expect, it, vi } from "vitest";

import { FLOWS_VERSION, SCHEMA_VERSION } from "@/generated/metrics";

import { loadSchema, loadWorkflows } from "./load";

/** Answer every `fetch` with `body`, or with a bare status when it is a number. */
function serve(body: unknown) {
  vi.stubGlobal(
    "fetch",
    vi.fn(async () =>
      typeof body === "number"
        ? new Response(null, { status: body, statusText: "Not Found" })
        : new Response(JSON.stringify(body)),
    ),
  );
}

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe("loadSchema", () => {
  it("returns a schema of the version this build reads", async () => {
    serve({ schemaVersion: SCHEMA_VERSION, name: "shop", tables: [] });
    expect((await loadSchema()).name).toBe("shop");
  });

  it("refuses another version, naming both", async () => {
    serve({ schemaVersion: SCHEMA_VERSION + 1, name: "shop" });
    await expect(loadSchema()).rejects.toThrow(`version ${SCHEMA_VERSION + 1}`);
  });

  it("says which file it could not load", async () => {
    serve(404);
    await expect(loadSchema()).rejects.toThrow("could not load schema.json: 404");
  });
});

describe("loadWorkflows", () => {
  it("returns the workflows of a readable file", async () => {
    serve({ flowsVersion: FLOWS_VERSION, workflows: [{ name: "checkout", steps: [] }] });
    expect((await loadWorkflows()).map((w) => w.name)).toEqual(["checkout"]);
  });

  /** No workflows is the common case, not an error: the diagram still draws. */
  it("is empty, not a failure, when the file is missing or another version", async () => {
    vi.spyOn(console, "warn").mockImplementation(() => {});
    serve(404);
    expect(await loadWorkflows()).toEqual([]);
    serve({ flowsVersion: FLOWS_VERSION + 1, workflows: [{ name: "x", steps: [] }] });
    expect(await loadWorkflows()).toEqual([]);
  });
});
