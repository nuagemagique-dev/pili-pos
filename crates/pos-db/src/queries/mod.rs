pub mod allergens;
pub mod badges;
pub mod booking;
pub mod courses;
pub mod deliveries;
pub mod eft;
pub mod fiscal;
pub mod gift_vouchers;
#[cfg(feature = "hubrise")]
pub mod integrations;
pub mod invoicing;
pub mod menu;
#[cfg(feature = "catalog-sync")]
pub mod menu_admin;
pub mod orders;
pub mod owner_settings;
pub mod pending_signs;
#[cfg(feature = "hubrise")]
pub mod price_changes;
pub mod prints;
pub mod reports;
pub mod shifts;
pub mod sold_out;
pub mod table_plan;
#[cfg(feature = "hubrise")]
pub mod till_discounts;
pub mod users;

/// Run `f` atomically: all its writes or none. A SAVEPOINT, so it nests: it works
/// the same inside a transaction or another savepoint the caller started (the
/// engine groups several queries in one transaction), where `unchecked_transaction`
/// would fail. On an error everything `f` wrote is rolled back and the error is
/// returned.
pub fn with_savepoint<T>(
    conn: &rusqlite::Connection,
    f: impl FnOnce(&rusqlite::Connection) -> Result<T, rusqlite::Error>,
) -> Result<T, rusqlite::Error> {
    conn.execute_batch("SAVEPOINT pili_atomic")?;
    match f(conn) {
        Ok(v) => {
            conn.execute_batch("RELEASE pili_atomic")?;
            Ok(v)
        }
        Err(e) => {
            let _ = conn.execute_batch("ROLLBACK TO pili_atomic; RELEASE pili_atomic");
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::with_savepoint;
    use rusqlite::Connection;

    fn conn() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch("CREATE TABLE t (a INTEGER)").unwrap();
        c
    }

    fn count(c: &Connection) -> i64 {
        c.query_row("SELECT COUNT(*) FROM t", [], |r| r.get(0))
            .unwrap()
    }

    fn fail() -> Result<(), rusqlite::Error> {
        Err(rusqlite::Error::QueryReturnedNoRows)
    }

    #[test]
    fn it_commits_on_success_and_rolls_back_on_error() {
        let c = conn();
        with_savepoint(&c, |c| {
            c.execute("INSERT INTO t VALUES (1)", []).map(|_| ())
        })
        .unwrap();
        assert_eq!(count(&c), 1);
        let err = with_savepoint(&c, |c| {
            c.execute("INSERT INTO t VALUES (2)", [])?;
            fail()
        });
        assert!(err.is_err());
        assert_eq!(count(&c), 1);
    }

    #[test]
    fn it_nests_inside_itself_and_inside_a_transaction() {
        let c = conn();
        c.execute_batch("BEGIN").unwrap();
        c.execute("INSERT INTO t VALUES (1)", []).unwrap();
        // An inner failure rolls back only the inner work.
        let inner = with_savepoint(&c, |c| {
            c.execute("INSERT INTO t VALUES (2)", [])?;
            with_savepoint(c, |c| {
                c.execute("INSERT INTO t VALUES (3)", [])?;
                Ok(())
            })?;
            fail()
        });
        assert!(inner.is_err());
        assert_eq!(count(&c), 1);
        // A success inside the caller's transaction is undone with it.
        with_savepoint(&c, |c| {
            c.execute("INSERT INTO t VALUES (4)", []).map(|_| ())
        })
        .unwrap();
        assert_eq!(count(&c), 2);
        c.execute_batch("ROLLBACK").unwrap();
        assert_eq!(count(&c), 0);
    }
}
