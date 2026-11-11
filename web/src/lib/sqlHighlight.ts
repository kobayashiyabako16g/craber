import sql from "@shikijs/langs/sql";
import { createCssVariablesTheme, createHighlighterCoreSync } from "shiki/core";
import { createJavaScriptRegexEngine } from "shiki/engine/javascript";

/**
 * A step's SQL → coloured tokens, for the expanded step bar.
 *
 * No React, like `workflow.ts`. Shiki with only the SQL grammar and the JavaScript
 * regex engine, so there is no WASM to fetch and the highlighter can be built
 * synchronously. It is still the biggest thing the page loads, so the step bar
 * imports this module the first time it is expanded rather than up front.
 *
 * The theme does not carry colours: each token's colour is a `var(--craber-sql-…)`
 * that `StepBar.css` defines, so the palette stays with the rest of the CSS.
 */

export type SqlToken = { text: string; color: string };

const THEME = "craber";

const highlighter = createHighlighterCoreSync({
  themes: [createCssVariablesTheme({ name: THEME, variablePrefix: "--craber-sql-" })],
  langs: [sql],
  engine: createJavaScriptRegexEngine(),
});

/** One array of tokens per line of `code`. */
export function sqlTokens(code: string): SqlToken[][] {
  return highlighter
    .codeToTokens(code, { lang: "sql", theme: THEME })
    .tokens.map((line) => line.map((token) => ({ text: token.content, color: token.color ?? "" })));
}
