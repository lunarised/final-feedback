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

    // Create cookie tracking table for soft limit (1 per 30 mins per device)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS cookie_submissions (
            cookie_id TEXT PRIMARY KEY,
            submitted_at TEXT NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_cookie_submitted_at ON cookie_submissions (submitted_at)",
        [],
    )?;

    // Create IP attempt tracking table for rate limiting purposes
    conn.execute(
        "CREATE TABLE IF NOT EXISTS ip_attempts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            ip_address TEXT NOT NULL,
            attempted_at TEXT NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_ip_attempts_ip_address ON ip_attempts (ip_address)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_ip_attempts_attempted_at ON ip_attempts (attempted_at)",
        [],
    )?;

    // Clean up old cookie entries (older than 1 hour)
    let cutoff = chrono::Utc::now() - chrono::Duration::hours(1);
    let cutoff_str = cutoff.format("%Y-%m-%d %H:%M:%S").to_string();
    let _ = conn.execute(
        "DELETE FROM cookie_submissions WHERE submitted_at < ?1",
        [&cutoff_str],
    );

    // Clean up old IP attempts (older than 1 hour)
    let _ = conn.execute(
        "DELETE FROM ip_attempts WHERE attempted_at < ?1",
        [&cutoff_str],
    );

    log::info!("Database initialized at {}", db_path);
    Ok(conn)
}

pub enum RateLimitType {
    CookieSoftLimit,      // Same device, tried within 30 mins
    IpHardLimit,          // Same IP, 10+ submissions in last hour
}

pub fn check_rate_limits(
    conn: &Connection,
    ip_address: &str,
    cookie_id: &str,
    ip_limit_max: i64,
) -> Result<Option<RateLimitType>> {
    // Check IP hard limit first (includes both actual submissions and blocked attempts)
    let one_hour_ago = chrono::Utc::now() - chrono::Duration::hours(1);
    let cutoff_str = one_hour_ago.format("%Y-%m-%d %H:%M:%S").to_string();
    
    // Count actual submissions
    let submission_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM feedback WHERE ip_address = ?1 AND created_at > ?2",
        rusqlite::params![ip_address, &cutoff_str],
        |row| row.get(0),
    )?;
    
    // Count blocked attempts
    let attempt_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM ip_attempts WHERE ip_address = ?1 AND attempted_at > ?2",
        rusqlite::params![ip_address, &cutoff_str],
        |row| row.get(0),
    )?;
    
    let total_count = submission_count + attempt_count;
    
    if total_count >= ip_limit_max {
        return Ok(Some(RateLimitType::IpHardLimit));
    }
    
    // Check cookie soft limit (1 per 30 mins per device)
    let thirty_mins_ago = chrono::Utc::now() - chrono::Duration::minutes(30);
    let cutoff_str = thirty_mins_ago.format("%Y-%m-%d %H:%M:%S").to_string();
    
    let cookie_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM cookie_submissions WHERE cookie_id = ?1 AND submitted_at > ?2",
        rusqlite::params![cookie_id, &cutoff_str],
        |row| row.get(0),
    )?;
    
    if cookie_count > 0 {
        return Ok(Some(RateLimitType::CookieSoftLimit));
    }
    
    Ok(None) // No limits hit
}

pub fn record_submission(conn: &Connection, cookie_id: &str) -> Result<()> {
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    conn.execute(
        "INSERT OR REPLACE INTO cookie_submissions (cookie_id, submitted_at) VALUES (?1, ?2)",
        rusqlite::params![cookie_id, now],
    )?;
    Ok(())
}

pub fn record_ip_attempt(conn: &Connection, ip_address: &str) -> Result<()> {
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    conn.execute(
        "INSERT INTO ip_attempts (ip_address, attempted_at) VALUES (?1, ?2)",
        rusqlite::params![ip_address, now],
    )?;
    Ok(())
}
