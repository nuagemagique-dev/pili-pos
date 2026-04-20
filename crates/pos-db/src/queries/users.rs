use pos_core::user::{User, UserRole};
use rusqlite::{Connection, params};

pub fn get_user(conn: &Connection, insz: &str) -> Result<Option<User>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT insz, name, role, active, pin_hash IS NOT NULL
         FROM users WHERE insz = ?1 AND active = 1",
    )?;

    let mut rows = stmt.query_map(params![insz], |row| {
        let role_str: String = row.get(2)?;
        Ok(User {
            insz: row.get(0)?,
            name: row.get(1)?,
            role: role_str
                .parse()
                .map_err(|e: String| rusqlite::Error::FromSqlConversionFailure(2, rusqlite::types::Type::Text, Box::from(e)))?,
            active: row.get::<_, i32>(3)? != 0,
            has_pin: row.get::<_, bool>(4)?,
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
        "SELECT insz, name, role, active, pin_hash IS NOT NULL
         FROM users WHERE active = 1 ORDER BY name",
    )?;

    let rows = stmt.query_map([], |row| {
        let role_str: String = row.get(2)?;
        Ok(User {
            insz: row.get(0)?,
            name: row.get(1)?,
            role: role_str
                .parse()
                .map_err(|e: String| rusqlite::Error::FromSqlConversionFailure(2, rusqlite::types::Type::Text, Box::from(e)))?,
            active: row.get::<_, i32>(3)? != 0,
            has_pin: row.get::<_, bool>(4)?,
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
    conn.execute(
        "INSERT INTO users (insz, name, role) VALUES (?1, ?2, ?3)",
        params![insz, name, role.as_str()],
    )?;

    Ok(User {
        insz: insz.to_string(),
        name: name.to_string(),
        role,
        active: true,
        has_pin: false,
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

/// Store a bcrypt-hashed PIN for an INSZ.
pub fn set_pin_hash(conn: &Connection, insz: &str, hash: &str) -> Result<(), rusqlite::Error> {
    conn.execute(
        "UPDATE users SET pin_hash = ?1 WHERE insz = ?2",
        params![hash, insz],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
