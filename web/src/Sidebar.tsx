import { useMemo, useState } from "react";
import { List, type RowComponentProps } from "react-window";

import { EyeIcon, EyeOffIcon, SearchIcon, TableIcon } from "./icons";
import { qualifiedName } from "./schema";
import type { Table } from "./types";

const ROW_HEIGHT = 26;

type SidebarProps = {
  tables: Table[];
  selected: string | null;
  hidden: ReadonlySet<string>;
  onSelect: (id: string) => void;
  onToggleHidden: (id: string) => void;
  onSetAllHidden: (hidden: boolean, ids: string[]) => void;
};

/**
 * The table list.
 *
 * Rows are virtualized because the schemas CrabER is built for — a `pg_dump` of
 * a real application — run to hundreds of tables, and mounting a DOM node per
 * table makes the first paint of exactly those schemas the slow one.
 */
export function Sidebar({
  tables,
  selected,
  hidden,
  onSelect,
  onToggleHidden,
  onSetAllHidden,
}: SidebarProps) {
  const [query, setQuery] = useState("");

  const matches = useMemo(() => {
    const needle = query.trim().toLowerCase();
    if (!needle) return tables;
    return tables.filter((table) => qualifiedName(table).toLowerCase().includes(needle));
  }, [tables, query]);

  const matchIds = useMemo(() => matches.map(qualifiedName), [matches]);

  // The button acts on the rows the reader can actually see, so a search is a
  // way to hide a group of tables at once rather than something the button
  // ignores.
  const filtering = matches.length !== tables.length;
  const allMatchesHidden = matches.length > 0 && matchIds.every((id) => hidden.has(id));
  // Counted, not subtracted: `hidden` is whatever `?hide=` carried, so a link
  // written before a table was renamed or dropped holds names this schema does
  // not have — and `tables.length - hidden.size` then reports fewer tables on
  // screen than are on screen.
  const visibleCount = tables.reduce(
    (count, table) => count + (hidden.has(qualifiedName(table)) ? 0 : 1),
    0,
  );

  return (
    <aside className="craber-sidebar">
      <div className="craber-sidebar-head">
        <span className="craber-sidebar-title">Tables</span>
        <button
          type="button"
          className="craber-ghost-button"
          disabled={matches.length === 0}
          onClick={() => onSetAllHidden(!allMatchesHidden, matchIds)}
        >
          {allMatchesHidden ? <EyeIcon size={13} /> : <EyeOffIcon size={13} />}
          {filtering
            ? `${allMatchesHidden ? "Show" : "Hide"} ${matches.length}`
            : allMatchesHidden
              ? "Show all"
              : "Hide all"}
        </button>
      </div>

      <label className="craber-search">
        <SearchIcon size={13} />
        <input
          type="search"
          value={query}
          placeholder="Search tables"
          onChange={(event) => setQuery(event.target.value)}
        />
      </label>

      <div className="craber-sidebar-list">
        {matches.length === 0 ? (
          <p className="craber-sidebar-empty">No table matches “{query}”.</p>
        ) : (
          <List
            rowComponent={TableRow}
            rowCount={matches.length}
            rowHeight={ROW_HEIGHT}
            rowProps={{ tables: matches, selected, hidden, onSelect, onToggleHidden }}
          />
        )}
      </div>

      <div className="craber-sidebar-foot">
        {visibleCount} / {tables.length}
      </div>
    </aside>
  );
}

/**
 * What one row needs, which is the sidebar's own props minus the two that are
 * about the list as a whole.
 *
 * Derived rather than restated: these five were written out twice, so a change
 * to a handler's signature type-checked in one of the copies and not the other.
 */
type RowProps = Pick<
  SidebarProps,
  "tables" | "selected" | "hidden" | "onSelect" | "onToggleHidden"
>;

function TableRow({
  index,
  style,
  tables,
  selected,
  hidden,
  onSelect,
  onToggleHidden,
}: RowComponentProps<RowProps>) {
  const table = tables[index];
  if (!table) return null;
  const id = qualifiedName(table);
  const isHidden = hidden.has(id);

  return (
    <div
      style={style}
      className={`craber-sidebar-row${id === selected ? " is-selected" : ""}${
        isHidden ? " is-hidden" : ""
      }`}
    >
      <button type="button" className="craber-sidebar-row-name" onClick={() => onSelect(id)}>
        <TableIcon size={13} />
        <span>{table.name}</span>
      </button>
      <button
        type="button"
        className="craber-icon-button"
        title={isHidden ? "Show on the canvas" : "Hide from the canvas"}
        onClick={() => onToggleHidden(id)}
      >
        {isHidden ? <EyeOffIcon size={13} /> : <EyeIcon size={13} />}
      </button>
    </div>
  );
}
