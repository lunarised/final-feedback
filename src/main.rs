mod db;
mod handlers;
mod models;
mod templates;

use actix_files as fs;
use actix_web::{middleware, web, App, HttpServer};
use parking_lot::Mutex;
use std::env;
use std::sync::Arc;

use handlers::AppState;
use templates::PlayerConfig;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize logging
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    // Load environment variables from .env file if present
    // Use manifest directory to find .env from the crate root
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let env_file = std::path::PathBuf::from(manifest_dir).join(".env");
    if env_file.exists() {
        log::info!("Loading .env from: {}", env_file.display());
        match dotenvy::from_path(&env_file) {
            Ok(_) => log::info!(".env loaded successfully"),
            Err(e) => log::error!("Failed to load .env: {e:?}"),
        }
    } else {
        log::warn!(".env file not found at: {}", env_file.display());
        dotenvy::dotenv().ok();
    }

    // Configuration
    let host = env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let db_path = env::var("DATABASE_PATH").unwrap_or_else(|_| "feedback.db".to_string());
    let (admin_password, is_default_admin_password) = match env::var("ADMIN_PASSWORD") {
        Ok(pass) => (pass, false),
        Err(_) => {
            log::warn!(
                "ADMIN_PASSWORD not set, using default 'admin123' - CHANGE THIS IN PRODUCTION!"
            );
            ("admin123".to_string(), true)
        }
    };
    let discord_webhook_url = env::var("DISCORD_WEBHOOK_URL").ok();

    // Player configuration
    let player_name = env::var("PLAYER_NAME").unwrap_or_else(|_| "Your Character".to_string());
    let player_server = env::var("PLAYER_SERVER").unwrap_or_else(|_| "Server".to_string());
    let player_datacenter =
        env::var("PLAYER_DATACENTER").unwrap_or_else(|_| "Datacenter".to_string());
    let banner_image =
        env::var("BANNER_IMAGE").unwrap_or_else(|_| "/assets/banner.webp".to_string());
    let profile_image =
        env::var("PROFILE_IMAGE").unwrap_or_else(|_| "/assets/profile.webp".to_string());
    let tagline = env::var("TAGLINE")
        .unwrap_or_else(|_| "Ran content with me? Let me know how I did!".to_string());
    let rate_limit_minutes = env::var("RATE_LIMIT_MINUTES")
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(30);
    let ip_rate_limit_max = env::var("IP_RATE_LIMIT_MAX")
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(10);

    // Parse trusted proxy IPs (comma-separated)
    // Example: "127.0.0.1,192.168.1.1"
    let trusted_proxy_ips: Vec<String> = env::var("TRUSTED_PROXY_IPS")
        .unwrap_or_default()
        .split(',')
        .map(|ip| ip.trim().to_string())
        .filter(|ip| !ip.is_empty())
        .collect();

    let player = PlayerConfig {
        name: player_name,
        server: player_server,
        datacenter: player_datacenter,
        banner_image,
        profile_image,
        tagline,
    };

    if discord_webhook_url.is_some() {
        log::info!("Discord webhook notifications enabled");
    }

    if is_default_admin_password {
        log::error!("WARNING: Using default admin password! Admin panel will show error page until ADMIN_PASSWORD is set.");
    }

    if !trusted_proxy_ips.is_empty() {
        log::info!("Trusted proxy IPs: {}", trusted_proxy_ips.join(", "));
    } else {
        log::warn!("No trusted proxies configured - X-Forwarded-For headers will be ignored");
    }

    log::info!(
        "Player: {} @ {} ({})",
        player.name,
        player.server,
        player.datacenter
    );
    log::info!("Rate limit window: {rate_limit_minutes} minutes");

    // Initialize database
    let conn = db::init_database(&db_path).expect("Failed to initialize database");
    let db_pool = Arc::new(Mutex::new(conn));

    let bind_addr = format!("{}:{}", host, port);
    log::info!("Starting server at http://{}", bind_addr);
    log::info!("Admin panel available at http://{}/admin/panel", bind_addr);

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(AppState {
                db: db_pool.clone(),
                admin_password: admin_password.clone(),
                discord_webhook_url: discord_webhook_url.clone(),
                player: player.clone(),
                rate_limit_minutes,
                ip_rate_limit_max,
                trusted_proxy_ips: trusted_proxy_ips.clone(),
                is_default_admin_password,
            }))
            .wrap(middleware::Logger::default())
            .wrap(middleware::Compress::default())
            // Public routes
            .route("/", web::get().to(handlers::index))
            .route("/submit", web::post().to(handlers::submit_feedback))
            // Admin routes (not linked from main site)
            .route("/admin", web::get().to(handlers::admin_login))
            .route("/admin/panel", web::get().to(handlers::admin_panel))
            .route(
                "/admin/delete/{id}",
                web::delete().to(handlers::delete_feedback),
            )
            // Static assets
            .service(fs::Files::new("/assets", "src/assets").use_last_modified(true))
            .service(fs::Files::new("/static", "static").use_last_modified(true))
    })
    .bind(&bind_addr)?
    .run()
    .await
}
