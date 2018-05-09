pub mod schema {
    table! {
        locations (uid, timestamp) {
            uid -> BigInt,
            timestamp -> BigInt,
            lat -> Float,
            lon -> Float,
            health -> Integer,
            appstate -> Integer,
        }
    }

    table! {
        zombie_locations (uid, timestamp) {
            uid -> BigInt,
            timestamp -> BigInt,
            lat -> Float,
            lon -> Float,
            health -> Integer,
            bearing -> Float,
        }
    }

    table! {
        engagements (playeruid, zombieuid, timestamp) {
            playeruid -> BigInt,
            zombieuid -> BigInt,
            timestamp -> BigInt,
            active -> Integer,
            accepted -> Integer,
        }
    }

    table! {
        player_engagements (player1uid, player2uid, timestamp) {
            player1uid -> BigInt,
            player2uid -> BigInt,
            timestamp -> BigInt,
            active -> Integer,
            state -> Integer,
        }
    }

    table! {
        items (itemuid) {
            itemuid -> BigInt,
            owneruid -> BigInt,
            itemtype -> Integer,
            timestamp -> BigInt,
            lat -> Float,
            lon -> Float,
        }
    }

    table! {
        player_info (playeruid) {
            playeruid -> BigInt,
            xp -> Integer,
            health -> Integer,
        }
    }

    allow_tables_to_appear_in_same_query!(locations, zombie_locations, player_engagements);
}

use self::schema::*;

#[derive(Queryable, Insertable, Identifiable, QueryableByName, Debug)]
#[primary_key(uid, timestamp)]
#[table_name = "locations"]
pub struct Location {
    pub uid: i64,
    pub timestamp: i64,
    pub lat: f32,
    pub lon: f32,
    pub health: i32,
    pub appstate: i32,
}

#[derive(Queryable, Insertable, Identifiable, QueryableByName, Serialize, Deserialize, Clone,
         Debug)]
#[primary_key(uid, timestamp)]
#[table_name = "zombie_locations"]
pub struct Zombie {
    pub uid: i64,
    pub timestamp: i64,
    pub lat: f32,
    pub lon: f32,
    pub health: i32,
    pub bearing: f32,
}

#[derive(Serialize, Deserialize, Clone, Queryable, Insertable, Identifiable, QueryableByName,
         Debug)]
#[primary_key(playeruid, zombieuid, timestamp)]
#[table_name = "engagements"]
pub struct Engagement {
    pub playeruid: i64,
    pub zombieuid: i64,
    pub timestamp: i64,
    pub active: i32,
    pub accepted: i32,
}

#[derive(Serialize, Deserialize, Clone, Queryable, Insertable, Identifiable, QueryableByName,
         Debug)]
#[primary_key(player1uid, player2uid, timestamp)]
#[table_name = "player_engagements"]
pub struct PlayerEngagement {
    pub player1uid: i64,
    pub player2uid: i64,
    pub timestamp: i64,
    pub active: i32,
    pub state: i32,
}

#[derive(Serialize, Deserialize, Clone, Queryable, Insertable, Identifiable, QueryableByName,
         Debug)]
#[primary_key(itemuid)]
#[table_name = "items"]
pub struct Item {
    pub itemuid: i64,
    pub owneruid: i64,
    pub itemtype: i32,
    pub timestamp: i64,
    pub lat: f32,
    pub lon: f32,
}

#[derive(Serialize, Deserialize, Clone, Queryable, Insertable, Identifiable, QueryableByName,
         Debug)]
#[primary_key(playeruid)]
#[table_name = "player_info"]
pub struct PlayerInfo {
    pub playeruid: i64,
    pub xp: i32,
    pub health: i32,
}
