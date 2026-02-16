use crate::models::Feedback;
use askama::Template;

#[derive(Clone)]
pub struct PlayerConfig {
    pub name: String,
    pub server: String,
    pub datacenter: String,
    pub banner_image: String,
    pub profile_image: String,
    pub tagline: String,
}

#[derive(Template)]
#[template(path = "index.html")]
pub struct IndexTemplate {
    pub player: PlayerConfig,
}

#[derive(Template)]
#[template(path = "success.html")]
pub struct SuccessTemplate {
    pub player: PlayerConfig,
}

#[derive(Template)]
#[template(path = "rate_limited.html")]
pub struct RateLimitedTemplate {
    pub player: PlayerConfig,
}

#[derive(Template)]
#[template(path = "rate_limited_hard.html")]
pub struct RateLimitedHardTemplate {
    pub player: PlayerConfig,
}

#[derive(Template)]
#[template(path = "admin_login.html")]
pub struct AdminLoginTemplate {}

#[derive(Template)]
#[template(path = "default_password_error.html")]
pub struct DefaultPasswordErrorTemplate {}

#[derive(Template)]
#[template(path = "admin.html")]
pub struct AdminTemplate {
    pub player: PlayerConfig,
    pub feedbacks: Vec<Feedback>,
    pub total_count: usize,
    pub avg_overall: f32,
}
