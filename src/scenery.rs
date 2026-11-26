//! Prepared terrain overlays for highways, their towns, and real coastal anchorages.

use crate::airfield::{AirfieldMaterial, AirfieldSample, AirfieldSite};
use crate::alpine_lakes::{AlpineLake, AlpineSurface};
use crate::facilities::{FacilitySample, RegionalFacilities};
use crate::highway_links::HighwayLink;
use crate::ports::{CargoColor, Port, PortSample, PortSurface};
use crate::watershed::Watershed;
use crate::maritime::{ANCHORAGE_TILE, Anchorage, DeckMaterial};
use crate::roads::{ROAD_HALF_WIDTH, ROAD_REGION_SIZE, RoadNetwork};
use crate::settlement::{Settlement, SettlementSurface, Terminus};
use crate::world::{Sample, Surface, World};

mod expansion;
pub use expansion::{DistrictExpansion, DistrictExpansionRequest};

#[derive(Clone)]
pub struct DistrictRegion {
    region_x: i32,
    region_z: i32,
    road: Option<RoadNetwork>,
    towns: [Option<Settlement>; 2],
    bounds: [f32; 4],
    town_bounds: [[f32; 4]; 2],
}

impl DistrictRegion {
    pub fn build(world: &World, region_x: i32, region_z: i32) -> Self {
        let road = RoadNetwork::generate(world, region_x, region_z);
        let towns = match &road {
            Some(road) => [
                Settlement::generate(world, road, Terminus::Start),
                Settlement::generate(world, road, Terminus::End),
            ],
            None => [None, None],
        };
        if let Some(road) = road {
            return Self::from_prepared(region_x, region_z, road, towns);
        }
        Self { region_x, region_z, road: None, towns, bounds: [0.0; 4], town_bounds: [[0.0; 4]; 2] }
    }

    pub(crate) fn from_prepared(
        region_x: i32, region_z: i32, road: RoadNetwork, towns: [Option<Settlement>; 2],
    ) -> Self {
        let mut bounds = road.bounds();
        let town_bounds = towns.each_ref().map(|town| town.as_ref().map_or([0.0; 4], Settlement::bounds));
        for area in town_bounds.into_iter().zip(&towns).filter_map(|(area, town)| town.as_ref().map(|_| area)) {
            bounds = [bounds[0].min(area[0]), bounds[1].min(area[1]), bounds[2].max(area[2]), bounds[3].max(area[3])];
        }
        Self { region_x, region_z, road: Some(road), towns, bounds, town_bounds }
    }

    pub fn road(&self) -> Option<&RoadNetwork> {
        self.road.as_ref()
    }

    pub(crate) fn bounds(&self) -> [f32; 4] { self.bounds }

    pub fn towns(&self) -> impl Iterator<Item = &Settlement> {
        self.towns.iter().flatten()
    }

    pub fn town(&self, end: Terminus) -> Option<&Settlement> {
        let index = match end { Terminus::Start => 0, Terminus::End => 1 };
        self.towns[index].as_ref()
    }

    pub(crate) fn obstruction(&self, x: f32, z: f32) -> f32 {
        self.towns().map(|town| town.obstruction(x, z)).fold(0.0, f32::max)
    }

    pub fn overlay(&self, x: f32, z: f32, ground: Sample) -> Sample {
        if region_axis(x, ROAD_REGION_SIZE) != self.region_x
            || region_axis(z, ROAD_REGION_SIZE) != self.region_z
        {
            return ground;
        }
        self.overlay_local(x, z, ground)
    }

    pub(crate) fn overlay_local(&self, x: f32, z: f32, ground: Sample) -> Sample {
        self.overlay_local_with_road(x, z, ground, |road| {
            let paved = road.proximity(x, z)?;
            Some(if paved.distance_squared <= ROAD_HALF_WIDTH * ROAD_HALF_WIDTH {
                Surface::Pavement
            } else {
                Surface::Gravel
            })
        })
    }

    pub(crate) fn overlay_local_with_road(
        &self, x: f32, z: f32, ground: Sample, road_surface: impl FnOnce(&RoadNetwork) -> Option<Surface>,
    ) -> Sample {
        let [left, near, right, far] = self.bounds;
        if x < left || x > right || z < near || z > far { return ground; }
        let Some(road) = &self.road else { return ground };
        if let Some(surface) = road_surface(road) {
            return Sample { surface, ..ground };
        }
        for (town, [left, near, right, far]) in self.towns.iter().zip(self.town_bounds) {
            if x < left || x > right || z < near || z > far { continue; }
            let Some(town) = town else { continue };
            let detail = if matches!(ground.surface, Surface::Pavement | Surface::Gravel) {
                town.sample_transport_reservation(x, z)
            } else {
                town.sample(x, z)
            };
            let Some(detail) = detail else { continue };
            return cover_settlement(detail, ground);
        }
        ground
    }
}

pub(crate) fn cover_settlement(detail: crate::settlement::SettlementSample, ground: Sample) -> Sample {
    let surface = match detail.surface {
        SettlementSurface::Street => Surface::Pavement,
        SettlementSurface::Park => Surface::Park,
        SettlementSurface::WaterfrontPlaza => Surface::Gravel,
        SettlementSurface::RiverWalk => Surface::Boardwalk,
        SettlementSurface::ParkCanopy => Surface::TreeCanopy,
        SettlementSurface::PitchedRoof => Surface::PitchedRoof,
        SettlementSurface::FlatRoof => Surface::FlatRoof,
        SettlementSurface::TowerRoof => Surface::TowerRoof,
        SettlementSurface::IndustryBrick => Surface::IndustryBrick,
        SettlementSurface::IndustryConcrete => Surface::IndustryConcrete,
        SettlementSurface::IndustryMetal => Surface::IndustryMetal,
    };
    Sample { surface, elevation: detail.elevation.unwrap_or(ground.elevation) }
}

pub fn overlay_airfield(field: &AirfieldSite, x: f32, z: f32, ground: Sample) -> Sample {
    let Some(piece) = field.sample(x, z) else { return ground };
    cover_airfield(piece, ground)
}

fn cover_airfield(piece: AirfieldSample, ground: Sample) -> Sample {
    let surface = match piece.material {
        AirfieldMaterial::GrassRunway => Surface::GrassRunway,
        AirfieldMaterial::PavedRunway | AirfieldMaterial::Taxiway | AirfieldMaterial::AccessRoad => Surface::Pavement,
        AirfieldMaterial::Apron => Surface::Gravel,
        AirfieldMaterial::ApronStand => Surface::Pavement,
        AirfieldMaterial::ApronMarking => Surface::AirfieldGuide,
        AirfieldMaterial::RunwayMarking => Surface::RunwayMarking,
        AirfieldMaterial::EdgeLight => Surface::RunwayLight,
        AirfieldMaterial::HangarRoof => Surface::FlatRoof,
        AirfieldMaterial::PitchedHangarRoof => Surface::PitchedRoof,
        AirfieldMaterial::GlazedAirfieldRoof => Surface::TowerRoof,
    };
    Sample { elevation: piece.elevation.unwrap_or(ground.elevation), surface }
}

pub fn overlay_port(port: &Port, x: f32, z: f32, ground: Sample) -> Sample {
    let Some(piece) = port.sample(x, z, ground) else { return ground };
    cover_port(piece)
}

fn cover_port(piece: PortSample) -> Sample {
    let surface = match piece.surface {
        PortSurface::AccessRoad | PortSurface::Apron => Surface::Pavement,
        PortSurface::Quay => Surface::Gravel,
        PortSurface::Terminal | PortSurface::Warehouse => Surface::FlatRoof,
        PortSurface::Crane => Surface::HarborCrane,
        PortSurface::Cargo(CargoColor::Ochre) | PortSurface::Equipment(CargoColor::Ochre) => Surface::CargoOchre,
        PortSurface::Cargo(CargoColor::Blue) | PortSurface::Equipment(CargoColor::Blue) => Surface::CargoBlue,
        PortSurface::Cargo(CargoColor::Rust) | PortSurface::Equipment(CargoColor::Rust) => Surface::ShipCargo,
        PortSurface::Cargo(CargoColor::Pale) | PortSurface::Equipment(CargoColor::Pale) => Surface::ShipCabin,
        PortSurface::BoatHull => Surface::ShipHull,
        PortSurface::BoatDeck => Surface::ShipDeck,
        PortSurface::BoatCabin | PortSurface::BoatSail => Surface::ShipCabin,
        PortSurface::BoatCargo => Surface::ShipCargo,
    };
    Sample { elevation: piece.elevation, surface }
}

pub fn overlay_facilities(
    facilities: &RegionalFacilities, x: f32, z: f32, raw: Sample, developed: Sample,
) -> Sample {
    match facilities.sample(x, z, raw) {
        None => developed,
        Some(FacilitySample::Port(piece)) => {
            if developed.surface == Surface::Pavement && piece.surface == PortSurface::AccessRoad {
                developed
            } else {
                cover_port(piece)
            }
        }
        Some(FacilitySample::Airfield(piece)) => cover_airfield(piece, developed),
        Some(FacilitySample::Rail(piece)) => crate::rail_art::cover_rail(piece, developed),
        Some(FacilitySample::Quarry) => facilities.quarry().map_or(developed, |quarry| quarry.overlay_developed(x, z, raw, developed)),
        Some(FacilitySample::Solar(piece)) => crate::solar_sites::cover_sample(piece, developed),
        Some(FacilitySample::Quayside(piece)) => piece,
        Some(FacilitySample::ScenicTurnout(piece)) => {
            if developed.surface == Surface::Pavement { developed } else { piece }
        }
        Some(FacilitySample::WaterTower) => {
            let ground = if developed.surface.is_tree_canopy() { raw } else { developed };
            facilities.water_tower().map_or(developed, |tower| tower.overlay(x, z, ground))
        }
    }
}

pub fn overlay_watershed(watershed: &Watershed, x: f32, z: f32, ground: Sample) -> Sample {
    let Some(water) = watershed.sample(x, z) else { return ground };
    cover_river_sample(water, ground)
}

pub(crate) fn cover_river_sample(water: crate::watershed::RiverSample, ground: Sample) -> Sample {
    if water.is_channel() && ground.elevation > 0.0 {
        return Sample {
            elevation: water.level,
            surface: Surface::RiverWater,
        };
    }
    let bank_height = ground.elevation - water.level;
    if water.bank_strength >= 0.63 && (0.0..2.0).contains(&bank_height) {
        return Sample { surface: Surface::Gravel, ..ground };
    }
    if water.bank_strength >= 0.35 && (0.0..4.0).contains(&bank_height)
        && matches!(ground.surface, Surface::Grass)
    {
        return Sample { surface: Surface::WetMeadow, ..ground };
    }
    ground
}

pub fn overlay_alpine(lake: &AlpineLake, x: f32, z: f32, ground: Sample) -> Sample {
    let Some(cover) = lake.sample(x, z, ground) else { return ground };
    let surface = match cover.surface {
        AlpineSurface::Tarn => Surface::TarnWater,
        AlpineSurface::TarnShallows => Surface::TarnShallows,
        AlpineSurface::Meadow => Surface::AlpineMeadow,
        AlpineSurface::OutletGravel => Surface::Gravel,
        AlpineSurface::Shore(surface) => surface,
    };
    Sample { elevation: cover.elevation, surface }
}

pub fn connect_districts(
    world: &World, regions: [&DistrictRegion; 2], rivers: [&Watershed; 2],
) -> Option<HighwayLink> {
    HighwayLink::fit_avoiding(world, regions[0].road()?, regions[1].road()?, |x, z| {
        rivers.iter().any(|river| river.sample(x, z).is_some_and(|water| water.is_channel()))
            || regions.iter().flat_map(|region| region.towns()).any(|town| {
                town.sample(x, z).is_some_and(|detail| detail.elevation.is_some())
            })
    })
}

pub fn overlay_highway_link(link: &HighwayLink, x: f32, z: f32, ground: Sample) -> Sample {
    let Some(road) = link.proximity(x, z) else { return ground };
    Sample {
        surface: if road.distance_squared <= ROAD_HALF_WIDTH * ROAD_HALF_WIDTH {
            Surface::Pavement
        } else {
            Surface::Gravel
        },
        ..ground
    }
}

#[derive(Clone, Copy)]
struct VesselTile {
    x: i32,
    z: i32,
    ship: Option<Anchorage>,
}

impl VesselTile {
    const EMPTY: Self = Self { x: i32::MIN, z: i32::MIN, ship: None };
}

const ZONES: i32 = 7;

pub struct CoastalScenery {
    center: Option<(i32, i32)>,
    tiles: [[VesselTile; ZONES as usize]; ZONES as usize],
    world: World,
    seed: u64,
}

impl CoastalScenery {
    pub const fn new() -> Self {
        Self {
            center: None,
            tiles: [[VesselTile::EMPTY; ZONES as usize]; ZONES as usize],
            world: World::new(0),
            seed: 0,
        }
    }

    pub fn reset(&mut self, world: World, seed: u64, x: f32, z: f32) {
        self.world = world;
        self.seed = seed;
        self.center = None;
        self.tiles.fill([VesselTile::EMPTY; ZONES as usize]);
        self.follow(x, z);
    }

    pub fn follow(&mut self, x: f32, z: f32) {
        let center_x = region_axis(x, ANCHORAGE_TILE);
        let center_z = region_axis(z, ANCHORAGE_TILE);
        if self.center == Some((center_x, center_z)) {
            return;
        }
        self.center = Some((center_x, center_z));
        for tz in center_z - ZONES / 2..=center_z + ZONES / 2 {
            for tx in center_x - ZONES / 2..=center_x + ZONES / 2 {
                let tile = &mut self.tiles[wrap(tz)][wrap(tx)];
                if tile.x != tx || tile.z != tz {
                    let ship = Anchorage::generate(&self.world, self.seed, tx, tz);
                    *tile = VesselTile { x: tx, z: tz, ship };
                }
            }
        }
    }

    pub fn overlay(&self, x: f32, z: f32, ground: Sample) -> Sample {
        if !matches!(ground.surface, Surface::DeepWater | Surface::Shallows) {
            return ground;
        }
        let tx = region_axis(x, ANCHORAGE_TILE);
        let tz = region_axis(z, ANCHORAGE_TILE);
        let tile = &self.tiles[wrap(tz)][wrap(tx)];
        if tile.x != tx || tile.z != tz {
            return ground;
        }
        let Some(detail) = tile.ship.and_then(|ship| ship.sample(x, z)) else { return ground };
        let surface = match detail.material {
            DeckMaterial::Hull => Surface::ShipHull,
            DeckMaterial::Deck => Surface::ShipDeck,
            DeckMaterial::Bridge => Surface::ShipCabin,
            DeckMaterial::Cargo => Surface::ShipCargo,
        };
        Sample { elevation: detail.height, surface }
    }

    pub fn visible_anchorages(&self) -> impl Iterator<Item = Anchorage> + '_ {
        self.tiles.iter().flatten().filter_map(|tile| tile.ship)
    }
}

impl Default for CoastalScenery {
    fn default() -> Self {
        Self::new()
    }
}

fn wrap(index: i32) -> usize {
    index.rem_euclid(ZONES) as usize
}

pub(crate) fn region_axis(value: f32, tile: i32) -> i32 {
    let scaled = value * (1.0 / tile as f32);
    let integer = scaled as i32;
    integer - i32::from(scaled < integer as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn highway_shares_visible_pavement_with_real_termini() {
        let world = World::new(1);
        let region = DistrictRegion::build(&world, 3, -3);
        let road = region.road().unwrap();
        for point in road.polyline() {
            let natural = world.sample(point.x, point.z);
            let rendered = region.overlay(point.x, point.z, natural);
            assert_eq!(rendered.surface, Surface::Pavement);
            assert_eq!(rendered.elevation, natural.elevation);
        }
        assert!(region.towns().any(|town| town.building_count() > 10));
    }

    #[test]
    fn anchored_objects_stay_fixed_when_the_visible_tile_window_moves() {
        let world = World::new(1);
        let mut scenery = CoastalScenery::new();
        scenery.reset(world, 1, 450.0, 71.0);
        let before: std::vec::Vec<_> = scenery.visible_anchorages().collect();
        assert!(!before.is_empty());
        scenery.follow(550.0, 310.0);
        let after: std::vec::Vec<_> = scenery.visible_anchorages().collect();
        for ship in before.into_iter().filter(|ship| after.contains(ship)) {
            let sample = scenery.overlay(ship.x, ship.z, world.sample(ship.x, ship.z));
            assert!(sample.elevation > 1.0);
            assert!(matches!(sample.surface, Surface::ShipDeck | Surface::ShipCabin | Surface::ShipCargo));
        }
    }
}
