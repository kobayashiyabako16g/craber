import { useEffect, useMemo, useState } from "react";

import { ColumnGlyph } from "@/components/ColumnGlyph/ColumnGlyph";
import { CheckIcon, CloseIcon, CopyIcon, TableIcon } from "@/components/Icons/Icons";
import { relationshipsOf, type Relation, type Relationship } from "@/lib/graph";
import { foreignKeyColumns, primaryKeyColumns, qualifiedName, uniqueColumns } from "@/lib/schema";
import { tableDdl } from "@/lib/sql";
import type { SqlToken } from "@/lib/sqlHighlight";
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

const TABS = [
  { id: "columns", label: "Columns" },
  { id: "relations", label: "Relations" },
  { id: "ddl", label: "DDL" },
] as const;
type Tab = (typeof TABS)[number]["id"];

/**
 * What the canvas cannot say about the selected table.
 *
 * The node shows a column's name and type because that is what a diagram is
 * for. Everything else about it — nullability, defaults, comments, the indexes,
 * the exact columns each relationship joins — would turn every node into a wall
 * of text, so it lives here, for the one table the reader asked about.
 *
 * Three tabs, because they answer three questions: what is in it, what is it
 * joined to, and how would I create it. The tab is kept from one table to the
 * next, so a reader comparing definitions does not have to ask again each time.
 */
export function TableDetails({ table, relations, known, onSelect, onClose }: TableDetailsProps) {
  const [tab, setTab] = useState<Tab>("columns");
  const relationships = useMemo(
    () => relationshipsOf(qualifiedName(table), relations, known),
    [table, relations, known],
  );

  return (
    <aside className="craber-details">
      <header className="craber-details-head">
        <div className="craber-details-eyebrow">
          <TableIcon size={13} />
          <span>{table.schema ?? "Table"}</span>
          <button
            type="button"
            className="craber-icon-button craber-details-close"
            title="Close"
            onClick={onClose}
          >
            <CloseIcon />
          </button>
        </div>
        <h2 className="craber-details-title">{table.name}</h2>
        {table.comment && <p className="craber-details-comment">{table.comment}</p>}

        <div className="craber-details-tabs">
          {TABS.map(({ id, label }) => (
            <button
              type="button"
              key={id}
              className={cx("craber-details-tab", tab === id && "is-active")}
              aria-pressed={tab === id}
              onClick={() => setTab(id)}
            >
              {label}
            </button>
          ))}
        </div>
      </header>

      <div className="craber-details-body">
        {tab === "columns" && (
          <ColumnsTab table={table} relationships={relationships} onSelect={onSelect} />
        )}
        {tab === "relations" && <RelationsTab relationships={relationships} onSelect={onSelect} />}
        {tab === "ddl" && <DdlTab table={table} />}
      </div>
    </aside>
  );
}

function ColumnsTab({
  table,
  relationships,
  onSelect,
}: {
  table: Table;
  relationships: Relationship[];
  onSelect: (id: string) => void;
}) {
  const pk = primaryKeyColumns(table);
  const fk = foreignKeyColumns(table);
  const unique = uniqueColumns(table);
  const indexes = table.indexes ?? [];
  // `relationshipsOf` writes a key's own end as `<table>.<column>`, so that is
  // what finds the keys a column carries.
  const targets = (column: Column) =>
    relationships.filter(
      (relationship) =>
        relationship.direction === "out" && relationship.from === `${table.name}.${column.name}`,
    );

  return (
    <>
      {(table.columns ?? []).map((column) => (
        <ColumnRow
          key={column.name}
          column={column}
          isPk={pk.has(column.name)}
          isFk={fk.has(column.name)}
          isUnique={unique.has(column.name)}
          targets={targets(column)}
          onSelect={onSelect}
        />
      ))}

      {indexes.length > 0 && (
        <section>
          <h3 className="craber-details-section">Indexes</h3>
          {indexes.map((index, i) => (
            <div className="craber-details-index" key={index.name ?? `index-${i}`}>
              <div className="craber-details-index-text">
                <div className="craber-details-index-name">{index.name ?? "(unnamed)"}</div>
                <div className="craber-details-index-columns">({index.columns.join(", ")})</div>
              </div>
              <span className={cx("craber-details-badge", index.unique ? "is-unique" : "is-plain")}>
                {index.unique ? "UNIQUE" : "INDEX"}
              </span>
            </div>
          ))}
        </section>
      )}
    </>
  );
}

/**
 * One column: what it is on the first line, and under it only what it has —
 * a default, a comment, the column a foreign key points at.
 *
 * The flags are badges rather than a list of facts: `PK`, `FK`, `UNIQUE` and
 * `NOT NULL` are read down the panel's right edge the way the glyphs are read
 * down a node's left, and a nullable column is the one with nothing there.
 */
function ColumnRow({
  column,
  isPk,
  isFk,
  isUnique,
  targets,
  onSelect,
}: {
  column: Column;
  isPk: boolean;
  isFk: boolean;
  isUnique: boolean;
  targets: Relationship[];
  onSelect: (id: string) => void;
}) {
  return (
    <div className="craber-details-column">
      <div className="craber-details-column-head">
        <ColumnGlyph column={column} isPk={isPk} isFk={isFk} plain={false} />
        <span className="craber-details-column-name">{column.name}</span>
        <span className="craber-details-column-type">{column.dataType}</span>
        <span className="craber-details-badges">
          {isPk && <span className="craber-details-badge is-pk">PK</span>}
          {isFk && <span className="craber-details-badge is-fk">FK</span>}
          {isUnique && <span className="craber-details-badge is-unique">UNIQUE</span>}
          {!column.nullable && <span className="craber-details-badge is-plain">NOT NULL</span>}
        </span>
      </div>
      {column.default != null && (
        <div className="craber-details-default">DEFAULT {column.default}</div>
      )}
      {column.comment && <p className="craber-details-prose">{column.comment}</p>}
      {targets.map((target) => (
        <button
          type="button"
          key={target.id}
          className="craber-details-jump"
          title={`Go to ${target.other}`}
          onClick={() => onSelect(target.other)}
        >
          → {target.to}
        </button>
      ))}
    </div>
  );
}

/**
 * Both directions, apart: the keys this table declares, and the ones other
 * tables point back at it with. They are different questions — what does a row
 * here need, and what breaks if a row here goes.
 */
function RelationsTab({
  relationships,
  onSelect,
}: {
  relationships: Relationship[];
  onSelect: (id: string) => void;
}) {
  const outgoing = relationships.filter((relationship) => relationship.direction === "out");
  const incoming = relationships.filter((relationship) => relationship.direction === "in");
  return (
    <>
      <RelationList
        title="References"
        empty="This table declares no foreign keys."
        relationships={outgoing}
        onSelect={onSelect}
      />
      <RelationList
        title="Referenced by"
        empty="No table refers to this one."
        relationships={incoming}
        onSelect={onSelect}
      />
    </>
  );
}

function RelationList({
  title,
  empty,
  relationships,
  onSelect,
}: {
  title: string;
  empty: string;
  relationships: Relationship[];
  onSelect: (id: string) => void;
}) {
  return (
    <section>
      <h3 className="craber-details-section">{title}</h3>
      {relationships.length === 0 ? (
        <p className="craber-details-empty">{empty}</p>
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
              <span className="craber-details-relation-name">{relationship.other}</span>
              {relationship.inferred && (
                <span className="craber-details-badge is-plain">inferred</span>
              )}
              <span className="craber-details-badge is-fk">{relationship.cardinality}</span>
            </span>
            <span className="craber-details-relation-join">
              {relationship.from} → {relationship.to}
            </span>
          </button>
        ))
      )}
    </section>
  );
}

/**
 * The table as SQL — the statements `Export SQL` would write for it, to read
 * or to copy without downloading the whole schema.
 */
function DdlTab({ table }: { table: Table }) {
  const ddl = useMemo(() => tableDdl(table), [table]);
  const [highlight, setHighlight] = useState<((sql: string) => SqlToken[][]) | null>(null);
  const [copied, setCopied] = useState(false);

  // Shiki arrives with the first statement shown, as it does for the step bar;
  // until then the definition is drawn plain.
  useEffect(() => {
    let live = true;
    void import("@/lib/sqlHighlight").then(({ sqlTokens }) => {
      if (live) setHighlight(() => sqlTokens);
    });
    return () => {
      live = false;
    };
  }, []);

  useEffect(() => {
    if (!copied) return;
    const timer = window.setTimeout(() => setCopied(false), 1600);
    return () => window.clearTimeout(timer);
  }, [copied]);

  return (
    <section>
      <div className="craber-details-ddl-head">
        <h3 className="craber-details-section">Definition</h3>
        <button
          type="button"
          className="craber-ghost-button"
          onClick={() => {
            // Refused outside a secure context, where the text can still be
            // selected by hand; the button just does not claim to have copied.
            void navigator.clipboard?.writeText(ddl).then(
              () => setCopied(true),
              () => {},
            );
          }}
        >
          {copied ? <CheckIcon size={13} /> : <CopyIcon size={13} />}
          {copied ? "Copied" : "Copy"}
        </button>
      </div>
      <pre className="craber-details-ddl">
        <code>
          {highlight
            ? highlight(ddl).map((line, i) => (
                <span key={i}>
                  {line.map((token, j) => (
                    <span key={j} style={token.color ? { color: token.color } : undefined}>
                      {token.text}
                    </span>
                  ))}
                  {"\n"}
                </span>
              ))
            : ddl}
        </code>
      </pre>
    </section>
  );
}
