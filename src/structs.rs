use database::*;

// TODO: reuse structs defined in database::* (right now this is duplicated for most, not needed)

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Player {
    pub lat: f32,
    pub lon: f32,
    pub timestamp: i64,
    pub uid: i64,
    pub health: i32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LatLon {
    pub lat: f32,
    pub lon: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UpdateResponse {
    pub players: Vec<Player>,
    pub zombies: Vec<Zombie>,
    pub boundingbox: [LatLon; 2],
    pub player_boundingbox: [LatLon; 2],
    pub engagements: Vec<Engagement>,
    pub items: Vec<Item>,
    pub player_items: Vec<Item>,
    pub player_engagements: Vec<PlayerEngagement>,
    pub player_info: Vec<PlayerInfo>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Records {
    pub accuracy: f64,
    pub bearing: f32,
    pub lat: f32,
    pub lon: f32,
    pub speed: f32,
    pub timestamp: i64,
    pub health: i32,
    pub appstate: i32,
}

// FIXME: better game state object!!!
// Vec<Vec<i64>> should be replaced by a Vec of interaction and engagement structs...
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RootInterface {
    pub records: Vec<Records>,
    pub accepted_engagements: Vec<i64>,
    pub interactions: Vec<Vec<i64>>,
    pub player_interactions: Vec<String>, //other player ID, state (attack/run away/befriend)
}
