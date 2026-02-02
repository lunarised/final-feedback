use rusqlite::{Connection, Result};

pub fn init_database(db_path: &str) -> Result<Connection> {
    let conn = Connection::open(db_path)?;
    
    conn.execute(
        "CREATE TABLE IF NOT EXISTS feedback (
            id TEXT PRIMARY KEY,
            character_name TEXT,
            server TEXT,
            is_anonymous INTEGER NOT NULL DEFAULT 0,
            rating_mechanics INTEGER NOT NULL CHECK (rating_mechanics >= 1 AND rating_mechanics <= 5),
            rating_damage INTEGER NOT NULL CHECK (rating_damage >= 1 AND rating_damage <= 5),
            rating_teamwork INTEGER NOT NULL CHECK (rating_teamwork >= 1 AND rating_teamwork <= 5),
            rating_communication INTEGER NOT NULL CHECK (rating_communication >= 1 AND rating_communication <= 5),
            rating_overall INTEGER NOT NULL CHECK (rating_overall >= 1 AND rating_overall <= 5),
            comments TEXT,
            content_type TEXT,
            player_job TEXT,
            ip_address TEXT NOT NULL,
            created_at TEXT NOT NULL
        )",
        [],
    )?;

    // Migration: Add player_job column if it doesn't exist (for existing databases)
    let _ = conn.execute("ALTER TABLE feedback ADD COLUMN player_job TEXT", []);

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_feedback_created_at ON feedback (created_at)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_feedback_ip_address ON feedback (ip_address)",
        [],
    )?;

    log::info!("Database initialized at {}", db_path);
    Ok(conn)
}

pub fn check_rate_limit(conn: &Connection, ip_address: &str, minutes: i64) -> Result<bool> {
    let cutoff = chrono::Utc::now() - chrono::Duration::minutes(minutes);
    let cutoff_str = cutoff.format("%Y-%m-%d %H:%M:%S").to_string();
    
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM feedback WHERE ip_address = ?1 AND created_at > ?2",
        [ip_address, &cutoff_str],
        |row| row.get(0),
    )?;
    
    Ok(count == 0) // true if allowed to submit
}
