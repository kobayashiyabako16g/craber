import { describe, expect, it } from "vitest";

import { type SqlToken, sqlTokens } from "./sqlHighlight";

/** The colour `text` was given on its line. */
function colorOf(line: SqlToken[] | undefined, text: string): string | undefined {
  return line?.find((token) => token.text.trim() === text)?.color;
}

describe("sqlTokens", () => {
  it("keeps the statement's lines", () => {
    const lines = sqlTokens("SELECT\n  u.id\nFROM\n  users u");
    expect(lines.map((line) => line.map((token) => token.text).join(""))).toEqual([
      "SELECT",
      "  u.id",
      "FROM",
      "  users u",
    ]);
  });

  it("colours keywords, strings and numbers apart from the rest", () => {
    const [line] = sqlTokens("WHERE u.email = 'a' AND u.id = 1");
    const keyword = colorOf(line, "WHERE");
    const string = colorOf(line, "'a'");
    const number = colorOf(line, "1");
    const plain = colorOf(line, "u.email");

    expect(keyword).toMatch(/^var\(--craber-sql-/);
    expect(new Set([keyword, string, number, plain]).size).toBe(4);
    expect(colorOf(line, "AND")).toBe(keyword);
  });
});
