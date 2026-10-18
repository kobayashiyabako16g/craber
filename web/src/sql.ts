/**
 * The schema, written back out as SQL.
 *
 * `Export SQL` hands the reader the table definitions and nothing else: no
 * grants, owners, tablespaces or data.
 *
 * All we have to work with is `schema.json`, which is normalized away from the
 * source dialect, so this is a restatement of what the diagram shows rather
 * than a round-trip of the file that was parsed: identifiers come out quoted
 * the standard way, with `"`, whichever dialect they were read from.
 */

import type {
  Action,
  Column,
  EnumType,
  ForeignKey,
  Schema,
  Table,
  TableIndex,
  TableRef,
} from "./types";

/** The whole schema as a single SQL script. */
export function schemaSql(schema: Schema): string {
  const tables = schema.tables ?? [];
  const enums = schema.enums ?? [];
  // Foreign keys go in their own pass at the end rather than inside the
  // `CREATE TABLE`: a schema is a graph and any table order would otherwise
  // reference something that does not exist yet.
  const constraints = tables.flatMap((table) =>
    (table.fks ?? []).filter((fk) => !fk.inferred).map((fk) => foreignKeySql(table, fk)),
  );
  const inferred = tables.reduce(
    (count, table) => count + (table.fks ?? []).filter((fk) => fk.inferred).length,
    0,
  );

  const sections = [
    header(schema, inferred),
    ...enums.map(enumSql),
    ...tables.map(tableSql),
    ...tables.flatMap((table) => (table.indexes ?? []).map((index) => indexSql(table, index))),
    ...constraints,
  ];
  return `${sections.filter(Boolean).join("\n\n")}\n`;
}

/** The filename the download lands under. */
export function sqlFileName(schema: Schema): string {
  const stem = schema.name.replace(/[^\w.-]+/g, "-").replace(/^-+|-+$/g, "");
  return `${stem || "schema"}.sql`;
}

function header(schema: Schema, inferred: number): string {
  const lines = [
    `-- ${schema.name} — table definitions exported by CrabER.`,
    "-- Structure only: no privileges, ownership or data.",
  ];
  // The dashed edges are CrabER's guesses, not constraints anybody declared;
  // writing them out as SQL would put them in the database for real.
  if (inferred > 0) {
    lines.push(
      `-- ${inferred} inferred relationship${inferred === 1 ? "" : "s"} omitted: not declared in the schema.`,
    );
  }
  return lines.join("\n");
}

function enumSql(type: EnumType): string {
  const values = type.values.map(quoteLiteral).join(", ");
  return `CREATE TYPE ${qualifiedSql(type.schema, type.name)} AS ENUM (${values});`;
}

function tableSql(table: Table): string {
  const columns = (table.columns ?? []).map(columnSql);
  const pk = table.pk ?? [];
  if (pk.length > 0) columns.push(`PRIMARY KEY (${columnList(pk)})`);

  const body = columns.length > 0 ? `\n  ${columns.join(",\n  ")}\n` : "";
  const comment = table.comment ? `-- ${table.comment}\n` : "";
  return `${comment}CREATE TABLE ${tableName(table)} (${body});`;
}

function columnSql(column: Column): string {
  const parts = [quote(column.name), column.dataType];
  if (!column.nullable) parts.push("NOT NULL");
  // Defaults are kept as the source wrote them, so they are emitted verbatim
  // rather than re-quoted as a literal.
  if (column.default) parts.push(`DEFAULT ${column.default}`);
  return parts.join(" ");
}

function indexSql(table: Table, index: TableIndex): string {
  const unique = index.unique ? "UNIQUE " : "";
  // An index the source did not name is left unnamed here too, rather than
  // invented: the database generates the same kind of name we would.
  const name = index.name ? `${quote(index.name)} ` : "";
  return `CREATE ${unique}INDEX ${name}ON ${tableName(table)} (${columnList(index.columns)});`;
}

function foreignKeySql(table: Table, fk: ForeignKey): string {
  const name = fk.name ? `CONSTRAINT ${quote(fk.name)} ` : "";
  const clauses = [
    `ALTER TABLE ${tableName(table)}`,
    `  ADD ${name}FOREIGN KEY (${columnList(fk.columns)})`,
    `  REFERENCES ${refName(fk.refTable)} (${columnList(fk.refColumns)})`,
  ];
  if (fk.onDelete !== "noAction") clauses.push(`  ON DELETE ${actionSql(fk.onDelete)}`);
  if (fk.onUpdate !== "noAction") clauses.push(`  ON UPDATE ${actionSql(fk.onUpdate)}`);
  return `${clauses.join("\n")};`;
}

function actionSql(action: Action): string {
  switch (action) {
    case "noAction":
      return "NO ACTION";
    case "restrict":
      return "RESTRICT";
    case "cascade":
      return "CASCADE";
    case "setNull":
      return "SET NULL";
    case "setDefault":
      return "SET DEFAULT";
  }
}

function tableName(table: Table): string {
  return qualifiedSql(table.schema, table.name);
}

/**
 * A reference is written as it was read. An unqualified one stays unqualified:
 * `qualifiedName` resolution is for matching nodes on the canvas, and guessing
 * a namespace here would put a table in the SQL that the source never named.
 */
function refName(ref: TableRef): string {
  return qualifiedSql(ref.schema, ref.name);
}

function qualifiedSql(schema: string | null | undefined, name: string): string {
  return schema ? `${quote(schema)}.${quote(name)}` : quote(name);
}

function columnList(columns: readonly string[]): string {
  return columns.map(quote).join(", ");
}

function quote(identifier: string): string {
  return `"${identifier.replaceAll('"', '""')}"`;
}

function quoteLiteral(value: string): string {
  return `'${value.replaceAll("'", "''")}'`;
}
