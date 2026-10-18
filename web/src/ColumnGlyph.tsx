import { DiamondIcon, KeyIcon, LinkIcon } from "./icons";
import type { Column } from "./types";

/**
 * What a column is, in one glyph: key, foreign key, or ordinary.
 *
 * Both the canvas node and the details panel draw this, and they were deciding
 * it separately — a node saying `PK` where the panel said nothing would be a
 * diagram disagreeing with itself about the same column.
 */
export function ColumnGlyph({
  column,
  isPk,
  isFk,
  plain = true,
}: {
  column: Column;
  isPk: boolean;
  isFk: boolean;
  /** Whether an ordinary column gets a glyph at all. The panel lists columns as
   *  cards with their nullability written out, so a diamond there would be
   *  saying the same thing twice. */
  plain?: boolean;
}) {
  if (isPk) return <KeyIcon size={12} className="craber-glyph craber-glyph-pk" />;
  if (isFk) return <LinkIcon size={12} className="craber-glyph craber-glyph-fk" />;
  if (!plain) return null;
  // Filled means NOT NULL: the column always carries a value.
  return (
    <DiamondIcon size={12} filled={!column.nullable} className="craber-glyph craber-glyph-plain" />
  );
}
