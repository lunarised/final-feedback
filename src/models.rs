use serde::{Deserialize, Deserializer, Serialize};

fn deserialize_checkbox<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    // Checkboxes send "true" or "on" when checked, nothing when unchecked
    let opt: Option<String> = Option::deserialize(deserializer)?;
    Ok(opt
        .map(|s| s == "true" || s == "on" || s == "1")
        .unwrap_or(false))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FeedbackSubmission {
    pub character_name: Option<String>,
    pub server: Option<String>,
    #[serde(default, deserialize_with = "deserialize_checkbox")]
    pub is_anonymous: bool,
    pub rating_mechanics: i32,
    pub rating_damage: i32,
    pub rating_teamwork: i32,
    pub rating_communication: i32,
    pub rating_overall: i32,
    pub comments: Option<String>,
    pub content_type: Option<String>,
    pub player_job: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Feedback {
    pub id: String,
    pub character_name: Option<String>,
    pub server: Option<String>,
    pub is_anonymous: bool,
    pub rating_mechanics: i32,
    pub rating_damage: i32,
    pub rating_teamwork: i32,
    pub rating_communication: i32,
    pub rating_overall: i32,
    pub comments: Option<String>,
    pub content_type: Option<String>,
    pub player_job: Option<String>,
    pub ip_address: String,
    pub created_at: String,
}

impl Feedback {
    #[allow(dead_code)]
    pub fn average_rating(&self) -> f32 {
        (self.rating_mechanics
            + self.rating_damage
            + self.rating_teamwork
            + self.rating_communication
            + self.rating_overall) as f32
            / 5.0
    }
}

// FFXIV Server list for validation
pub const FFXIV_SERVERS: &[&str] = &[
    // NA - Aether
    "Adamantoise",
    "Cactuar",
    "Faerie",
    "Gilgamesh",
    "Jenova",
    "Midgardsormr",
    "Sargatanas",
    "Siren",
    // NA - Crystal
    "Balmung",
    "Brynhildr",
    "Coeurl",
    "Diabolos",
    "Goblin",
    "Malboro",
    "Mateus",
    "Zalera",
    // NA - Primal
    "Behemoth",
    "Excalibur",
    "Exodus",
    "Famfrit",
    "Hyperion",
    "Lamia",
    "Leviathan",
    "Ultros",
    // NA - Dynamis
    "Halicarnassus",
    "Maduin",
    "Marilith",
    "Seraph",
    "Cuchulainn",
    "Golem",
    "Kraken",
    "Rafflesia",
    // EU - Chaos
    "Cerberus",
    "Louisoix",
    "Moogle",
    "Omega",
    "Phantom",
    "Ragnarok",
    "Sagittarius",
    "Spriggan",
    // EU - Light
    "Alpha",
    "Lich",
    "Odin",
    "Phoenix",
    "Raiden",
    "Shiva",
    "Twintania",
    "Zodiark",
    // JP - Elemental
    "Aegis",
    "Atomos",
    "Carbuncle",
    "Garuda",
    "Gungnir",
    "Kujata",
    "Tonberry",
    "Typhon",
    // JP - Gaia
    "Alexander",
    "Bahamut",
    "Durandal",
    "Fenrir",
    "Ifrit",
    "Ridill",
    "Tiamat",
    "Ultima",
    // JP - Mana
    "Anima",
    "Asura",
    "Chocobo",
    "Hades",
    "Ixion",
    "Masamune",
    "Pandaemonium",
    "Titan",
    // JP - Meteor
    "Belias",
    "Mandragora",
    "Ramuh",
    "Shinryu",
    "Unicorn",
    "Valefor",
    "Yojimbo",
    "Zeromus",
    // OCE - Materia
    "Bismarck",
    "Ravana",
    "Sephirot",
    "Sophia",
    "Zurvan",
];

pub fn is_valid_server(server: &str) -> bool {
    FFXIV_SERVERS
        .iter()
        .any(|&s| s.eq_ignore_ascii_case(server))
}
