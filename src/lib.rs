#![no_std]

#[cfg(test)]
extern crate std;

pub mod air;
pub mod airfield;
pub mod aircraft_art;
pub mod alpine_glaciers;
pub mod alpine_lakes;
pub mod beach;
pub mod bridge_silhouette;
pub mod cableways;
pub mod caprock_mesas;
pub mod climate;
mod coast_channel;
mod coastal_bluffs;
pub mod coastal_highways;
pub mod coastal_landforms;
pub mod controls;
mod cove_dunes;
pub mod crossings;
pub mod distant_roads;
pub mod distant_airfields;
#[cfg(feature = "ground-cover")]
pub mod desert_props;
pub mod dry_landmarks;
pub mod encounters;
pub mod facilities;
pub mod facility_stream;
pub mod feature_tiles;
pub mod flight;
pub mod floodplain;
pub mod glacial_moraines;
#[cfg(feature = "ground-cover")]
mod ground_cover;
pub mod ground_details;
pub mod highway_art;
pub mod highway_links;
pub mod highway_stream;
pub mod horizon_cache;
pub mod land_stream;
pub mod lighthouses;
pub mod local_routes;
pub mod lookout_art;
pub mod maritime;
pub mod natural_stream;
pub mod outwash_fans;
pub mod ports;
mod point_bars;
pub mod river_islands;

pub mod quayside;
pub mod rail;
pub mod rail_art;
pub mod rail_corridor;
pub mod rail_stream;
mod rail_station_art;
pub mod rail_survey;
pub mod render;
pub mod river_district;
pub mod river_stream;
pub mod roads;
pub mod scenery;
pub mod scenic_turnout;
pub mod scree_fans;
pub mod settlement;
pub mod sky;
pub mod skyline;
pub mod sky_events;
pub mod solar_sites;
pub mod summit_sites;
pub mod summit_stream;
mod summit_trail;
pub mod terrain_cache;
pub mod train_art;
pub mod train_follow;
pub mod tunnel_art;
pub mod tunnels;
pub mod ui;
pub mod working_quarry;
pub mod world;
pub mod world_mesh;
pub mod waterfall;
pub mod water_tower;
pub mod watershed;
pub mod weather_effects;

mod math;
