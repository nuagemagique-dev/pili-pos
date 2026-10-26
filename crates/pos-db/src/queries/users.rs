use pos_core::user::{User, UserRole};
use rusqlite::{Connection, params};

pub fn get_user(conn: &Connection, insz: &str) -> Result<Option<User>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT insz, name, role, active, pin_hash IS NOT NULL, presence_only
         FROM users WHERE insz = ?1 AND active = 1",
    )?;

    let mut rows = stmt.query_map(params![insz], |row| {
        let role_str: String = row.get(2)?;
        Ok(User {
            insz: row.get(0)?,
            name: row.get(1)?,
            role: role_str.parse().map_err(|e: String| {
                rusqlite::Error::FromSqlConversionFailure(
                    2,
                    rusqlite::types::Type::Text,
                    Box::from(e),
                )
            })?,
            active: row.get::<_, i32>(3)? != 0,
            has_pin: row.get::<_, bool>(4)?,
            presence_only: row.get::<_, i32>(5)? != 0,
        })
    })?;

    match rows.next() {
        Some(Ok(user)) => Ok(Some(user)),
        Some(Err(e)) => Err(e),
        None => Ok(None),
    }
}

pub fn list_users(conn: &Connection) -> Result<Vec<User>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT insz, name, role, active, pin_hash IS NOT NULL, presence_only
         FROM users WHERE active = 1 AND insz <> ?1 ORDER BY name",
    )?;

    // The system identity is not an operator (see `SYSTEM_INSZ`).
    let rows = stmt.query_map([pos_core::user::SYSTEM_INSZ], |row| {
        let role_str: String = row.get(2)?;
        Ok(User {
            insz: row.get(0)?,
            name: row.get(1)?,
            role: role_str.parse().map_err(|e: String| {
                rusqlite::Error::FromSqlConversionFailure(
                    2,
                    rusqlite::types::Type::Text,
                    Box::from(e),
                )
            })?,
            active: row.get::<_, i32>(3)? != 0,
            has_pin: row.get::<_, bool>(4)?,
            presence_only: row.get::<_, i32>(5)? != 0,
        })
    })?;

    rows.collect()
}

pub fn create_user(
    conn: &Connection,
    insz: &str,
    name: &str,
    role: UserRole,
) -> Result<User, rusqlite::Error> {
    create_user_flagged(conn, insz, name, role, false)
}

/// [`create_user`] with the presence-only flag: an employee who never
/// uses the till (kitchen), clocked in and out by a manager. The
/// caller keeps the role at `waiter` and stores no PIN for them.
#[expect(
    clippy::fn_params_excessive_bools,
    reason = "the presence-only flag names the function; its single caller is the engine's user creation"
)]
pub fn create_user_flagged(
    conn: &Connection,
    insz: &str,
    name: &str,
    role: UserRole,
    presence_only: bool,
) -> Result<User, rusqlite::Error> {
    // A deactivated user (never deleted: their INSZ is on archived
    // events) comes back with the new name, role and flag, and no PIN.
    // An active one is a duplicate: the UNIQUE constraint refuses it.
    let reactivated = conn.execute(
        "UPDATE users SET name = ?2, role = ?3, pin_hash = NULL, active = 1,
                presence_only = ?4
         WHERE insz = ?1 AND active = 0",
        params![insz, name, role.as_str(), presence_only],
    )?;
    if reactivated == 0 {
        conn.execute(
            "INSERT INTO users (insz, name, role, presence_only) VALUES (?1, ?2, ?3, ?4)",
            params![insz, name, role.as_str(), presence_only],
        )?;
    }

    Ok(User {
        insz: insz.to_string(),
        name: name.to_string(),
        role,
        active: true,
        has_pin: false,
        presence_only,
    })
}

/// Retrieve the bcrypt PIN hash for an INSZ (None if no PIN set).
pub fn get_pin_hash(conn: &Connection, insz: &str) -> Result<Option<String>, rusqlite::Error> {
    conn.query_row(
        "SELECT pin_hash FROM users WHERE insz = ?1 AND active = 1",
        params![insz],
        |row| row.get(0),
    )
}

/// Store a bcrypt-hashed PIN for an active user
/// (`QueryReturnedNoRows` when there is none).
pub fn set_pin_hash(conn: &Connection, insz: &str, hash: &str) -> Result<(), rusqlite::Error> {
    let updated = conn.execute(
        "UPDATE users SET pin_hash = ?1 WHERE insz = ?2 AND active = 1",
        params![hash, insz],
    )?;
    if updated == 0 {
        return Err(rusqlite::Error::QueryReturnedNoRows);
    }
    Ok(())
}

/// Store the first PIN of an active user who has none yet. Returns
/// false when the user already has a PIN (or is unknown/inactive), so
/// two terminals racing on the same tile cannot both choose it.
///
/// A manager's first PIN only on first boot (no manager has a PIN
/// yet); later managers get theirs from a manager. Checked in the
/// UPDATE itself, so two claims cannot both pass.
pub fn set_first_pin_hash(
    conn: &Connection,
    insz: &str,
    hash: &str,
) -> Result<bool, rusqlite::Error> {
    let updated = conn.execute(
        "UPDATE users SET pin_hash = ?1
         WHERE insz = ?2 AND active = 1 AND pin_hash IS NULL
           AND (role <> 'admin' OR NOT EXISTS (
                SELECT 1 FROM users
                WHERE role = 'admin' AND active = 1 AND pin_hash IS NOT NULL))",
        params![hash, insz],
    )?;
    Ok(updated == 1)
}

/// Whether an active manager has a PIN (see [`set_first_pin_hash`]).
pub fn any_admin_has_pin(conn: &Connection) -> Result<bool, rusqlite::Error> {
    conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM users
                        WHERE role = 'admin' AND active = 1 AND pin_hash IS NOT NULL)",
        [],
        |row| row.get(0),
    )
}

/// Deactivate a user: they can no longer log in, and leave the lists.
/// Never a deletion - their INSZ stays on the events they signed.
/// `QueryReturnedNoRows` when no active user has this INSZ.
pub fn deactivate_user(conn: &Connection, insz: &str) -> Result<(), rusqlite::Error> {
    let updated = conn.execute(
        "UPDATE users SET active = 0 WHERE insz = ?1 AND active = 1",
        params![insz],
    )?;
    if updated == 0 {
        return Err(rusqlite::Error::QueryReturnedNoRows);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::schema::run_migrations(&conn).unwrap();
        conn
    }

    #[test]
    fn a_deactivated_user_cannot_log_in_and_comes_back_when_rehired() {
        let conn = db();
        create_user(&conn, "12345678958", "Anna", UserRole::Cashier).unwrap();
        set_pin_hash(&conn, "12345678958", "hash").unwrap();
        deactivate_user(&conn, "12345678958").unwrap();
        assert!(get_user(&conn, "12345678958").unwrap().is_none());
        assert!(list_users(&conn).unwrap().is_empty());
        assert!(matches!(
            deactivate_user(&conn, "12345678958"),
            Err(rusqlite::Error::QueryReturnedNoRows)
        ));
        assert!(matches!(
            set_pin_hash(&conn, "12345678958", "hash"),
            Err(rusqlite::Error::QueryReturnedNoRows)
        ));

        let back = create_user(&conn, "12345678958", "Anna B.", UserRole::Waiter).unwrap();
        assert_eq!(back.role, UserRole::Waiter);
        let user = get_user(&conn, "12345678958").unwrap().unwrap();
        assert_eq!(user.name, "Anna B.");
        assert!(!user.has_pin, "the old PIN does not come back");
        let rows: i64 = conn
            .query_row("SELECT COUNT(*) FROM users", [], |r| r.get(0))
            .unwrap();
        assert_eq!(rows, 1, "never deleted, never duplicated");
    }

    #[test]
    fn the_presence_only_flag_is_stored_listed_and_reset_on_rehire() {
        let conn = db();
        let user =
            create_user_flagged(&conn, "12345678958", "Kim", UserRole::Waiter, true).unwrap();
        assert!(user.presence_only);
        assert!(
            get_user(&conn, "12345678958")
                .unwrap()
                .unwrap()
                .presence_only
        );
        assert!(list_users(&conn).unwrap()[0].presence_only);
        // A plain user is not flagged.
        let plain = create_user(&conn, "11111111111", "Alice", UserRole::Waiter).unwrap();
        assert!(!plain.presence_only);

        // Deactivate then create again as a till user: the flag goes.
        deactivate_user(&conn, "12345678958").unwrap();
        let back = create_user(&conn, "12345678958", "Kim", UserRole::Cashier).unwrap();
        assert!(!back.presence_only);
        assert!(
            !get_user(&conn, "12345678958")
                .unwrap()
                .unwrap()
                .presence_only
        );
        // And the other way round.
        deactivate_user(&conn, "12345678958").unwrap();
        create_user_flagged(&conn, "12345678958", "Kim", UserRole::Waiter, true).unwrap();
        assert!(
            get_user(&conn, "12345678958")
                .unwrap()
                .unwrap()
                .presence_only
        );
    }

    #[test]
    fn a_pin_cannot_be_set_for_an_unknown_user() {
        let conn = db();
        assert!(matches!(
            set_pin_hash(&conn, "12345678958", "hash"),
            Err(rusqlite::Error::QueryReturnedNoRows)
        ));
    }
    use crate::schema::run_migrations;

    #[test]
    fn create_and_get_user() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();

        create_user(&conn, "00000000029", "Robot", UserRole::Admin).unwrap();
        let user = get_user(&conn, "00000000029").unwrap().unwrap();
        assert_eq!(user.name, "Robot");
        assert_eq!(user.role, UserRole::Admin);
    }

    #[test]
    fn missing_user_returns_none() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();

        let user = get_user(&conn, "99999999999").unwrap();
        assert!(user.is_none());
    }

    #[test]
    fn new_user_has_no_pin() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();

        create_user(&conn, "11111111111", "Alice", UserRole::Waiter).unwrap();
        let user = get_user(&conn, "11111111111").unwrap().unwrap();
        assert!(!user.has_pin);
        assert!(get_pin_hash(&conn, "11111111111").unwrap().is_none());
    }

    #[test]
    fn set_pin_hash_is_reflected_in_has_pin() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();

        create_user(&conn, "11111111111", "Alice", UserRole::Waiter).unwrap();
        set_pin_hash(&conn, "11111111111", "$2b$12$fakehash").unwrap();

        let user = get_user(&conn, "11111111111").unwrap().unwrap();
        assert!(user.has_pin);
        assert_eq!(
            get_pin_hash(&conn, "11111111111").unwrap().as_deref(),
            Some("$2b$12$fakehash")
        );
    }

    #[test]
    fn duplicate_insz_is_rejected() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();

        create_user(&conn, "11111111111", "Alice", UserRole::Waiter).unwrap();
        let dup = create_user(&conn, "11111111111", "Alice Bis", UserRole::Cashier);
        assert!(dup.is_err(), "insz is the primary key - no duplicates");
    }

    #[test]
    fn list_users_excludes_inactive_and_sorts_by_name() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();

        create_user(&conn, "33333333333", "Zoe", UserRole::Waiter).unwrap();
        create_user(&conn, "11111111111", "Alice", UserRole::Cashier).unwrap();
        create_user(&conn, "22222222222", "Bob", UserRole::Admin).unwrap();
        conn.execute("UPDATE users SET active = 0 WHERE insz = '22222222222'", [])
            .unwrap();

        let users = list_users(&conn).unwrap();
        let names: Vec<&str> = users.iter().map(|u| u.name.as_str()).collect();
        assert_eq!(
            names,
            vec!["Alice", "Zoe"],
            "Bob is inactive, list is sorted by name"
        );
    }

    #[test]
    fn get_user_ignores_inactive_users() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();

        create_user(&conn, "11111111111", "Alice", UserRole::Waiter).unwrap();
        conn.execute("UPDATE users SET active = 0 WHERE insz = '11111111111'", [])
            .unwrap();

        assert!(get_user(&conn, "11111111111").unwrap().is_none());
    }

    #[test]
    fn every_role_round_trips_through_the_db() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();

        for (insz, role) in [
            ("10000000001", UserRole::Admin),
            ("10000000002", UserRole::Cashier),
            ("10000000003", UserRole::Waiter),
        ] {
            create_user(&conn, insz, "Test", role).unwrap();
            let user = get_user(&conn, insz).unwrap().unwrap();
            assert_eq!(user.role, role);
        }
    }

    #[test]
    fn the_system_identity_is_never_listed() {
        let conn = db();
        create_user(&conn, pos_core::user::SYSTEM_INSZ, "Robot", UserRole::Admin).unwrap();
        create_user(&conn, "12345678958", "Anna", UserRole::Cashier).unwrap();
        let listed: Vec<_> = list_users(&conn)
            .unwrap()
            .into_iter()
            .map(|u| u.insz)
            .collect();
        assert_eq!(listed, vec!["12345678958".to_string()]);
    }

    /// Once a manager has a PIN, another manager tile is not claimable.
    #[test]
    fn a_manager_first_pin_is_a_first_boot_only_claim() {
        let conn = db();
        create_user(&conn, "12345678958", "Anna", UserRole::Admin).unwrap();
        create_user(&conn, "85073003328", "Ben", UserRole::Admin).unwrap();
        create_user(&conn, "90010100183", "Chloe", UserRole::Cashier).unwrap();
        assert!(!any_admin_has_pin(&conn).unwrap());

        assert!(set_first_pin_hash(&conn, "12345678958", "anna").unwrap());
        assert!(any_admin_has_pin(&conn).unwrap());
        assert!(!set_first_pin_hash(&conn, "85073003328", "stranger").unwrap());
        assert_eq!(get_pin_hash(&conn, "85073003328").unwrap(), None);
        assert!(set_first_pin_hash(&conn, "90010100183", "chloe").unwrap());
    }

    #[test]
    fn a_first_pin_is_stored_once_only() {
        let conn = db();
        create_user(&conn, "12345678958", "Anna", UserRole::Admin).unwrap();
        assert!(set_first_pin_hash(&conn, "12345678958", "first").unwrap());
        assert!(!set_first_pin_hash(&conn, "12345678958", "second").unwrap());
        assert_eq!(
            get_pin_hash(&conn, "12345678958").unwrap().as_deref(),
            Some("first")
        );
    }

    #[test]
    fn a_first_pin_is_refused_for_an_unknown_or_departed_user() {
        let conn = db();
        assert!(!set_first_pin_hash(&conn, "12345678958", "hash").unwrap());
        create_user(&conn, "12345678958", "Anna", UserRole::Admin).unwrap();
        deactivate_user(&conn, "12345678958").unwrap();
        assert!(!set_first_pin_hash(&conn, "12345678958", "hash").unwrap());
    }
}
