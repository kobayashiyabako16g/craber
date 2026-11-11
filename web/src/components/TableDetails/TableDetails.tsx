import { useMemo } from "react";

import { ColumnGlyph } from "@/components/ColumnGlyph/ColumnGlyph";
import { relationshipsOf, type Relation } from "@/lib/graph";
import { CloseIcon, LinkIcon, TableIcon } from "@/components/Icons/Icons";
import { foreignKeyColumns, primaryKeyColumns, qualifiedName, uniqueColumns } from "@/lib/schema";
import type { Column, Table } from "@/types";
import { cx } from "@/lib/cx";

import "./TableDetails.css";

type TableDetailsProps = {
  table: Table;
  /** Every relationship on the canvas; the panel lists this table's. */
  relations: Relation[];
  known: ReadonlyMap<string, Table>;
  onSelect: (id: string) => void;
  onClose: () => void;
};

/**
 * What the canvas cannot say about the selected table.
 *
 * The node shows a column's name and type because that is what a diagram is
 * for. Everything else about it — nullability, defaults, comments, the indexes,
 * the exact columns each relationship joins — would turn every node into a wall
 * of text, so it lives here, for the one table the reader asked about.
 */
export function TableDetails({ table, relations, known, onSelect, onClose }: TableDetailsProps) {
  const pk = primaryKeyColumns(table);
  const fk = foreignKeyColumns(table);
  const unique = uniqueColumns(table);
  const relationships = useMemo(
    () => relationshipsOf(qualifiedName(table), relations, known),
    [table, relations, known],
  );

  return (
    <aside className="craber-details">
      <header className="craber-details-head">
        <TableIcon size={14} />
        <span className="craber-details-title">
          {table.schema && <span className="craber-details-schema">{table.schema}.</span>}
          {table.name}
        </span>
        <button type="button" className="craber-icon-button" title="Close" onClick={onClose}>
          <CloseIcon />
        </button>
      </header>

      <div className="craber-details-body">
        {table.comment && <p className="craber-details-comment">{table.comment}</p>}

        <section>
          <h2 className="craber-details-section">Columns</h2>
          {(table.columns ?? []).map((column) => (
            <ColumnCard
              key={column.name}
              column={column}
              isPk={pk.has(column.name)}
              isFk={fk.has(column.name)}
              isUnique={unique.has(column.name)}
            />
          ))}
        </section>

        {(table.indexes ?? []).length > 0 && (
          <section>
            <h2 className="craber-details-section">Indexes</h2>
            {(table.indexes ?? []).map((index, i) => (
              <div className="craber-details-card" key={index.name ?? `index-${i}`}>
                <div className="craber-details-card-title">{index.name ?? "(unnamed)"}</div>
                <dl>
                  <Fact label="Columns" value={index.columns.join(", ")} mono />
                  {index.unique && <Fact label="Unique" />}
                </dl>
              </div>
            ))}
          </section>
        )}

        <section>
          <h2 className="craber-details-section">Related tables</h2>
          {relationships.length === 0 ? (
            <p className="craber-details-empty">No relationships.</p>
          ) : (
            relationships.map((relationship) => (
              <button
                type="button"
                className="craber-details-relation"
                key={relationship.id}
                onClick={() => onSelect(relationship.other)}
                title={`Go to ${relationship.other}`}
              >
                <span className="craber-details-relation-head">
                  <LinkIcon size={12} className="craber-details-relation-icon" />
                  <span className="craber-details-relation-name">{relationship.other}</span>
                  <span className="craber-details-tag">{relationship.cardinality}</span>
                  {relationship.inferred && <span className="craber-details-tag">inferred</span>}
                </span>
                <span className="craber-details-mono craber-details-relation-join">
                  {relationship.from} → {relationship.to}
                </span>
              </button>
            ))
          )}
        </section>
      </div>
    </aside>
  );
}

function ColumnCard({
  column,
  isPk,
  isFk,
  isUnique,
}: {
  column: Column;
  isPk: boolean;
  isFk: boolean;
  isUnique: boolean;
}) {
  return (
    <div className="craber-details-card">
      <div className="craber-details-card-title craber-details-mono">
        <ColumnGlyph column={column} isPk={isPk} isFk={isFk} plain={false} />
        {column.name}
      </div>
      <dl>
        <Fact label="Type" value={column.dataType} mono />
        <Fact label={column.nullable ? "Nullable" : "Not null"} />
        {isPk && <Fact label="Primary key" />}
        {isUnique && <Fact label="Unique" />}
        {column.default != null && <Fact label="Default" value={column.default} mono />}
        {column.comment && <Fact label="Comment" value={column.comment} prose />}
      </dl>
    </div>
  );
}

/**
 * One labelled fact about a column or an index.
 *
 * A short value sits on the label's line, right-aligned, which keeps a column's
 * type and flags scannable down the panel. Anything longer than that — a
 * comment, a `nextval(...)` default — gets the full width of the panel on its
 * own line instead: squeezed into the value column it wraps after three or four
 * words, ragged and right-aligned, and reads as though it had been cut off.
 *
 * A flag with no value (`Not null`, `Primary key`) is just the label.
 */
function Fact({
  label,
  value,
  mono,
  prose,
}: {
  label: string;
  value?: string;
  mono?: boolean;
  prose?: boolean;
}) {
  const stacked = prose || (value !== undefined && value.length > 24);
  return (
    <div className={cx("craber-details-fact", stacked && "is-stacked")}>
      <dt>{label}</dt>
      <dd className={cx(mono && "craber-details-mono", prose && "craber-details-prose")}>
        {value}
      </dd>
    </div>
  );
}
