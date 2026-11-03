import { describe, expect, it } from "vitest";

/**
 * The CSS naming rules, checked against the source rather than written down
 * and hoped for:
 *
 * - a class is `craber-<block>[-<element>…]`, a state is `is-<state>` set
 *   beside the class it qualifies, and React Flow's own classes are only
 *   overridden, never invented;
 * - each stylesheet owns the blocks listed in `OWNERS`, so the file a class
 *   lives in follows from its name — `theme.css` holds only what more than one
 *   component uses;
 * - every class the components use is defined, and every class defined is used.
 *
 * Read through Vite's `?raw` imports, so no Node typings are needed.
 */

const OWNERS: Record<string, string[]> = {
  "theme.css": [
    "craber-app",
    "craber-body",
    "craber-message",
    "craber-icon-button",
    "craber-ghost-button",
    "craber-primary-button",
  ],
  "components/TopBar/TopBar.css": ["craber-topbar"],
  "components/Sidebar/Sidebar.css": ["craber-sidebar"],
  "components/Diagram/Diagram.css": ["craber-canvas", "craber-edge"],
  "components/Toolbar/Toolbar.css": ["craber-toolbar"],
  "components/TableNode/TableNode.css": ["craber-node"],
  "components/ColumnGlyph/ColumnGlyph.css": ["craber-glyph"],
  "components/TableDetails/TableDetails.css": ["craber-details"],
  "components/WorkflowPicker/WorkflowPicker.css": ["craber-picker"],
  "components/StepBar/StepBar.css": ["craber-stepbar"],
  "components/Markers/Markers.css": ["craber-markers"],
};

const stylesheets = rel(
  import.meta.glob<string>(["./**/*.css", "!./generated/**"], {
    query: "?raw",
    import: "default",
    eager: true,
  }),
);
const sources = rel(
  import.meta.glob<string>(["./**/*.{ts,tsx}", "!./**/*.test.ts", "!./types/**", "!./generated/**"], {
    query: "?raw",
    import: "default",
    eager: true,
  }),
);

const CLASS = /^craber-[a-z0-9]+(?:-[a-z0-9]+)*$/;
const STATE = /^is-[a-z]+$/;
const REACT_FLOW = /^react-flow__[a-z-]+$/;
// The crates, which comments and tooltips name in the same `craber-` shape.
const CRATE = /^craber-(?:core|parser|introspect|layout|render|export|site)$/;

function rel(files: Record<string, string>): Record<string, string> {
  return Object.fromEntries(Object.entries(files).map(([path, text]) => [path.replace(/^\.\//, ""), text]));
}

/** Each rule's selector list, with comments and declaration blocks dropped. */
function selectors(css: string): string[] {
  return css
    .replace(/\/\*[\s\S]*?\*\//g, "")
    .split(/\{[^{}]*\}/)
    .flatMap((list) => list.split(","))
    .map((s) => s.trim())
    .filter(Boolean);
}

function classesIn(selector: string): string[] {
  return [...selector.matchAll(/\.(-?[_a-zA-Z][\w-]*)/g)].flatMap((m) => m[1] ?? []);
}

const defined = Object.entries(stylesheets).flatMap(([file, css]) =>
  selectors(css).flatMap((selector) => classesIn(selector).map((name) => ({ file, selector, name }))),
);

/**
 * Whole `craber-…` and `is-…` words outside comments. A marker id built as
 * `craber-${kind}-…` never matches: the prefix alone is not a word.
 */
function used(): Set<string> {
  const names = new Set<string>();
  for (const source of Object.values(sources)) {
    const code = source.replace(/\/\*[\s\S]*?\*\//g, "").replace(/\/\/.*$/gm, "");
    for (const m of code.matchAll(/(?<![\w-])(?:craber|is)-[a-z0-9]+(?:-[a-z0-9]+)*(?![\w-])/g)) {
      if (!CRATE.test(m[0])) names.add(m[0]);
    }
  }
  return names;
}

function owns(block: string, name: string): boolean {
  return name === block || name.startsWith(`${block}-`);
}

describe("CSS class names", () => {
  it("are a craber- block, an is- state or a React Flow class", () => {
    const stray = defined.filter(({ name }) => !CLASS.test(name) && !STATE.test(name) && !REACT_FLOW.test(name));
    expect(stray).toEqual([]);
  });

  it("set a state only beside the class it qualifies", () => {
    const bare = Object.values(stylesheets)
      .flatMap(selectors)
      .flatMap((selector) => selector.split(/[\s>+~]+/))
      .filter((compound) => /\.is-/.test(compound) && !/\.craber-/.test(compound));
    expect(bare).toEqual([]);
  });

  it("are defined in the stylesheet that owns their block", () => {
    expect(Object.keys(stylesheets).sort()).toEqual(Object.keys(OWNERS).sort());
    const misplaced = defined
      .filter(({ name }) => CLASS.test(name))
      .filter(({ file, name }) => !OWNERS[file]?.some((block) => owns(block, name)))
      .map(({ file, name }) => `${file}: ${name}`);
    expect(misplaced).toEqual([]);
  });

  it("match between the stylesheets and the components", () => {
    const css = new Set(defined.filter(({ name }) => !REACT_FLOW.test(name)).map(({ name }) => name));
    const tsx = used();
    expect([...tsx].filter((name) => !css.has(name)).sort()).toEqual([]);
    expect([...css].filter((name) => !tsx.has(name)).sort()).toEqual([]);
  });
});
