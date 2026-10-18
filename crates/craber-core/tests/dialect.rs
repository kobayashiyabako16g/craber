//! Picking a dialect from a filename or a connection string.

use std::path::Path;

use craber_core::Dialect;

#[test]
fn a_scheme_names_its_dialect() {
    for (dsn, expected) in [
        ("postgres://user@host/db", Dialect::Postgres),
        ("postgresql://user:pw@host:5432/db", Dialect::Postgres),
        ("mysql://root@127.0.0.1/shop", Dialect::MySql),
        ("mariadb://root@127.0.0.1/shop", Dialect::MySql),
        ("sqlite://app.db", Dialect::Sqlite),
        ("file:app.db?mode=ro", Dialect::Sqlite),
    ] {
        assert_eq!(Dialect::from_dsn(dsn), Some(expected), "{dsn}");
    }
}

/// `sqlite:app.db` — the form with no `//` — is one sqlx accepts, so CrabER has
/// to read it too.
#[test]
fn a_scheme_without_slashes_still_counts() {
    assert_eq!(Dialect::from_dsn("sqlite:app.db"), Some(Dialect::Sqlite));
    assert_eq!(Dialect::from_dsn("sqlite::memory:"), Some(Dialect::Sqlite));
}

#[test]
fn an_unknown_or_absent_scheme_decides_nothing() {
    assert_eq!(Dialect::from_dsn("oracle://host/db"), None);
    assert_eq!(Dialect::from_dsn("app.db"), None);
    assert_eq!(Dialect::from_dsn(""), None);
}

/// The scheme and the filename rules are independent: `pg` names a dialect in a
/// filename but is not a connection scheme, and the reverse holds for `file`.
#[test]
fn filenames_keep_their_own_rules() {
    assert_eq!(
        Dialect::from_path(Path::new("dump.pg.sql")),
        Some(Dialect::Postgres)
    );
    assert_eq!(Dialect::from_dsn("pg://host/db"), None);
}
