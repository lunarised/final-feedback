use actix_web::{web, HttpRequest, HttpResponse, http::header};
use rusqlite::Connection;
use parking_lot::Mutex;
use std::sync::Arc;
use askama_actix::TemplateToResponse;
use serde_json::json;

use crate::models::{Feedback, FeedbackSubmission, is_valid_server};
use crate::db::{check_rate_limits, record_submission, record_ip_attempt, RateLimitType};
use crate::templates::{IndexTemplate, SuccessTemplate, RateLimitedTemplate, RateLimitedHardTemplate, AdminTemplate, AdminLoginTemplate, PlayerConfig};

pub type DbPool = Arc<Mutex<Connection>>;

pub struct AppState {
    pub db: DbPool,
    pub admin_password: String,
    pub discord_webhook_url: Option<String>,
    pub player: PlayerConfig,
    #[allow(dead_code)]
    pub rate_limit_minutes: i64,
    pub ip_rate_limit_max: i64,
    pub rate_limit_localhost: bool,
}

// Maximum allowed lengths for text fields to avoid unbounded DB growth
const MAX_CHAR_NAME: usize = 100;
const MAX_SERVER: usize = 50;
const MAX_COMMENTS: usize = 200;
const MAX_CONTENT_TYPE: usize = 100;
const MAX_PLAYER_JOB: usize = 100;

fn truncate_opt(input: Option<String>, max_chars: usize) -> Option<String> {
    input.and_then(|s| {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return None;
        }
        let cnt = trimmed.chars().count();
        let out = if cnt > max_chars {
            trimmed.chars().take(max_chars).collect::<String>()
        } else {
            trimmed.to_string()
        };
        Some(out)
    })
}

fn get_client_ip(req: &HttpRequest) -> String {
    // Check for forwarded headers first (for reverse proxies)
    if let Some(forwarded) = req.headers().get("X-Forwarded-For") {
        if let Ok(forwarded_str) = forwarded.to_str() {
            if let Some(ip) = forwarded_str.split(',').next() {
                return ip.trim().to_string();
            }
        }
    }
    
    if let Some(real_ip) = req.headers().get("X-Real-IP") {
        if let Ok(ip) = real_ip.to_str() {
            return ip.trim().to_string();
        }
    }
    
    // Fall back to connection info
    req.connection_info()
        .realip_remote_addr()
        .unwrap_or("unknown")
        .to_string()
}

pub async fn index(data: web::Data<AppState>) -> HttpResponse {
    let template = IndexTemplate {
        player: data.player.clone(),
    };
    template.to_response()
}

pub async fn submit_feedback(
    req: HttpRequest,
    data: web::Data<AppState>,
    form: web::Form<FeedbackSubmission>,
) -> HttpResponse {
    let ip_address = get_client_ip(&req);
    let conn = data.db.lock();
    
    // Generate or retrieve cookie ID
    let cookie_id = if let Some(cookie) = req.cookie("feedback_session") {
        cookie.value().to_string()
    } else {
        uuid::Uuid::new_v4().to_string()
    };
    
    // Check rate limits (skip for localhost if disabled)
    let should_rate_limit = data.rate_limit_localhost || (!ip_address.starts_with("127.") && ip_address != "localhost");
    
    if should_rate_limit {
    match check_rate_limits(&conn, &ip_address, &cookie_id, data.ip_rate_limit_max) {
        Ok(Some(limit_type)) => {
            match limit_type {
                RateLimitType::CookieSoftLimit => {
                    // Soft limit - same device, tried within 30 mins
                    // Record this as an IP attempt to count towards the hard limit
                    let _ = record_ip_attempt(&conn, &ip_address);
                    let template = RateLimitedTemplate { player: data.player.clone() };
                    return template.to_response();
                }
                RateLimitType::IpHardLimit => {
                    // Hard limit - too many submissions from this IP in the last hour
                    let template = RateLimitedHardTemplate { player: data.player.clone() };
                    return template.to_response();
                }
                }
            }
            Err(e) => {
                log::error!("Rate limit check failed: {}", e);
                return HttpResponse::InternalServerError().body("Database error");
            }
            Ok(None) => {} // No limits hit, continue
        }
    } // End should_rate_limit check
    
    // Validate ratings
    let ratings = [
        form.rating_mechanics,
        form.rating_damage,
        form.rating_teamwork,
        form.rating_communication,
        form.rating_overall,
    ];
    
    for rating in ratings {
        if !(1..=5).contains(&rating) {
            return HttpResponse::BadRequest().body("Invalid rating value");
        }
    }
    
    // Validate server if provided and not anonymous
    if !form.is_anonymous {
        if let Some(ref server) = form.server {
            if !server.is_empty() {
                if server.chars().count() > MAX_SERVER {
                    return HttpResponse::BadRequest().body("Invalid server name");
                }
                if !is_valid_server(server) {
                    return HttpResponse::BadRequest().body("Invalid server name");
                }
            }
        }
    }
    
    let id = uuid::Uuid::new_v4().to_string();
    let created_at = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    
    let (char_name, server) = if form.is_anonymous {
        (None, None)
    } else {
        (
            truncate_opt(form.character_name.clone(), MAX_CHAR_NAME),
            truncate_opt(form.server.clone(), MAX_SERVER),
        )
    };

    let comments = truncate_opt(form.comments.clone(), MAX_COMMENTS);
    let content_type = truncate_opt(form.content_type.clone(), MAX_CONTENT_TYPE);
    let player_job = truncate_opt(form.player_job.clone(), MAX_PLAYER_JOB);
    
    let result = conn.execute(
        "INSERT INTO feedback (id, character_name, server, is_anonymous, rating_mechanics, 
         rating_damage, rating_teamwork, rating_communication, rating_overall, comments, 
         content_type, player_job, ip_address, created_at) 
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        rusqlite::params![
            id,
            char_name.clone(),
            server.clone(),
            form.is_anonymous as i32,
            form.rating_mechanics,
            form.rating_damage,
            form.rating_teamwork,
            form.rating_communication,
            form.rating_overall,
            comments,
            content_type,
            player_job,
            ip_address,
            created_at,
        ],
    );
    
    match result {
        Ok(_) => {
            log::info!("New feedback submitted from IP: {}", ip_address);
            
            // Send Discord notification if webhook is configured
            if let Some(ref webhook_url) = data.discord_webhook_url {
                let webhook_url = webhook_url.clone();
                let feedback_data = DiscordFeedbackData {
                    character_name: char_name,
                    server,
                    is_anonymous: form.is_anonymous,
                    rating_mechanics: form.rating_mechanics,
                    rating_damage: form.rating_damage,
                    rating_teamwork: form.rating_teamwork,
                    rating_communication: form.rating_communication,
                    rating_overall: form.rating_overall,
                    comments: comments.clone(),
                    content_type: content_type.clone(),
                    player_job: player_job.clone(),
                };
                
                // Spawn async task to send webhook (don't block response)
                tokio::spawn(async move {
                    if let Err(e) = send_discord_notification(&webhook_url, feedback_data).await {
                        log::error!("Failed to send Discord notification: {}", e);
                    }
                });
            }
            
            // Record the cookie submission for soft limit tracking
            if let Err(e) = record_submission(&conn, &cookie_id) {
                log::error!("Failed to record cookie submission: {}", e);
            }
            
            let mut response = SuccessTemplate {
                player: data.player.clone(),
            }.to_response();
            
            // Set cookie with 1 hour expiration
            let cookie = format!(
                "feedback_session={}; Max-Age=3600; Path=/; HttpOnly; SameSite=Lax",
                cookie_id
            );
            if let Ok(header_value) = cookie.parse() {
                response.headers_mut().insert(header::SET_COOKIE, header_value);
            }
            
            response
        }
        Err(e) => {
            log::error!("Failed to insert feedback: {}", e);
            HttpResponse::InternalServerError().body("Failed to save feedback")
        }
    }
}

struct DiscordFeedbackData {
    character_name: Option<String>,
    server: Option<String>,
    is_anonymous: bool,
    rating_mechanics: i32,
    rating_damage: i32,
    rating_teamwork: i32,
    rating_communication: i32,
    rating_overall: i32,
    comments: Option<String>,
    content_type: Option<String>,
    player_job: Option<String>,
}

fn stars(rating: i32) -> String {
    "★".repeat(rating as usize) + &"☆".repeat((5 - rating) as usize)
}

async fn send_discord_notification(webhook_url: &str, data: DiscordFeedbackData) -> Result<(), reqwest::Error> {
    let client = reqwest::Client::new();
    
    // Build reviewer info
    let reviewer = if data.is_anonymous {
        "Anonymous".to_string()
    } else {
        match (&data.character_name, &data.server) {
            (Some(name), Some(server)) => format!("{} @ {}", name, server),
            (Some(name), None) => name.clone(),
            _ => "Unknown".to_string(),
        }
    };
    
    // Build context info
    let mut context_parts = Vec::new();
    if let Some(ref job) = data.player_job {
        context_parts.push(format!("**Job:** {}", job));
    }
    if let Some(ref content) = data.content_type {
        context_parts.push(format!("**Content:** {}", content));
    }
    let context = if context_parts.is_empty() {
        "Not specified".to_string()
    } else {
        context_parts.join(" | ")
    };
    
    // Calculate average rating
    let avg = (data.rating_mechanics + data.rating_damage + data.rating_teamwork 
        + data.rating_communication + data.rating_overall) as f32 / 5.0;
    
    // Determine embed color based on overall rating
    let color = match data.rating_overall {
        5 => 0x4CAF50, // Green
        4 => 0x8BC34A, // Light green
        3 => 0xFFC107, // Amber
        2 => 0xFF9800, // Orange
        _ => 0xF44336, // Red
    };
    
    // Build the embed
    let embed = json!({
        "embeds": [{
            "title": "📝 New Feedback Received!",
            "color": color,
            "fields": [
                {
                    "name": "👤 Reviewer",
                    "value": reviewer,
                    "inline": true
                },
                {
                    "name": "🎮 Context",
                    "value": context,
                    "inline": true
                },
                {
                    "name": "Overall Rating",
                    "value": format!("{} ({:.1}/5)", stars(data.rating_overall), avg),
                    "inline": true
                },
                {
                    "name": "Ratings Breakdown",
                    "value": format!(
                        "**Mechanics:** {}\n**Damage/Healing:** {}\n**Teamwork:** {}\n**Communication:** {}",
                        stars(data.rating_mechanics),
                        stars(data.rating_damage),
                        stars(data.rating_teamwork),
                        stars(data.rating_communication)
                    ),
                    "inline": false
                },
                {
                    "name": "Comments",
                    "value": data.comments
                        .filter(|c| !c.is_empty())
                        .map(|c| if c.len() > 500 { format!("{}...", &c[..500]) } else { c })
                        .unwrap_or_else(|| "_No comments provided_".to_string()),
                    "inline": false
                }
            ],
            "footer": {
                "text": "FinalFeedback - FFXIV Performance Survey"
            },
            "timestamp": chrono::Utc::now().to_rfc3339()
        }]
    });
    
    client.post(webhook_url)
        .json(&embed)
        .send()
        .await?;
    
    log::info!("Discord notification sent successfully");
    Ok(())
}

fn check_admin_auth(req: &HttpRequest, admin_password: &str) -> bool {
    if let Some(auth_header) = req.headers().get(header::AUTHORIZATION) {
        if let Ok(auth_str) = auth_header.to_str() {
            if auth_str.starts_with("Basic ") {
                let encoded = &auth_str[6..];
                if let Ok(decoded) = base64::Engine::decode(
                    &base64::engine::general_purpose::STANDARD,
                    encoded
                ) {
                    if let Ok(credentials) = String::from_utf8(decoded) {
                        // Format: username:password
                        if let Some((_user, pass)) = credentials.split_once(':') {
                            return pass == admin_password;
                        }
                    }
                }
            }
        }
    }
    false
}

pub async fn admin_login() -> HttpResponse {
    let template = AdminLoginTemplate {};
    template.to_response()
}

pub async fn admin_panel(
    req: HttpRequest,
    data: web::Data<AppState>,
) -> HttpResponse {
    if !check_admin_auth(&req, &data.admin_password) {
        return HttpResponse::Unauthorized()
            .insert_header((header::WWW_AUTHENTICATE, "Basic realm=\"Admin Panel\""))
            .body("Unauthorized");
    }
    
    let conn = data.db.lock();
    
    let mut stmt = match conn.prepare(
        "SELECT id, character_name, server, is_anonymous, rating_mechanics, rating_damage,
         rating_teamwork, rating_communication, rating_overall, comments, content_type,
         player_job, ip_address, created_at FROM feedback ORDER BY created_at DESC"
    ) {
        Ok(s) => s,
        Err(e) => {
            log::error!("Failed to prepare statement: {}", e);
            return HttpResponse::InternalServerError().body("Database error");
        }
    };
    
    let feedback_iter = stmt.query_map([], |row| {
        Ok(Feedback {
            id: row.get(0)?,
            character_name: row.get(1)?,
            server: row.get(2)?,
            is_anonymous: row.get::<_, i32>(3)? != 0,
            rating_mechanics: row.get(4)?,
            rating_damage: row.get(5)?,
            rating_teamwork: row.get(6)?,
            rating_communication: row.get(7)?,
            rating_overall: row.get(8)?,
            comments: row.get(9)?,
            content_type: row.get(10)?,
            player_job: row.get(11)?,
            ip_address: row.get(12)?,
            created_at: row.get(13)?,
        })
    });
    
    let feedbacks: Vec<Feedback> = match feedback_iter {
        Ok(iter) => iter.filter_map(|r| r.ok()).collect(),
        Err(e) => {
            log::error!("Failed to query feedback: {}", e);
            return HttpResponse::InternalServerError().body("Database error");
        }
    };
    
    let total_count = feedbacks.len();
    let avg_overall: f32 = if total_count > 0 {
        feedbacks.iter().map(|f| f.rating_overall as f32).sum::<f32>() / total_count as f32
    } else {
        0.0
    };
    
    let template = AdminTemplate {
        player: data.player.clone(),
        feedbacks,
        total_count,
        avg_overall,
    };
    
    template.to_response()
}

pub async fn delete_feedback(
    req: HttpRequest,
    data: web::Data<AppState>,
    path: web::Path<String>,
) -> HttpResponse {
    if !check_admin_auth(&req, &data.admin_password) {
        return HttpResponse::Unauthorized()
            .insert_header((header::WWW_AUTHENTICATE, "Basic realm=\"Admin Panel\""))
            .body("Unauthorized");
    }
    
    let id = path.into_inner();
    let conn = data.db.lock();
    
    match conn.execute("DELETE FROM feedback WHERE id = ?1", [&id]) {
        Ok(rows) => {
            if rows > 0 {
                log::info!("Deleted feedback: {}", id);
                HttpResponse::Ok().body("Deleted")
            } else {
                HttpResponse::NotFound().body("Feedback not found")
            }
        }
        Err(e) => {
            log::error!("Failed to delete feedback: {}", e);
            HttpResponse::InternalServerError().body("Failed to delete")
        }
    }
}
