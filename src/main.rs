#[macro_use]
extern crate diesel;
extern crate dotenv;
extern crate persistent;

use dotenv::dotenv;
use std::env;

extern crate rand;
extern crate time;

use rand::distributions::{IndependentSample, Range};
use rand::Rng;
use std::thread;
use std::time::Duration;

pub mod consts;
pub mod database;
pub mod structs;
use consts::*;
use structs::*;

extern crate iron;
extern crate params;
extern crate router;

use iron::prelude::*;
use router::Router;

extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate serde_derive;

extern crate bodyparser;

use diesel::prelude::*;
use diesel::r2d2::ConnectionManager;

use database::schema::*;
use database::*;

use iron::typemap::Key;
use persistent::Read;
pub struct AppDb;
impl Key for AppDb {
    type Value = diesel::r2d2::Pool<ConnectionManager<PgConnection>>;
}

/// saves a new player position update into the database and returns the list of nearby players and zombies
fn post_update(req: &mut Request) -> IronResult<Response> {
    let uid = {
        let req_uid = req.extensions.get::<Router>().unwrap().find("uid").unwrap();
        &(*req_uid).to_string()
    };
    let uid_number;
    match uid.parse::<i64>() {
        Ok(id) => uid_number = id,
        _ => {
            return Ok(Response::with((
                iron::status::BadRequest,
                "error, you passed a non-number uid",
            )))
        }
    }

    let struct_body = req.get::<bodyparser::Struct<RootInterface>>();
    match struct_body {
        Ok(Some(struct_body)) => {
            let pool = req.get::<Read<AppDb>>().unwrap();
            let conn = pool.get().unwrap();

            let player_info = player_info::table
                .filter(player_info::playeruid.eq(uid_number))
                .load::<PlayerInfo>(&*conn)
                .expect("Error loading player_info");

            if player_info.is_empty() {
                let info = PlayerInfo {
                    playeruid: uid_number,
                    xp: 0,
                    health: 100,
                };
                diesel::insert_into(player_info::table)
                    .values(&info)
                    .execute(&*conn)
                    .expect("Error inserting player info");
            } else {
                println!("Player info already available for user #{}", uid_number);
            }

            let (lat, lon) = {
                let records: Vec<Records> = struct_body.records.clone();
                let last_record = records.last();
                let (lat, lon) = match last_record {
                    Some(x) => (x.lat, x.lon),
                    None => (0.0, 0.0),
                };

                (lat, lon)
            };

            let records: Vec<Records> = struct_body.records;
            for record in records {
                let health = {
                    if record.health < 0 {
                        match player_info.last() {
                            Some(x) => x.health,
                            None => 100,
                        }
                    } else {
                        record.health
                    }
                };
                let new_record = Location {
                    uid: uid_number,
                    timestamp: record.timestamp,
                    lat: record.lat,
                    lon: record.lon,
                    health,
                    appstate: record.appstate,
                };

                diesel::insert_into(locations::table)
                    .values(&new_record)
                    .on_conflict((locations::uid, locations::timestamp))
                    .do_nothing()
                    .execute(&*conn)
                    .expect("Error inserting location");

                let _update_player_info = diesel::update(player_info::table.filter(
                        player_info::playeruid.eq(uid_number),
                    )).set(player_info::health.eq(health)) //FIXME: this is obviously a bad idea because everyone can cheat
                        .execute(&*conn);
            }

            // TODO: item pickup if appstate = 0 (foreground)

            // accept engagements
            for engagement in struct_body.accepted_engagements {
                let _update_result = diesel::update(
                    engagements::table
                        .filter(engagements::active.eq(1))
                        .filter(engagements::accepted.eq(0))
                        .filter(engagements::playeruid.eq(uid_number))
                        .filter(engagements::zombieuid.eq(engagement)),
                ).set(engagements::accepted.eq(1))
                    .execute(&*conn);
            }

            // update engagement state
            for interaction in struct_body.interactions {
                if interaction.len() == 3 {
                    let id = interaction[0];
                    let zombiehealth = match interaction[1] as i32 {
                        n if n > ZOMBIE_MAX_HEALTH => ZOMBIE_MAX_HEALTH,
                        n if n < 0 => 0,
                        _ => interaction[1] as i32,
                    };
                    let playerhealth = match interaction[2] as i32 {
                        n if n > PLAYER_MAX_HEALTH => PLAYER_MAX_HEALTH,
                        n if n < 0 => 0,
                        _ => interaction[2] as i32,
                    };

                    let _ = diesel::update(
                        zombie_locations::table.filter(zombie_locations::uid.eq(id)),
                    ).set(zombie_locations::health.eq(zombiehealth))
                        .execute(&*conn);

                    let _ = diesel::update(locations::table.filter(locations::uid.eq(uid_number)))
                        .set(locations::health.eq(playerhealth))
                        .execute(&*conn);

                    if zombiehealth == 0 {
                        println!("zombie #{} defeated by player #{}", id, uid_number);
                    }
                    if playerhealth == 0 {
                        println!("player #{} defeated by zombie #{}", uid_number, id);
                    }

                    // FIME: this just disables this engagement, need to update the zombie or the player health according to the outcome

                    // FIXME: ADD timestampt to this query, because the same zombie could be reurrected and attack again... (not right now but maybe in the future)
                    let _update_result =
                        diesel::update(
                            engagements::table
                                .filter(engagements::active.eq(1))
                                .filter(engagements::playeruid.eq(uid_number))
                                .filter(engagements::zombieuid.eq(id)),
                        ).set((engagements::active.eq(0), engagements::accepted.eq(1)))
                            .execute(&*conn);
                }
            }

            if struct_body.player_interactions.len() == 2 {
                let id_str = &struct_body.player_interactions[0];
                let state_str = &struct_body.player_interactions[1];
                let id = id_str.parse::<i64>().unwrap();
                let state = state_str.parse::<i32>().unwrap();

                // set player engagements state
                let _update_result = diesel::update(
                    player_engagements::table
                        .filter(player_engagements::active.eq(1))
                        .filter(player_engagements::player1uid.eq(uid_number))
                        .filter(player_engagements::player2uid.eq(id)),
                ).set(player_engagements::state.eq(state))
                    .execute(&*conn);
            }

            // generate items and zombies before querying
            let zombie_conn = pool.get().unwrap();
            generate_zombies_around_location(&*zombie_conn, lat, lon);

            let item_conn = pool.get().unwrap();
            generate_items_around_location(&*item_conn, lat, lon);

            // TODO: do a zombie <-> player intersection check here so we have less latency
            let players_struct = query_nearby_players(req, lat, lon);

            let json = serde_json::to_string(&players_struct).expect("Couldn't serialize response");

            Ok(Response::with((iron::status::Ok, json)))
        }
        Ok(None) => {
            println!("No body");
            Ok(Response::with((iron::status::BadRequest, "no body")))
        }
        Err(err) => {
            println!("Error: {:?}", err);
            Ok(Response::with((iron::status::BadRequest, "error")))
        }
    }
}

/// returns a new latitude and longitude moved by the given distance
fn get_new_lat_lon(lat: f32, lon: f32, distance: f32) -> (f32, f32) {
    get_new_lat_lon_xy(lat, lon, distance, distance)
}

/// returns a new latitude and longitude moved by the given x and y distance
fn get_new_lat_lon_xy(lat: f32, lon: f32, x_distance: f32, y_distance: f32) -> (f32, f32) {
    use std::f32::consts::PI;
    const R_EARTH: f32 = 6378.0;
    let new_latitude = lat + (x_distance / R_EARTH) * (180.0 / PI);
    let new_longitude = lon + (y_distance / R_EARTH) * (180.0 / PI) / (lat * PI / 180.0).cos();
    (new_latitude, new_longitude)
}

/// returns a new latitude and longitude moved by the given distance in a given angle
fn get_new_lat_lon_with_angle(lat: f32, lon: f32, distance: f32, angle: f32) -> (f32, f32) {
    let x_distance = distance * (angle.to_radians().cos());
    let y_distance = distance * (angle.to_radians().sin());
    get_new_lat_lon_xy(lat, lon, x_distance, y_distance)
}

/// returns the nearby players for the requesting player in a 1x1km square
fn query_nearby_players(req: &mut Request, lat: f32, lon: f32) -> UpdateResponse {
    let (lat_max, lon_max) = get_new_lat_lon(lat, lon, 1.0); // positive distance meanss up&right
    let (lat_min, lon_min) = get_new_lat_lon(lat, lon, -1.0); // negative distance mease down&left

    let pool = req.get::<Read<AppDb>>().unwrap();
    let conn = pool.get().unwrap();
    let uid_str = req.extensions.get::<Router>().unwrap().find("uid").unwrap();

    let uid_number = uid_str.parse::<i64>().unwrap();
    let now = time::get_time();
    let now_ms = now.sec * 1000 + i64::from(now.nsec) / 1_000_000;
    let now_with_timeout = now_ms - PLAYER_TIMEOUT;
    // first query all players that updated in the last PLAYER_TIMEOUT seconds
    let results = locations::table
        .distinct_on(locations::uid)
        .order((locations::uid, locations::timestamp.desc()))
        .filter(locations::timestamp.gt(now_with_timeout))
        .filter(locations::uid.ne(uid_number))
        .filter(locations::lat.gt(lat_min))
        .filter(locations::lat.lt(lat_max))
        .filter(locations::lon.gt(lon_min))
        .filter(locations::lon.lt(lon_max))
        .load::<Location>(&*conn)
        .expect("Error loading locations");

    let mut players = Vec::new();

    for location in results {
        players.push(Player {
            uid: location.uid,
            lat: location.lat,
            lon: location.lon,
            timestamp: location.timestamp,
            health: location.health,
        });
    }

    // restrict zombies queried to the same area as players
    let zombies = zombie_locations::table
        .distinct_on(zombie_locations::uid)
        .order((zombie_locations::uid, zombie_locations::timestamp.desc()))
        .filter(zombie_locations::timestamp.gt(now_with_timeout)) // this should remove dead zombies after a while
        .filter(zombie_locations::lat.gt(lat_min))
        .filter(zombie_locations::lat.lt(lat_max))
        .filter(zombie_locations::lon.gt(lon_min))
        .filter(zombie_locations::lon.lt(lon_max))
        .load::<Zombie>(&*conn)
        .expect("Error loading zombies");

    let boundingbox: [LatLon; 2] = [
        LatLon {
            lat: BBMIN.0,
            lon: BBMIN.1,
        },
        LatLon {
            lat: BBMAX.0,
            lon: BBMAX.1,
        },
    ];
    let player_pos_min = get_new_lat_lon(lat, lon, -INTERACTION_AREA);
    let player_pos_max = get_new_lat_lon(lat, lon, INTERACTION_AREA);
    let player_boundingbox: [LatLon; 2] = [
        LatLon {
            lat: player_pos_min.0,
            lon: player_pos_min.1,
        },
        LatLon {
            lat: player_pos_max.0,
            lon: player_pos_max.1,
        },
    ];

    let engagements = engagements::table
        .filter(engagements::active.eq(1))
        .filter(engagements::playeruid.eq(uid_number))
        .load::<Engagement>(&*conn)
        .expect("Error loading engagements");

    // what we have to do is query all items in the greater area (not just the interaction area around the player), send those down to the player
    // now we also do an intersection check in the interacton area, set those items to the players uid
    // next we query the player items (which should now also contain the picked up items)
    // on the client we just check if the player_items object changes and show a corresponding graphic
    let checking_items = items::table
        .distinct_on(items::itemuid)
        .filter(items::lat.gt(lat_min))
        .filter(items::lat.lt(lat_max))
        .filter(items::lon.gt(lon_min))
        .filter(items::lon.lt(lon_max))
        .filter(items::owneruid.eq(-1))
        .order(items::itemuid)
        .load::<Item>(&*conn)
        .expect("Error loading items");

    for item in checking_items {
        if is_in_boundingbox(item.lat, item.lon, &player_boundingbox) {
            println!("item #{} getting picked up by player", item.itemuid);
            let _update_result = diesel::update(
                items::table.filter(items::itemuid.eq(item.itemuid)),
            ).set(items::owneruid.eq(uid_number))
                .execute(&*conn);
        }
    }

    let player_items = items::table
        .distinct_on(items::itemuid)
        .filter(items::owneruid.eq(uid_number))
        .order(items::itemuid)
        .load::<Item>(&*conn)
        .expect("Error loading player items"); // maybe     .limit(5)

    let items = items::table
        .distinct_on(items::itemuid)
        .filter(items::lat.gt(lat_min))
        .filter(items::lat.lt(lat_max))
        .filter(items::lon.gt(lon_min))
        .filter(items::lon.lt(lon_max))
        .filter(items::owneruid.eq(-1))
        .order(items::itemuid)
        .load::<Item>(&*conn)
        .expect("Error loading items");

    // TODO: doing the following in a single db-query would be a much better idea:
    let player_engagements = player_engagements::table
        .filter(player_engagements::active.eq(1))
        .filter(player_engagements::player1uid.eq(uid_number))
        .load::<PlayerEngagement>(&*conn)
        .expect("Error loading player engagements");

    let mut filtered_players = Vec::new();
    if player_engagements.len() >= 1 {
        for player in players {
            if uid_number == player_engagements[0].player1uid {
                filtered_players.push(player);
                break;
            }
        }
    }

    let player_engagements2 = player_engagements::table
        .filter(player_engagements::active.eq(1))
        .filter(player_engagements::player2uid.eq(uid_number))
        .load::<PlayerEngagement>(&*conn)
        .expect("Error loading player engagements");

    let mut combined_player_engagements = Vec::new();
    for player_engagement in player_engagements {
        combined_player_engagements.push(player_engagement);
    }
    for player_engagement in player_engagements2 {
        combined_player_engagements.push(player_engagement);
    }

    if combined_player_engagements.len() == 2 {
        // lets get player_info for both players here
        let player_info_player1 = player_info::table
            .filter(player_info::playeruid.eq(combined_player_engagements[0].player1uid))
            .load::<PlayerInfo>(&*conn)
            .expect("Error loading player_info_player1");
        let player_info_player2 = player_info::table
            .filter(player_info::playeruid.eq(combined_player_engagements[0].player2uid))
            .load::<PlayerInfo>(&*conn)
            .expect("Error loading player_info_player2");

        let p1xp = match player_info_player1.last() {
            Some(x) => x.xp,
            None => 0,
        };

        let p2xp = match player_info_player2.last() {
            Some(x) => x.xp,
            None => 0,
        };

        if combined_player_engagements[0].state == State::RunAway as i32
            || combined_player_engagements[1].state == State::RunAway as i32
        {
            // this means we or they ran away and the interaction has ended

            let (res1, res2) = if combined_player_engagements[0].state == State::RunAway as i32 {
                (State::WeRanAway, State::TheyRanAway)
            } else {
                (State::TheyRanAway, State::WeRanAway)
            };

            let _update_result1 = diesel::update(
                player_engagements::table
                    .filter(
                        player_engagements::player1uid
                            .eq(combined_player_engagements[0].player1uid),
                    )
                    .filter(
                        player_engagements::player2uid
                            .eq(combined_player_engagements[0].player2uid),
                    ),
            ).set((
                player_engagements::active.eq(0),
                player_engagements::state.eq(res1 as i32),
            ))
                .execute(&*conn);
            let _update_result2 = diesel::update(
                player_engagements::table
                    .filter(
                        player_engagements::player1uid
                            .eq(combined_player_engagements[1].player1uid),
                    )
                    .filter(
                        player_engagements::player2uid
                            .eq(combined_player_engagements[1].player2uid),
                    ),
            ).set((
                player_engagements::state.eq(res2 as i32),
            ))
                .execute(&*conn);
        } else if combined_player_engagements[0].state == State::Attack as i32
            && combined_player_engagements[1].state == State::Attack as i32
        {
            // we both attack
            // this means we need to display some sort of graphic until we get the result from the server

            // -> roll the dice here, one player gets some damage, the other a lot
            // one player gets some XP, the other a lot more

            // FIXME: take relative differences in xp into account and use some kind of random distribution
            let player1won;
            if p1xp > p2xp {
                // player 1 "wins"
                // TODO - get this result to the involved players (just abuse the state variable...)
                // FIXME
                println!("player 1 wins");
                player1won = true;
                let _update_xp =
                    diesel::update(player_info::table.filter(
                        player_info::playeruid.eq(combined_player_engagements[0].player1uid),
                    )).set(player_info::xp.eq(p1xp + PLAYER_XP_GAIN_ATTACK))
                        .execute(&*conn);
            } else if p1xp < p2xp {
                // player 2 "wins"
                println!("player 2 wins");
                player1won = false;
                let _update_xp =
                    diesel::update(player_info::table.filter(
                        player_info::playeruid.eq(combined_player_engagements[0].player2uid),
                    )).set(player_info::xp.eq(p2xp + PLAYER_XP_GAIN_ATTACK))
                        .execute(&*conn);
            } else {
                // roll the dice
                println!("both players have identical xp, roll the dice:");
                let mut rng = rand::thread_rng();
                if rng.gen_weighted_bool(2) {
                    //player 1 wins
                    println!("player 1 wins");
                    player1won = true;
                    let _update_xp = diesel::update(player_info::table.filter(
                        player_info::playeruid.eq(combined_player_engagements[0].player1uid),
                    )).set(player_info::xp.eq(p1xp + PLAYER_XP_GAIN_ATTACK))
                        .execute(&*conn);
                } else {
                    //player 2 wins
                    println!("player 2 wins");
                    player1won = false;
                    let _update_xp = diesel::update(player_info::table.filter(
                        player_info::playeruid.eq(combined_player_engagements[0].player2uid),
                    )).set(player_info::xp.eq(p2xp + PLAYER_XP_GAIN_ATTACK))
                        .execute(&*conn);
                }
            }
            if player1won {
                let _update_result = diesel::update(
                    player_engagements::table
                        .filter(
                            player_engagements::player1uid
                                .eq(combined_player_engagements[0].player1uid),
                        )
                        .filter(
                            player_engagements::player2uid
                                .eq(combined_player_engagements[0].player2uid),
                        )
                        .filter(player_engagements::active.eq(1)),
                ).set((
                    player_engagements::active.eq(0),
                    player_engagements::state.eq(State::WonFight as i32),
                ))
                    .execute(&*conn);

                let _update_result2 = diesel::update(
                    player_engagements::table
                        .filter(
                            player_engagements::player1uid
                                .eq(combined_player_engagements[1].player1uid),
                        )
                        .filter(
                            player_engagements::player2uid
                                .eq(combined_player_engagements[1].player2uid),
                        )
                        .filter(player_engagements::active.eq(1)),
                ).set((
                    //player_engagements::active.eq(0),
                    player_engagements::state.eq(State::LostFight as i32),
                ))
                    .execute(&*conn);

                // FIXME: modify the returned value - is this a good idea?
                combined_player_engagements[0].state = State::WonFight as i32;
                combined_player_engagements[1].state = State::LostFight as i32;
            } else {
                let _update_result = diesel::update(
                    player_engagements::table
                        .filter(
                            player_engagements::player1uid
                                .eq(combined_player_engagements[0].player1uid),
                        )
                        .filter(
                            player_engagements::player2uid
                                .eq(combined_player_engagements[0].player2uid),
                        )
                        .filter(player_engagements::active.eq(1)),
                ).set((
                    player_engagements::active.eq(0),
                    player_engagements::state.eq(State::LostFight as i32),
                ))
                    .execute(&*conn);

                let _update_result2 = diesel::update(
                    player_engagements::table
                        .filter(
                            player_engagements::player1uid
                                .eq(combined_player_engagements[1].player1uid),
                        )
                        .filter(
                            player_engagements::player2uid
                                .eq(combined_player_engagements[1].player2uid),
                        )
                        .filter(player_engagements::active.eq(1)),
                ).set((
                    //player_engagements::active.eq(0),
                    player_engagements::state.eq(State::WonFight as i32),
                ))
                    .execute(&*conn);
                combined_player_engagements[0].state = State::LostFight as i32;
                combined_player_engagements[1].state = State::WonFight as i32;
            }
        } else if combined_player_engagements[0].state == State::Befriend as i32
            && combined_player_engagements[1].state == State::Befriend as i32
        {
            // we both befriend
            // good for both of us, we both get some xp
            let _update_xp1 = diesel::update(
                player_info::table
                    .filter(player_info::playeruid.eq(combined_player_engagements[0].player1uid)),
            ).set(player_info::xp.eq(p1xp + PLAYER_XP_GAIN_BEFRIEND))
                .execute(&*conn);
            let _update_xp2 = diesel::update(
                player_info::table
                    .filter(player_info::playeruid.eq(combined_player_engagements[0].player2uid)),
            ).set(player_info::xp.eq(p2xp + PLAYER_XP_GAIN_BEFRIEND))
                .execute(&*conn);
            let _update_result = diesel::update(
                player_engagements::table
                    .filter(
                        player_engagements::player1uid
                            .eq(combined_player_engagements[0].player1uid),
                    )
                    .filter(
                        player_engagements::player2uid
                            .eq(combined_player_engagements[0].player2uid),
                    ),
            ).set((
                player_engagements::active.eq(0),
                player_engagements::state.eq(State::BothBefriended as i32),
            ))
                .execute(&*conn);

            let _update_result2 = diesel::update(
                player_engagements::table
                    .filter(
                        player_engagements::player1uid
                            .eq(combined_player_engagements[1].player1uid),
                    )
                    .filter(
                        player_engagements::player2uid
                            .eq(combined_player_engagements[1].player2uid),
                    ),
            ).set((
                //player_engagements::active.eq(0),
                player_engagements::state.eq(State::BothBefriended as i32),
            ))
                .execute(&*conn);
        } else if combined_player_engagements[0].state == State::Attack as i32
            && combined_player_engagements[1].state == State::Befriend as i32
        {
            // we attack, they befriend -> they made a mistake, they lose something? and we gain a little
            let _update_xp1 = diesel::update(
                player_info::table
                    .filter(player_info::playeruid.eq(combined_player_engagements[0].player1uid)),
            ).set(player_info::xp.eq(p1xp + PLAYER_XP_GAIN_ONESIDED_ATTACK))
                .execute(&*conn);
            let _update_result = diesel::update(
                player_engagements::table
                    .filter(
                        player_engagements::player1uid
                            .eq(combined_player_engagements[0].player1uid),
                    )
                    .filter(
                        player_engagements::player2uid
                            .eq(combined_player_engagements[0].player2uid),
                    ),
            ).set((
                player_engagements::active.eq(0),
                player_engagements::state.eq(State::RobbingSuccess as i32),
            ))
                .execute(&*conn);
            let _update_result2 = diesel::update(
                player_engagements::table
                    .filter(
                        player_engagements::player1uid
                            .eq(combined_player_engagements[1].player1uid),
                    )
                    .filter(
                        player_engagements::player2uid
                            .eq(combined_player_engagements[1].player2uid),
                    ),
            ).set((
                //player_engagements::active.eq(0),
                player_engagements::state.eq(State::Robbed as i32),
            ))
                .execute(&*conn);
        // they lose health and maybe some item:
        } else if combined_player_engagements[0].state == State::Befriend as i32
            && combined_player_engagements[1].state == State::Attack as i32
        {
            // they attack, we befriend -> we made a mistake, we lose something and they gain a little
            let _update_xp2 = diesel::update(
                player_info::table
                    .filter(player_info::playeruid.eq(combined_player_engagements[0].player2uid)),
            ).set(player_info::xp.eq(p2xp + PLAYER_XP_GAIN_ONESIDED_ATTACK))
                .execute(&*conn);

            // we lose health and maybe some item
            let _update_player_health = diesel::update(
                player_info::table.filter(player_info::playeruid.eq(uid_number)),
            ).set(player_info::health.eq(1))
                .execute(&*conn);

            let _update_result = diesel::update(
                player_engagements::table
                    .filter(
                        player_engagements::player1uid
                            .eq(combined_player_engagements[0].player1uid),
                    )
                    .filter(
                        player_engagements::player2uid
                            .eq(combined_player_engagements[0].player2uid),
                    )
                    .filter(player_engagements::active.eq(1)),
            ).set((
                player_engagements::active.eq(0),
                player_engagements::state.eq(State::Robbed as i32),
            ))
                .execute(&*conn);

            let _update_result2 = diesel::update(
                player_engagements::table
                    .filter(
                        player_engagements::player1uid
                            .eq(combined_player_engagements[1].player1uid),
                    )
                    .filter(
                        player_engagements::player2uid
                            .eq(combined_player_engagements[1].player2uid),
                    )
                    .filter(player_engagements::active.eq(1)),
            ).set((
                //player_engagements::active.eq(1),
                player_engagements::state.eq(State::RobbingSuccess as i32),
            ))
                .execute(&*conn);
        }
    }

    if combined_player_engagements.len() == 1 {
        println!("missig response received");
        if combined_player_engagements[0].player1uid == uid_number {
            println!("the response was meant for us ({})", uid_number);
            let _update_result = diesel::update(
                player_engagements::table
                    .filter(
                        player_engagements::player1uid
                            .eq(combined_player_engagements[0].player1uid),
                    )
                    .filter(
                        player_engagements::player2uid
                            .eq(combined_player_engagements[0].player2uid),
                    ),
            ).set(player_engagements::active.eq(0))
                .execute(&*conn);
        } else {
            println!(
                "the response was meant for player {}",
                combined_player_engagements[0].player1uid
            );
        }
    }

    if combined_player_engagements.is_empty() && engagements.is_empty() {
        println!("GIVING THE PLAYER SOME HEALTH BACK");
        // get the health and xp stats of the player here
        let player_info_up = player_info::table
            .filter(player_info::playeruid.eq(uid_number))
            .load::<PlayerInfo>(&*conn)
            .expect("Error loading player_info_player1");

        // TODO: whenever the player is not in an active engagement, either with a player or a zombie
        // give them the change to get some health back
        let new_health = {
            let mut health = 0;
            for player in player_info_up {
                health = player.health;
            }
            health += PLAYER_HEALTH_REGEN;
            if health > PLAYER_MAX_HEALTH {
                health = PLAYER_MAX_HEALTH;
            }
            health
        };

        let _update_player_result = diesel::update(
            player_info::table.filter(player_info::playeruid.eq(uid_number)),
        ).set(player_info::health.eq(new_health))
            .execute(&*conn);
    }

    let player_info = player_info::table
        .filter(player_info::playeruid.eq(uid_number))
        .load::<PlayerInfo>(&*conn)
        .expect("Error loading player_info_player1");

    UpdateResponse {
        players: filtered_players,
        zombies,
        boundingbox,
        player_boundingbox,
        engagements,
        items,
        player_items,
        player_engagements: combined_player_engagements,
        player_info,
    }
}

/// presents some debug information to the requesting party
fn get_home(req: &mut Request) -> IronResult<Response> {
    let pool = req.get::<Read<AppDb>>().unwrap();
    let conn = pool.get().unwrap();

    let now = time::get_time();
    let now_ms = now.sec * 1000 + i64::from(now.nsec) / 1_000_000;
    let now_with_timeout = now_ms - PLAYER_TIMEOUT;
    // first query all players that updated in the last PLAYER_TIMEOUT seconds
    let results = locations::table
        .distinct_on(locations::uid)
        .order((locations::uid, locations::timestamp.desc()))
        .filter(locations::timestamp.gt(now_with_timeout))
        .load::<Location>(&*conn)
        .expect("Error loading locations");

    let info = format!(
        "Disease Server Running\nActive players: {:?}",
        results.len()
    );

    Ok(Response::with((iron::status::Ok, info)))
}

/// checks if the given coordinate is inside the given boundingbox
fn is_in_boundingbox(lat: f32, lon: f32, boundingbox: &[LatLon; 2]) -> bool {
    lat > boundingbox[0].lat && lat < boundingbox[1].lat && lon > boundingbox[0].lon
        && lon < boundingbox[1].lon
}

/// Contains everything that needs to be executed at a regular "tick" interval.
/// This includes updating zombie positions, and intersection checks for engagements
fn tick(p: &diesel::r2d2::Pool<ConnectionManager<PgConnection>>) -> bool {
    let pool = p.clone();
    thread::spawn(move || loop {
        let conn = pool.get().unwrap();
        thread::sleep(Duration::from_secs(TICK));
        println!("tick");
        let now = time::get_time();
        let now_ms = now.sec * 1000 + i64::from(now.nsec) / 1_000_000;

        let zombies = zombie_locations::table
            .distinct_on(zombie_locations::uid)
            .order((zombie_locations::uid, zombie_locations::timestamp.desc()))
            .filter(zombie_locations::health.gt(0))
            .load::<Zombie>(&*conn)
            .expect("Error loading zombies");

        let mut updated_zombies: Vec<Zombie> = Vec::new();

        // now add a new location for every zombie
        for zombie in zombies {
            let (lat, lon) = get_new_lat_lon_with_angle(
                zombie.lat,
                zombie.lon,
                (ZOMBIE_SPEED * (TICK as f32)) / 1_000.0,
                zombie.bearing,
            );

            let boundingbox: [LatLon; 2] = [
                LatLon {
                    lat: BBMIN.0,
                    lon: BBMIN.1,
                },
                LatLon {
                    lat: BBMAX.0,
                    lon: BBMAX.1,
                },
            ];

            let engagements = engagements::table
                .filter(engagements::active.eq(1))
                .filter(engagements::zombieuid.eq(zombie.uid))
                .load::<Engagement>(&*conn)
                .expect("Error loading engagements");

            if !engagements.is_empty() {
                continue;
            }

            let in_bb = is_in_boundingbox(lat, lon, &boundingbox);
            if !in_bb {
                println!("outside of bounding box, stopping zombie and changing its bearing");
            }

            let updated_zombie: Zombie = Zombie {
                uid: zombie.uid,
                timestamp: now_ms,
                lat: {
                    if in_bb {
                        lat
                    } else {
                        zombie.lat
                    }
                },
                lon: {
                    if in_bb {
                        lon
                    } else {
                        zombie.lon
                    }
                },
                bearing: {
                    if in_bb {
                        zombie.bearing
                    } else {
                        let mut rng = rand::thread_rng();
                        let between = Range::new(0.0, 360.0);
                        between.ind_sample(&mut rng)
                    }
                },
                health: zombie.health,
            };

            let new_record = Zombie {
                uid: updated_zombie.uid,
                timestamp: updated_zombie.timestamp,
                lat: updated_zombie.lat,
                lon: updated_zombie.lon,
                bearing: updated_zombie.bearing,
                health: updated_zombie.health,
            };

            diesel::insert_into(zombie_locations::table)
                .values(&new_record)
                .execute(&*conn)
                .expect("tick() Error inserting into zombie_locations");

            updated_zombies.push(updated_zombie);
        }
        // get players

        let now_with_timeout = now_ms - PLAYER_TIMEOUT;
        // first query all players that updated in the last PLAYER_TIMEOUT seconds

        let players = locations::table
            .distinct_on(locations::uid)
            .order((locations::uid, locations::timestamp))
            .filter(locations::timestamp.gt(now_with_timeout))
            .load::<Location>(&*conn)
            .expect("Error loading locations");

        // check if zombie intersects with player?
        // if that is the case inform the player of this interaction in theor next update()
        // TODO -> notification would be better because we can send this message even to inactive players
        for player in &players {
            let player_pos_min = get_new_lat_lon(player.lat, player.lon, -INTERACTION_AREA);
            let player_pos_max = get_new_lat_lon(player.lat, player.lon, INTERACTION_AREA);
            let player_boundingbox: [LatLon; 2] = [
                LatLon {
                    lat: player_pos_min.0,
                    lon: player_pos_min.1,
                },
                LatLon {
                    lat: player_pos_max.0,
                    lon: player_pos_max.1,
                },
            ];

            for zombie in &updated_zombies {
                if is_in_boundingbox(zombie.lat, zombie.lon, &player_boundingbox.clone()) {
                    println!("zombie #{} in reach of player #{}!", zombie.uid, player.uid);

                    let zombie_engagements = engagements::table
                        .filter(engagements::active.eq(1))
                        .filter(engagements::zombieuid.eq(zombie.uid))
                        .load::<Engagement>(&*conn)
                        .expect("Error loading zombie engagements");

                    let player_engagements = engagements::table
                        .filter(engagements::active.eq(1))
                        .filter(engagements::playeruid.eq(player.uid))
                        .load::<Engagement>(&*conn)
                        .expect("Error loading player engagements");

                    if !zombie_engagements.is_empty() || !player_engagements.is_empty() {
                        println!("zombie or player already in active engagement");
                    } else {
                        println!("adding new engagement");
                        let new_engagement = Engagement {
                            playeruid: player.uid,
                            zombieuid: zombie.uid,
                            timestamp: now_ms,
                            active: 1,
                            accepted: 0,
                        };

                        diesel::insert_into(engagements::table)
                            .values(&new_engagement)
                            .execute(&*conn)
                            .expect("tick() Error inserting into engagements");

                        break; // break out of this loop so we only add the first zombie to an engagement
                    }
                }
            }

            for other_player in &players {
                if other_player.uid != player.uid
                    && is_in_boundingbox(
                        other_player.lat,
                        other_player.lon,
                        &player_boundingbox.clone(),
                    ) {
                    println!(
                        "other player #{} in reach of player #{}!",
                        other_player.uid, player.uid
                    );

                    let playerengagements = player_engagements::table
                        .filter(player_engagements::active.eq(1))
                        .filter(player_engagements::player1uid.eq(player.uid))
                        .filter(player_engagements::player2uid.eq(other_player.uid))
                        .load::<PlayerEngagement>(&*conn)
                        .expect("Error loading player<->player engagements");

                    if playerengagements.is_empty() {
                        // first get last engagement of both players:
                        let old_player_engagements = player_engagements::table
                            .filter(player_engagements::active.eq(0))
                            .filter(player_engagements::player1uid.eq(player.uid))
                            .filter(player_engagements::player2uid.eq(other_player.uid))
                            .order(player_engagements::timestamp.desc())
                            .limit(1)
                            .filter(
                                player_engagements::timestamp
                                    .gt(now_ms - PLAYER_INTERACTION_TIMEOUT),
                            )
                            .load::<PlayerEngagement>(&*conn)
                            .expect("Error loading player engagements");

                        if old_player_engagements.is_empty() {
                            println!("adding new player engagement");
                            let player1_engagement = PlayerEngagement {
                                player1uid: player.uid,
                                player2uid: other_player.uid,
                                timestamp: now_ms,
                                active: 1,
                                state: 0,
                            };

                            diesel::insert_into(player_engagements::table)
                                .values(&player1_engagement)
                                .execute(&*conn)
                                .expect("tick() Error inserting player1_engagement into player_engagements");

                            let player2_engagement = PlayerEngagement {
                                player1uid: other_player.uid,
                                player2uid: player.uid,
                                timestamp: now_ms,
                                active: 1,
                                state: 0,
                            };

                            diesel::insert_into(player_engagements::table)
                                .values(&player2_engagement)
                                .execute(&*conn)
                                .expect("tick() Error inserting player2_engagement into player_engagements");
                        } else {
                            println!("NOT adding new player engagement yet");
                        }

                        break;
                    } else {
                        println!("player already in engagement with another player");
                    }
                }
            }
        }
    }).join()
        .is_err() // FIXME? hack to respawn thread if diesel panics for some reason
}

/// generates up to `NUMBER_OF_ZOMBIES` zombies around a given location
/// but only if there are none in a 1x1km square around that location
fn generate_zombies_around_location(conn: &diesel::PgConnection, start_lat: f32, start_lon: f32) {
    let (lat_max, lon_max) = get_new_lat_lon(start_lat, start_lon, VISIBLE_AREA); // positive distance means up&right
    let (lat_min, lon_min) = get_new_lat_lon(start_lat, start_lon, -VISIBLE_AREA); // negative distance means down&left

    let now = time::get_time();
    let now_ms = now.sec * 1000 + i64::from(now.nsec) / 1_000_000;
    let now_with_timeout = now_ms - PLAYER_TIMEOUT;

    let results = zombie_locations::table
        .distinct_on(zombie_locations::uid)
        .order((zombie_locations::uid, zombie_locations::timestamp))
        .filter(zombie_locations::health.gt(0))
        .filter(zombie_locations::timestamp.gt(now_with_timeout)) // this should remove dead zombies after a while
        .filter(zombie_locations::lat.gt(lat_min))
        .filter(zombie_locations::lat.lt(lat_max))
        .filter(zombie_locations::lon.gt(lon_min))
        .filter(zombie_locations::lon.lt(lon_max))
        .load::<Zombie>(&*conn)
        .expect("Error loading zombie locations");

    if results.is_empty() {
        println!("there are no zombies around {} {}", start_lat, start_lon);
        println!(
            "this means we should insert {} now. but first get the newest zombie number",
            NUMBER_OF_ZOMBIES
        );
        let results_uid = zombie_locations::table
            .select(zombie_locations::uid)
            .distinct_on(zombie_locations::uid)
            .order(zombie_locations::uid.desc())
            .limit(1)
            .load::<i64>(&*conn)
            .expect("Error loading newest zombie uid");

        let start_uid = if results_uid.is_empty() {
            0
        } else {
            results_uid[0] + 1
        };

        let health = 100;
        let between = Range::new(0.0, 360.0);
        let loc_between = Range::new(-0.2_f32, 0.2_f32);
        let mut rng = rand::thread_rng();

        for uid_number in start_uid..(start_uid + NUMBER_OF_ZOMBIES) {
            let bearing = between.ind_sample(&mut rng);

            let (lat, lon) = get_new_lat_lon_xy(
                start_lat,
                start_lon,
                loc_between.ind_sample(&mut rng),
                loc_between.ind_sample(&mut rng),
            );

            let new_record = Zombie {
                uid: uid_number,
                timestamp: now_ms,
                lat,
                lon,
                health,
                bearing,
            };

            diesel::insert_into(zombie_locations::table)
                .values(&new_record)
                .execute(&*conn)
                .expect("Error inserting generated zombie");
        }
    }
}

/// generates up to `NUMBER_OF_ITEMS` items around a given location
/// but only if there are none in a 1x1km square around that location
fn generate_items_around_location(conn: &diesel::PgConnection, start_lat: f32, start_lon: f32) {
    let (lat_max, lon_max) = get_new_lat_lon(start_lat, start_lon, VISIBLE_AREA); // positive distance means up&right
    let (lat_min, lon_min) = get_new_lat_lon(start_lat, start_lon, -VISIBLE_AREA); // negative distance means down&left

    let results = items::table
        .distinct_on(items::itemuid)
        .order((items::itemuid, items::timestamp))
        .filter(items::lat.gt(lat_min))
        .filter(items::lat.lt(lat_max))
        .filter(items::lon.gt(lon_min))
        .filter(items::lon.lt(lon_max))
        .load::<Item>(&*conn)
        .expect("Error loading item locations");

    if results.is_empty() {
        println!("there are no items around {} {}", start_lat, start_lon);
        println!(
            "this means we should insert {} now. but first get the newest item number",
            NUMBER_OF_ITEMS
        );
        let results_uid = items::table
            .select(items::itemuid)
            .distinct_on(items::itemuid)
            .order(items::itemuid.desc())
            .limit(1)
            .load::<i64>(&*conn)
            .expect("Error loading newest item uid");

        let start_uid = if results_uid.is_empty() {
            0
        } else {
            results_uid[0] + 1
        };

        let now = time::get_time();
        let now_ms = now.sec * 1000 + i64::from(now.nsec) / 1_000_000;

        let itemtype = Range::new(0_i32, 2_i32);
        let loc_between = Range::new(-0.3_f32, 0.3_f32);
        let mut rng = rand::thread_rng();

        for uid_number in start_uid..(start_uid + NUMBER_OF_ITEMS) {
            let (lat, lon) = get_new_lat_lon_xy(
                start_lat,
                start_lon,
                loc_between.ind_sample(&mut rng),
                loc_between.ind_sample(&mut rng),
            );

            let new_record = Item {
                itemuid: uid_number,
                owneruid: -1,
                itemtype: itemtype.ind_sample(&mut rng),
                timestamp: now_ms,
                lat,
                lon,
            };

            diesel::insert_into(items::table)
                .values(&new_record)
                .execute(&*conn)
                .expect("Error inserting generated item");
        }
    }
}

fn main() {
    println!("started server");

    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let manager: ConnectionManager<PgConnection> =
        ConnectionManager::<PgConnection>::new(database_url);
    let pool: diesel::r2d2::Pool<ConnectionManager<PgConnection>> = diesel::r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create pool.");

    // FIXME: this is really hacky
    let p = pool.clone();
    thread::spawn(move || while tick(&p.clone()) {});

    let mut router = Router::new();

    router.post("/update/v2/:uid", post_update, "uid");
    router.get("/", get_home, "index");

    let mut middleware = Chain::new(router);
    middleware.link(Read::<AppDb>::both(pool));

    Iron::new(middleware).http(HOST).unwrap();
}
