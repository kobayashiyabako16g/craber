import { useMemo } from "react";

import { ColumnGlyph } from "./ColumnGlyph";
import { cardinalityLabel } from "./diagram/graph";
import { CloseIcon, LinkIcon, TableIcon } from "./icons";
import { foreignKeyColumns, primaryKeyColumns, qualifiedName, resolveRef, tableIndex } from "./schema";
import type { Column, Table, TableRef } from "./types";

/** One relationship, seen from the table the panel is describing. */
type Relationship = {
  /** The table at the other end. */
  other: string;
  /** `orders.user_id`, from this table's side. */
  from: string;
  /** `users.id`, at the other end. */
  to: string;
  /** `n:1` or `1:1`, written from this table's side. */
  cardinality: string;
  inferred: boolean;
};

type TableDetailsProps = {
  table: Table;
  tables: Table[];
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
export function TableDetails({ table, tables, onSelect, onClose }: TableDetailsProps) {
  const pk = primaryKeyColumns(table);
  const fk = foreignKeyColumns(table);
  const relationships = useMemo(() => relationshipsOf(table, tables), [table, tables]);

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
                key={`${relationship.from}->${relationship.to}`}
                onClick={() => onSelect(relationship.other)}
                title={`Go to ${relationship.other}`}
              >
                <span className="craber-details-relation-head">
                  <LinkIcon size={12} className="craber-glyph-fk" />
                  <span className="craber-details-relation-name">{relationship.other}</span>
                  <span className="craber-details-tag">{relationship.cardinality}</span>
                  {relationship.inferred && <span className="craber-details-tag">inferred</span>}
                </span>
                <span className="craber-mono craber-details-relation-join">
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
}: {
  column: Column;
  isPk: boolean;
  isFk: boolean;
}) {
  return (
    <div className="craber-details-card">
      <div className="craber-details-card-title craber-mono">
        <ColumnGlyph column={column} isPk={isPk} isFk={isFk} plain={false} />
        {column.name}
      </div>
      <dl>
        <Fact label="Type" value={column.dataType} mono />
        <Fact label={column.nullable ? "Nullable" : "Not null"} />
        {isPk && <Fact label="Primary key" />}
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
    <div className={`craber-fact${stacked ? " is-stacked" : ""}`}>
      <dt>{label}</dt>
      <dd className={[mono ? "craber-mono" : "", prose ? "craber-prose" : ""].join(" ").trim()}>
        {value}
      </dd>
    </div>
  );
}

/**
 * Both directions: the foreign keys this table declares, and the ones other
 * tables point back at it with. A reader asking about `users` wants to know
 * what refers to it at least as much as what it refers to.
 */
function relationshipsOf(table: Table, tables: Table[]): Relationship[] {
  const id = qualifiedName(table);
  const byId = tableIndex(tables);
  const relationships: Relationship[] = [];

  const join = (owner: Table, columns: string[], ref: TableRef, refColumns: string[]) => ({
    from: `${owner.name}.${columns[0] ?? "?"}`,
    to: `${ref.name}.${refColumns[0] ?? "?"}`,
  });

  for (const fk of table.fks ?? []) {
    const other = resolveRef(fk.refTable, byId);
    if (!other) continue;
    relationships.push({
      other,
      ...join(table, fk.columns, fk.refTable, fk.refColumns),
      cardinality: cardinalityLabel(fk, "from"),
      inferred: fk.inferred,
    });
  }

  for (const source of tables) {
    if (qualifiedName(source) === id) continue;
    for (const fk of source.fks ?? []) {
      if (resolveRef(fk.refTable, byId) !== id) continue;
      relationships.push({
        other: qualifiedName(source),
        ...join(source, fk.columns, { schema: table.schema, name: table.name }, fk.refColumns),
        // Read from this table's side, so the "one" end is this one.
        cardinality: cardinalityLabel(fk, "to"),
        inferred: fk.inferred,
      });
    }
  }

  return relationships;
}
