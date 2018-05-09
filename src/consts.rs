pub const PLAYER_TIMEOUT: i64 = 60 * 1000; // 60 seconds, timestamps are in milliseconds in this code

pub const PLAYER_INTERACTION_TIMEOUT: i64 = 60 * 1000;

pub const PLAYER_XP_GAIN_ATTACK: i32 = 100;
pub const PLAYER_XP_GAIN_BEFRIEND: i32 = 100;
pub const PLAYER_XP_GAIN_ONESIDED_ATTACK: i32 = 100;

pub const PLAYER_MAX_HEALTH: i32 = 100;
pub const ZOMBIE_MAX_HEALTH: i32 = 100;
pub const PLAYER_HEALTH_REGEN: i32 = 10;

pub enum State {
    Idle = 0,
    RunAway = 1,
    Attack = 2,
    Befriend = 3,
    BothBefriended = 42,
    WeRanAway = 100,
    TheyRanAway = 101,
    WonFight = 200,
    LostFight = 201,
    RobbingSuccess = 300,
    Robbed = 301,
}
pub const HOST: &str = "0.0.0.0:5000"; // deployment

pub const TICK: u64 = 5;

pub const ZOMBIE_SPEED: f32 = 2.0; // fast walking speed of 2m/s

pub const NUMBER_OF_ZOMBIES: i64 = 5;
pub const NUMBER_OF_ITEMS: i64 = 5;

// hfg (49.002376, 8.383714)

pub const BBMIN: (f32, f32) = (48.998171, 8.379272); // karlsruhe
pub const BBMAX: (f32, f32) = (49.004060, 8.387512); // karlsruhe

pub const INTERACTION_AREA: f32 = 0.1; // defines the square around the player/zombies in which interactions can happen
pub const VISIBLE_AREA: f32 = 0.25; // defines the square around the player in which items or zombies are visible
