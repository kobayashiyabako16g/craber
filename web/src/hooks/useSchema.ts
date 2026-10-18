import { useEffect, useState } from "react";

import { loadSchema } from "../schema";
import type { Schema } from "../types";

/**
 * The published `schema.json`, or why it could not be read.
 *
 * Both failures the reader can actually hit come back as `error`: the file is
 * not there, or it was written by a binary whose wire format this bundle does
 * not read (`loadSchema` checks the version, because a site and the binary that
 * produced its data are deployed independently).
 */
export function useSchema(): { schema: Schema | null; error: string | null } {
  const [schema, setSchema] = useState<Schema | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    loadSchema()
      .then(setSchema)
      .catch((cause: unknown) => setError(String(cause)));
  }, []);

  return { schema, error };
}
