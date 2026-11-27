//! Seeded terrain and surface materials in world coordinates.

pub const SEA_LEVEL: f32 = 0.0;
// The published UTM 11N origins, in metres relative to Downtown Los Angeles.
pub const LAS_VEGAS_ATLAS_ORIGIN: [f32; 2] = [279_792.0, 229_511.0];
mod atlas_ground;
mod atlas_roads;
mod atlas_towns;
pub mod atlas_ground_tiles;
pub use atlas_ground::AtlasGround;
pub use atlas_roads::AtlasRoads;
pub use atlas_towns::AtlasTowns;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum Surface {
    DeepWater,
    Shallows,
    Sand,
    Grass,
    Forest,
    Rock,
    Snow,
    Desert,
    Pavement,
    Gravel,
    Park,
    TreeCanopy,
    FieldGreen,
    FieldGold,
    FieldSoil,
    PitchedRoof,
    FlatRoof,
    TowerRoof,
    ShipHull,
    ShipDeck,
    ShipCabin,
    ShipCargo,
    HarborCrane,
    CargoOchre,
    CargoBlue,
    RiverWater,
    WetMeadow,
    GrassRunway,
    RunwayMarking,
    RunwayLight,
    TarnWater,
    TarnShallows,
    AlpineMeadow,
    SaltCrust,
    DuneSand,
    ReedMarsh,
    GlacierIce,
    GlacierCrevasse,
    GlacierMoraine,
    GlacierMeltwater,
    SolarGlass,
    Boardwalk,
    RiverSilt,
    ScreeEdge,
    ScreeFine,
    ScreeCoarse,
    DryWash,
    SpruceCanopy,
    AspenCanopy,
    MapleCanopy,
    JuniperCanopy,
    RailTimber,
    RailMetal,
    RailGlass,
    RailRust,
    RailApron,
    IndustryBrick,
    IndustryConcrete,
    IndustryMetal,
    MesaCap,
    MesaCliff,
    MesaApron,
    AirfieldGuide,
    LandmarkRoof01,
    LandmarkRoof02,
    LandmarkRoof03,
    LandmarkRoof04,
    LandmarkRoof05,
    LandmarkRoof06,
    LandmarkRoof07,
    LandmarkRoof08,
    LandmarkRoof09,
    LandmarkRoof10,
    LandmarkRoof11,
    LandmarkRoof12,
    LandmarkRoof13,
    LandmarkRoof14,
    LandmarkRoof15,
    LandmarkRoof16,
    LandmarkRoof17,
    LandmarkRoof18,
    LandmarkRoof19,
    LandmarkRoof20,
    LandmarkRoof21,
    LandmarkRoof22,
    LandmarkRoof23,
    LandmarkRoof24,
    LandmarkRoof25,
    LandmarkRoof26,
    LandmarkRoof27,
    LandmarkRoof28,
    LandmarkRoof29,
    LandmarkRoof30,
    LandmarkRoof31,
    LandmarkRoof32,
    LandmarkRoof33,
    LandmarkRoof34,
    LandmarkRoof35,
    LandmarkRoof36,
    LandmarkRoof37,
    LandmarkRoof38,
    LandmarkRoof39,
    LandmarkRoof40,
    LandmarkRoof41,
    LandmarkRoof42,
    LandmarkRoof43,
    LandmarkRoof44,
    LandmarkRoof45,
    LandmarkRoof46,
    LandmarkRoof47,
    LandmarkRoof48,
    LandmarkRoof49,
    LandmarkRoof50,
    UrbanLot,
    Mojave,
    SphereShell,
    PyramidGlass,
    VegasLimestone,
    VegasIvory,
    VegasRose,
    VegasSlateGlass,
    VegasBronzeGlass,
    VegasTerracotta,
    VegasCreamSlate,
    VegasGoldGlass,
    VegasStucco,
    VegasConcrete,
}

impl Surface {
    pub(crate) const LANDMARK_ROOFS: [Self; 50] = [
        Self::LandmarkRoof01,
        Self::LandmarkRoof02,
        Self::LandmarkRoof03,
        Self::LandmarkRoof04,
        Self::LandmarkRoof05,
        Self::LandmarkRoof06,
        Self::LandmarkRoof07,
        Self::LandmarkRoof08,
        Self::LandmarkRoof09,
        Self::LandmarkRoof10,
        Self::LandmarkRoof11,
        Self::LandmarkRoof12,
        Self::LandmarkRoof13,
        Self::LandmarkRoof14,
        Self::LandmarkRoof15,
        Self::LandmarkRoof16,
        Self::LandmarkRoof17,
        Self::LandmarkRoof18,
        Self::LandmarkRoof19,
        Self::LandmarkRoof20,
        Self::LandmarkRoof21,
        Self::LandmarkRoof22,
        Self::LandmarkRoof23,
        Self::LandmarkRoof24,
        Self::LandmarkRoof25,
        Self::LandmarkRoof26,
        Self::LandmarkRoof27,
        Self::LandmarkRoof28,
        Self::LandmarkRoof29,
        Self::LandmarkRoof30,
        Self::LandmarkRoof31,
        Self::LandmarkRoof32,
        Self::LandmarkRoof33,
        Self::LandmarkRoof34,
        Self::LandmarkRoof35,
        Self::LandmarkRoof36,
        Self::LandmarkRoof37,
        Self::LandmarkRoof38,
        Self::LandmarkRoof39,
        Self::LandmarkRoof40,
        Self::LandmarkRoof41,
        Self::LandmarkRoof42,
        Self::LandmarkRoof43,
        Self::LandmarkRoof44,
        Self::LandmarkRoof45,
        Self::LandmarkRoof46,
        Self::LandmarkRoof47,
        Self::LandmarkRoof48,
        Self::LandmarkRoof49,
        Self::LandmarkRoof50,
    ];

    pub(crate) fn building_palette(self) -> Option<&'static crate::settlement::building_palettes::BuildingPalette> {
        let id = (self as usize).wrapping_sub(Self::LandmarkRoof01 as usize);
        if id < Self::LANDMARK_ROOFS.len() {
            Some(crate::settlement::building_palettes::from_id(id))
        } else {
            self.vegas_facade().map(|facade| &facade.palette)
        }
    }

    pub(crate) fn vegas_facade(self) -> Option<&'static crate::settlement::building_palettes::VegasFacade> {
        use crate::settlement::building_palettes::VEGAS;
        let id = (self as usize).wrapping_sub(Self::VegasLimestone as usize);
        VEGAS.get(id)
    }

    /// Sandstone pigments shared by the scenery renderer and terrain artwork.
    pub fn sandstone_color(self) -> Option<[u8; 3]> {
        match self {
            Self::MesaCap => Some([183, 143, 104]),
            Self::MesaCliff => Some([181, 111, 78]),
            Self::MesaApron => Some([158, 126, 95]),
            _ => None,
        }
    }

    /// Base daylight color used for the same foliage in gameplay and in the artwork atlas.
    pub const fn canopy_color(self) -> Option<[u8; 3]> {
        match self {
            Self::TreeCanopy => Some([61, 117, 55]),
            Self::SpruceCanopy => Some([35, 81, 64]),
            Self::AspenCanopy => Some([166, 153, 64]),
            Self::MapleCanopy => Some([159, 79, 48]),
            Self::JuniperCanopy => Some([77, 119, 117]),
            _ => None,
        }
    }

    pub const fn is_tree_canopy(self) -> bool {
        self.canopy_color().is_some()
    }

    pub(crate) const fn is_industry(self) -> bool {
        matches!(self, Self::IndustryBrick | Self::IndustryConcrete | Self::IndustryMetal)
    }

    pub(crate) const fn is_station_roof(self) -> bool {
        matches!(self, Self::RailTimber | Self::RailMetal | Self::RailGlass | Self::RailRust)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sample {
    pub elevation: f32,
    pub surface: Surface,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct World {
    key: [u32; 2],
    terrain: Terrain,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Terrain {
    Procedural(bool),
    Surveyed(&'static crate::LasVegasData),
    Atlas { maps: &'static AtlasMaps, metropolitan: bool },
}

#[derive(Debug, Eq, PartialEq)]
pub struct AtlasMaps {
    la: &'static crate::LasVegasData,
    vegas: &'static crate::LasVegasData,
    ground: Option<AtlasTopography>,
    roads: Option<AtlasRoads>,
    towns: Option<AtlasTowns>,
}

/// Surveyed elevation for targets that keep only nearby tiles in memory.
pub trait AtlasElevation: Sync {
    /// Signed height in metres and a weight for blending at the survey boundary.
    fn sample(&self, x: f32, z: f32) -> Option<(f32, f32)>;

    /// Conservative height for planning a route before its tiles are loaded.
    fn safety_height(&self, x: f32, z: f32) -> Option<f32> {
        self.sample(x, z).map(|(height, _)| height)
    }
}

enum AtlasTopography {
    Grid(AtlasGround),
    Streamed(&'static dyn AtlasElevation),
}

impl AtlasTopography {
    fn sample(&self, x: f32, z: f32) -> Option<(f32, f32)> {
        match self {
            Self::Grid(ground) => ground.sample(x, z),
            Self::Streamed(ground) => ground.sample(x, z),
        }
    }
}

impl core::fmt::Debug for AtlasTopography {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self { Self::Grid(_) => "Grid", Self::Streamed(_) => "Streamed" })
    }
}

impl PartialEq for AtlasTopography {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Grid(a), Self::Grid(b)) => a == b,
            (Self::Streamed(a), Self::Streamed(b)) => core::ptr::eq(*a, *b),
            _ => false,
        }
    }
}

impl Eq for AtlasTopography {}

impl AtlasMaps {
    pub fn new(la: &'static crate::LasVegasData, vegas: &'static crate::LasVegasData, ground: Option<AtlasGround>) -> Self {
        debug_assert_eq!(la.city(), crate::MappedCity::LosAngeles);
        debug_assert_eq!(vegas.city(), crate::MappedCity::LasVegas);
        Self { la, vegas, ground: ground.map(AtlasTopography::Grid), roads: None, towns: None }
    }

    pub fn with_streamed_ground(mut self, ground: &'static dyn AtlasElevation) -> Self {
        self.ground = Some(AtlasTopography::Streamed(ground));
        self
    }

    pub fn with_roads(mut self, roads: Option<AtlasRoads>) -> Self {
        self.roads = roads;
        self
    }

    pub fn with_towns(mut self, towns: AtlasTowns) -> Self {
        self.towns = Some(towns);
        self
    }
}

impl World {
    pub const fn new(seed: u64) -> Self {
        let mut key = seed.wrapping_add(0x9e37_79b9_7f4a_7c15);
        key = (key ^ (key >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        key = (key ^ (key >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        key ^= key >> 31;
        Self {
            key: [key as u32, (key >> 32) as u32],
            terrain: Terrain::Procedural(false),
        }
    }

    /// Most seeds use the ordinary terrain. A minority open inside a large,
    /// continuous city, with ordinary wilderness beyond the basin.
    pub fn for_flight_seed(seed: u64) -> Self {
        let mut world = Self::new(seed);
        world.terrain = Terrain::Procedural(crate::math::mix(seed ^ 0x6279_97e3) % 8 == 0);
        world
    }

    /// Fixed map geography. Flight seeds still vary the sky and encounters.
    pub fn mapped(data: &'static crate::LasVegasData) -> Self {
        let mut world = Self::new(0x4c61_7356_6567_6173);
        world.terrain = Terrain::Surveyed(data);
        world
    }

    pub fn las_vegas(data: &'static crate::LasVegasData) -> Self {
        Self::mapped(data)
    }

    pub fn atlas(seed: u64, maps: &'static AtlasMaps) -> Self {
        let mut world = Self::for_flight_seed(seed);
        world.terrain = Terrain::Atlas { maps, metropolitan: world.metropolitan() };
        world
    }

    pub fn is_atlas(&self) -> bool { matches!(self.terrain, Terrain::Atlas { .. }) }

    fn mapped_data(&self) -> Option<&'static crate::LasVegasData> {
        match self.terrain {
            Terrain::Surveyed(map) => Some(map),
            Terrain::Atlas { maps, .. } => Some(maps.la),
            Terrain::Procedural(_) => None,
        }
    }

    fn metropolitan(&self) -> bool {
        match self.terrain {
            Terrain::Procedural(value) | Terrain::Atlas { metropolitan: value, .. } => value,
            Terrain::Surveyed(_) => false,
        }
    }

    pub fn city_origin(&self, city: crate::MappedCity) -> [f32; 2] {
        if self.is_atlas() && city == crate::MappedCity::LasVegas { LAS_VEGAS_ATLAS_ORIGIN }
        else { [0.0, 0.0] }
    }

    pub(crate) fn local_city(&self, city: crate::MappedCity) -> Option<(Self, [f32; 3])> {
        let map = match self.terrain {
            Terrain::Atlas { maps, .. } if city == crate::MappedCity::LasVegas => maps.vegas,
            _ => self.mapped_data().filter(|map| map.city() == city)?,
        };
        let [east, north] = self.city_origin(city);
        let up = if self.is_atlas() { map.datum() } else { 0.0 };
        Some((Self::mapped(map), [east, up, north]))
    }

    fn atlas_patch(&self, x: f32, z: f32) -> Option<(&'static crate::LasVegasData, f32, f32, f32)> {
        let Terrain::Atlas { maps, .. } = self.terrain else { return None; };
        let vegas = maps.vegas;
        let [east, north] = LAS_VEGAS_ATLAS_ORIGIN;
        let lx = x - east;
        let lz = z - north;
        let weight = vegas.terrain_weight(lx, lz);
        if weight > 0.0 { return Some((vegas, lx, lz, weight)); }
        let la = maps.la;
        let weight = la.terrain_weight(x, z);
        (weight > 0.0).then_some((la, x, z, weight))
    }

    fn atlas_ground(&self, x: f32, z: f32) -> (Sample, f32) {
        let patch = self.atlas_patch(x, z);
        if let Some((map, east, north, 1.0)) = patch {
            let (mut ground, shore) = map.terrain(self, east, north);
            ground.elevation += map.datum();
            return (ground, shore);
        }
        let pattern = Self { terrain: Terrain::Procedural(self.metropolitan()), ..*self };
        let ground = match self.terrain { Terrain::Atlas { maps, .. } => maps.ground.as_ref(), _ => None };
        let (generated, coast) = ground.and_then(|ground| ground.sample(x, z))
            .map_or_else(|| pattern.sample_and_shore(x, z), |(height, weight)| {
                let (generated, coast) = if weight < 1.0 {
                    pattern.sample_and_shore(x, z)
                } else {
                    (Sample { elevation: 0.0, surface: Surface::Mojave }, 0.52)
                };
                let raw = if coast <= 0.0 { coast * 19.0 } else { generated.elevation };
                let elevation = mix(raw, height, weight);
                let surface = if elevation <= 0.0 { Surface::DeepWater }
                    else if elevation < 3.5 { Surface::Sand }
                    else if elevation > 2_800.0 { Surface::Snow }
                    else if elevation > 1_700.0 { Surface::Rock }
                    else if self.noise(x, z, 0.0007, 0xf331_86af) > 0.35 { Surface::Grass }
                    else { Surface::Mojave };
                (Sample { elevation: elevation.max(0.0), surface },
                    if elevation <= 0.0 { (elevation / 19.0).min(-0.01) } else { 0.52 })
            });
        let Some((map, east, north, weight)) = patch else { return (generated, coast); };
        let (ground, shore) = map.terrain(self, east, north);
        let generated_height = if coast <= 0.0 { coast * 19.0 } else { generated.elevation };
        let surveyed_height = if shore <= 0.0 && ground.elevation < 2.0 && map.datum() == 0.0 {
            shore * 19.0
        } else {
            ground.elevation + map.datum()
        };
        let elevation = mix(generated_height, surveyed_height, weight);
        let shore = if elevation <= 0.0 { elevation / 19.0 }
            else { mix(coast, shore, weight).max(0.001) };
        let material = if weight >= 0.5 { ground.surface } else { generated.surface };
        let surface = if elevation <= 0.0 {
            Surface::DeepWater
        } else if matches!(material, Surface::DeepWater | Surface::Shallows | Surface::TarnWater) {
            Surface::Sand
        } else {
            material
        };
        (Sample { elevation: elevation.max(0.0), surface }, shore)
    }

    pub fn mapped_city(&self) -> Option<crate::MappedCity> {
        self.mapped_data().map(|data| data.city())
    }

    pub fn is_las_vegas(&self) -> bool {
        self.mapped_city() == Some(crate::MappedCity::LasVegas)
    }

    pub fn is_mapped_city(&self) -> bool {
        self.mapped_data().is_some()
    }

    pub fn has_metropolis(&self) -> bool {
        self.metropolitan() || self.is_mapped_city()
    }

    pub(crate) fn metropolis(&self) -> Option<crate::metropolis::Metropolis> {
        self.metropolitan().then(|| {
            let seed = u64::from(self.key[0]) | (u64::from(self.key[1]) << 32);
            crate::metropolis::Metropolis::new(seed)
        })
    }

    pub fn city_occupies(&self, x: f32, z: f32) -> bool {
        if self.is_atlas() {
            return self.atlas_town_at(x, z)
                || self.atlas_patch(x, z).is_some_and(|(map, east, north, _)| map.in_mapped_area(east, north));
        }
        if let Some(map) = self.mapped_data() {
            return map.in_mapped_area(x, z);
        }
        self.metropolis().is_some_and(|city| city.owns(x, z))
    }

    pub fn mapped_sample(&self, x: f32, z: f32) -> Option<Sample> {
        if self.is_atlas() {
            let (map, east, north, _) = self.atlas_patch(x, z)?;
            if !map.in_mapped_area(east, north) { return None; }
            let mut sample = map.scene_sample(self, east, north);
            sample.elevation += map.datum();
            return Some(sample);
        }
        self.mapped_data().map(|map| map.scene_sample(self, x, z))
    }

    pub(crate) fn city_airport_departure(&self) -> Option<(f32, f32, f32)> {
        self.metropolis()?.airport_departure()
    }

    pub(crate) fn city_rail_departure(&self) -> Option<(f32, f32, f32)> {
        self.metropolis()?.rail_departure()
    }

    pub(crate) fn city_fringe_departure(&self) -> Option<(f32, f32, f32)> {
        self.metropolis()?.fringe_departure()
    }

    pub fn metro_transit(&self) -> Option<crate::MetroTransit> {
        self.metropolis().map(crate::metropolis::Metropolis::transit)
    }

    fn atlas_town_at(&self, x: f32, z: f32) -> bool {
        let Terrain::Atlas { maps, .. } = self.terrain else { return false; };
        maps.towns.is_some_and(|towns| towns.covers(x, z))
    }

    /// Surveyed roads, buildings and rail between the detailed city maps.
    pub fn corridor_surface(&self, x: f32, z: f32, base: Sample) -> Option<Sample> {
        let Terrain::Atlas { maps, .. } = self.terrain else { return None; };
        if base.elevation <= SEA_LEVEL { return None; }
        if let Some(detail) = maps.towns.and_then(|towns| towns.overlay(x, z, base)) {
            return Some(detail);
        }
        let surface = maps.roads?.surface(x, z)?;
        Some(Sample { surface, ..base })
    }

    pub fn city_surface(&self) -> impl Fn(f32, f32, Sample) -> Option<Sample> + Copy {
        let city = self.metropolis().map(|city| (city, city.transit()));
        let mapped = *self;
        move |x, z, base| {
            if mapped.is_atlas() {
                if let Some((map, east, north, _)) = mapped.atlas_patch(x, z) {
                    if map.in_mapped_area(east, north) { return Some(map.detail(east, north, base)); }
                }
                if let Some(detail) = mapped.corridor_surface(x, z, base) {
                    return Some(detail);
                }
                if mapped.atlas_town_at(x, z) { return None; }
            } else if let Some(map) = mapped.mapped_data() {
                return Some(map.detail(x, z, base));
            }
            let (city, rail) = city?;
            city.overlay_with_transit(rail, x, z, base)
        }
    }

    pub(crate) fn built_height(&self, x: f32, z: f32, ground: f32) -> f32 {
        let geographic = if self.is_atlas() { self.atlas_patch(x, z).map(|(map, east, north, _)| (map, east, north)) }
            else { self.mapped_data().map(|map| (map, x, z)) };
        let base = Sample { elevation: ground, surface: Surface::Grass };
        if let Some((map, east, north)) = geographic.filter(|(map, east, north)| map.in_mapped_area(*east, *north)) {
            let landmark = match map.city() {
                crate::MappedCity::LasVegas => crate::las_vegas::strip_landmark_clearance(east, north),
                crate::MappedCity::LosAngeles => crate::los_angeles::landmark_clearance(east, north),
            };
            return map.detail(east, north, base).elevation
                .max(ground + landmark);
        }
        if self.atlas_town_at(x, z) {
            return self.corridor_surface(x, z, base).map_or(ground, |sample| sample.elevation);
        }
        let Some(city) = self.metropolis() else { return ground; };
        let rail = city.transit();
        city.overlay_with_transit(rail, x, z, base).map_or(ground, |sample| sample.elevation)
            .max(city.rail_clearance(rail, x, z, ground))
    }

    pub(crate) fn city_approach_height(&self, x: f32, z: f32, ground: f32) -> f32 {
        if self.is_atlas() {
            return ground + self.atlas_patch(x, z).map_or(0.0, |(map, east, north, _)| match map.city() {
                crate::MappedCity::LasVegas => crate::las_vegas::safety_height(east, north),
                crate::MappedCity::LosAngeles => crate::los_angeles::safety_height(east, north),
            });
        }
        match self.mapped_city() {
            Some(crate::MappedCity::LasVegas) => ground + crate::las_vegas::safety_height(x, z),
            Some(crate::MappedCity::LosAngeles) => ground + crate::los_angeles::safety_height(x, z),
            None => self.metropolis().map_or(ground, |city| ground + city.overflight_height(x, z)),
        }
    }

    fn basin(&self, x: f32, z: f32) -> Option<(f32, f32)> {
        self.metropolis()?.basin(x, z)
    }

    // The dry shoreline and the height derive from the same basin. Reuse the
    // ordinary field outside the blend so unaffected areas stay seed-stable.
    fn shaped_ground<const SHAPED: bool>(&self, x: f32, z: f32) -> (f32, f32) {
        if self.is_atlas() {
            let (sample, shore) = self.atlas_ground(x, z);
            return (sample.elevation, shore);
        }
        if let Some(map) = self.mapped_data() {
            let (sample, shore) = map.terrain(self, x, z);
            return (sample.elevation, shore);
        }
        let basin = self.basin(x, z);
        if let Some((height, 1.0)) = basin {
            return (height, 0.43);
        }
        let coast = self.coast(x, z);
        let natural = if coast <= 0.0 { coast * 19.0 } else {
            self.land_height::<SHAPED>(x, z, coast)
        };
        if let Some((height, weight)) = basin {
            let blended = mix(natural, height, weight);
            let shore = if blended <= 0.0 { blended / 19.0 } else {
                mix(coast.max(0.0), 0.43, weight).max(0.001)
            };
            (blended, shore)
        } else {
            (natural, coast)
        }
    }

    /// Height of the visible terrain or water surface.
    pub fn elevation(&self, x: f32, z: f32) -> f32 {
        self.elevation_variant::<true>(x, z)
    }

    pub(crate) fn route_elevation(&self, x: f32, z: f32) -> f32 {
        let elevation = self.elevation(x, z);
        if let Terrain::Atlas { maps, .. } = self.terrain {
            if let Some(AtlasTopography::Streamed(ground)) = &maps.ground {
                if let Some(conservative) = ground.safety_height(x, z) {
                    return elevation.max(conservative);
                }
            }
        }
        elevation
    }

    pub(crate) fn signed_horizon_height(&self, x: f32, z: f32) -> f32 {
        self.shaped_ground::<true>(x, z).0
    }

    /// Fixed terrain signal for selecting regional layouts. Collisions and
    /// every placement check still use the shaped terrain.
    pub(crate) fn regional_pattern_height(&self, x: f32, z: f32) -> f32 {
        self.elevation_variant::<false>(x, z)
    }

    fn elevation_variant<const SHAPED: bool>(&self, x: f32, z: f32) -> f32 {
        SEA_LEVEL + self.shaped_ground::<SHAPED>(x, z).0.max(0.0)
    }

    pub fn sample(&self, x: f32, z: f32) -> Sample {
        self.sample_and_shore(x, z).0
    }

    pub(crate) fn sample_and_shore(&self, x: f32, z: f32) -> (Sample, f32) {
        if self.is_atlas() { return self.atlas_ground(x, z); }
        if let Some(map) = self.mapped_data() {
            return map.terrain(self, x, z);
        }
        let (height, coast) = self.shaped_ground::<true>(x, z);
        if coast <= 0.0 {
            let surface = if coast < -0.105 {
                Surface::DeepWater
            } else {
                Surface::Shallows
            };
            return (Sample { elevation: SEA_LEVEL, surface }, coast);
        }

        #[cfg(feature = "regional-climate")]
        let region = crate::climate::Climate::over_terrain(self, x, z, coast, height);
        let surface = if height < 2.5 {
            Surface::Sand
        } else if height >= 88.0 {
            let climate = self.noise(x, z, 0.0018, 0x58fe_b14d);
            let snow_line = 129.0 + climate * 24.0;
            #[cfg(feature = "regional-climate")]
            let snow_line = snow_line + (region.axes()[0] - 0.45) * 19.0;
            if height > snow_line {
                Surface::Snow
            } else {
                Surface::Rock
            }
        } else {
            let regional = self.noise(x, z, 0.0019, 0xca34_7b59);
            let local_moisture = self.noise(z, -x, 0.0063, 0x8cb9_2e21);
            let moisture = regional * 0.76 + local_moisture * 0.24;
            #[cfg(feature = "regional-climate")]
            let moisture = {
                let [heat, wet, relief, wind] = region.axes();
                moisture + (wet - 0.51) * 0.31 - (heat - 0.5) * 0.16
                    - relief * wind * 0.07
            };
            if moisture < -0.31 && height < 64.0 && coast > 0.12 {
                Surface::Desert
            } else if moisture > 0.16 && height < 76.0 {
                Surface::Forest
            } else {
                Surface::Grass
            }
        };
        (Sample { elevation: SEA_LEVEL + height, surface }, coast)
    }

    /// A signed, continuous coast field for route planning. Land is positive.
    pub(crate) fn coast_signal(&self, x: f32, z: f32) -> f32 {
        if !self.has_metropolis() {
            return self.coast(x, z);
        }
        self.shaped_ground::<false>(x, z).1
    }

    // The sign of this field alone decides whether the ground is dry.
    // Inlets shape the continental edge while relief grows inland.
    fn coast(&self, x: f32, z: f32) -> f32 {
        let u = x * 0.8 + z * 0.6;
        let v = z * 0.8 - x * 0.6;
        let continent = self.noise(x, z, 0.00135, 0x730d_31db);
        let shelf = self.noise(u, v, 0.0039, 0x3f91_8b27);
        let inlet = self.noise(u, v, 0.0127, 0x982d_74f1);
        0.11 + continent * 0.79 + shelf * 0.23 + inlet * 0.058
    }

    fn land_height<const SHAPED: bool>(&self, x: f32, z: f32, coast: f32) -> f32 {
        let rise = coast * (18.0 + 12.0 * coast);
        let foothills = smooth_range(0.035, 0.37, coast);
        if foothills == 0.0 {
            return rise;
        }

        let roll = self.noise(x, z, 0.0095, 0xf037_eb15);
        let hills = rise + foothills * (10.0 + 8.0 * roll);
        let inland = smooth_range(0.17, 0.63, coast);
        if inland == 0.0 {
            return hills;
        }

        let u = x * 0.8 - z * 0.6;
        let v = x * 0.6 + z * 0.8;
        let belt = self.noise(u, v, 0.0026, 0x17da_09b5);
        let mountain = inland * smooth_range(-0.28, 0.57, belt);
        if mountain == 0.0 {
            return hills;
        }

        let fold = self.noise(u, v, 0.0087, 0x491e_8a39);
        let crest = 1.0 - fold * fold;
        let scale = 100.0 + 11.0 * roll;
        let base = hills + mountain * (23.0 + scale * crest * crest * crest);
        if !SHAPED || base <= 68.0 {
            return base;
        }
        let relief = smooth_range(68.0, 110.0, base);
        let distance = (fold + 0.08 * roll).abs().min(1.0);
        // Round only the top of each ridge, retaining a clear shoulder.
        let shoulder = if distance < 0.16 {
            0.08 + distance * distance * 3.125
        } else {
            distance
        };
        let ridge = 1.0 - shoulder;
        let summit = hills + mountain * (37.0 + scale * ridge * (0.52 + 0.48 * ridge));
        let face = self.noise(
            u * 1.20 + v * 0.18,
            v * 0.72 - u * 0.10,
            0.0117,
            0x5af2_9037,
        );
        let slope = fold.abs();
        let valley_flank =
            smooth_range(0.10, 0.29, slope) * (1.0 - smooth_range(0.61, 0.85, slope));
        let cirque = 1.0 - smooth_range(-0.44, 0.50, face + fold * 0.16);
        let incision = mountain * (2.0 + 13.0 * cirque * cirque) * valley_flank;
        let basin = self.alpine_basin(u, v);
        let shaped = base + relief * (0.58 * (summit - base) - incision - basin);
        #[cfg(feature = "regional-climate")]
        let shaped = {
            // Stony dry hillsides retain hard ribs between narrow channels.
            // Confine incision to the flank so it connects to the existing valleys.
            let moisture = self.noise(x, z, 0.00071, 0xbb14_06e7);
            let dry = 1.0 - smooth_range(-0.46, 0.05, moisture);
            let fluting = self.noise(u * 1.54 + v * 0.17, v * 0.62, 0.028, 0x4ab2_e5c9);
            let gullies = 1.0 - smooth_range(0.07, 0.58, fluting.abs());
            shaped - relief * mountain * valley_flank * dry * gullies * 4.5
        };
        shaped.max(68.0 + (base - 68.0) * 0.23)
    }

    // In some massifs, an elongated glacial trough cuts a broad floor.
    // The largest outline fits inside its seed cell, so seams cannot cut it.
    fn alpine_basin(&self, u: f32, v: f32) -> f32 {
        const CELL: f32 = 384.0;
        let a = u / CELL;
        let b = v / CELL;
        let ix = lattice_floor(a);
        let iz = lattice_floor(b);
        let design = lattice(ix, iz, self.key[0] ^ 0x14d9_87ba);
        if design < 0.30 {
            return 0.0;
        }
        let cross = a - ix as f32 - 0.49;
        let along = b - iz as f32 - 0.50;
        let tilt = (design - 0.63) * 0.59;
        let waist = (cross - along * tilt) / (0.18 + design * 0.047);
        let reach = along / (0.30 + design * 0.085);
        let edge = waist * waist + reach * reach;
        let bowl = 1.0 - smooth_range(0.13, 1.0, edge);
        (14.0 + 13.0 * design) * bowl
    }

    pub(crate) fn noise(&self, x: f32, z: f32, scale: f32, lane: u32) -> f32 {
        let x = x * scale;
        let z = z * scale;
        let ix = lattice_floor(x);
        let iz = lattice_floor(z);
        let tx = ease((x - ix as f32).clamp(0.0, 1.0));
        let tz = ease((z - iz as f32).clamp(0.0, 1.0));
        let next_x = ix.wrapping_add(1);
        let next_z = iz.wrapping_add(1);
        let key = self.key[(lane >> 31) as usize] ^ lane;
        let lower = mix(lattice(ix, iz, key), lattice(next_x, iz, key), tx);
        let upper = mix(lattice(ix, next_z, key), lattice(next_x, next_z, key), tx);
        mix(lower, upper, tz)
    }
}

fn lattice_floor(value: f32) -> i32 {
    let integer = value as i32;
    if value < integer as f32 {
        integer.saturating_sub(1)
    } else {
        integer
    }
}

fn lattice(x: i32, z: i32, key: u32) -> f32 {
    let mut bits =
        key ^ (x as u32).wrapping_mul(0x9e37_79b1) ^ (z as u32).wrapping_mul(0x85eb_ca77);
    bits = (bits ^ (bits >> 16)).wrapping_mul(0x7feb_352d);
    bits = (bits ^ (bits >> 15)).wrapping_mul(0x846c_a68b);
    bits ^= bits >> 16;
    (bits >> 8) as f32 * (2.0 / 16_777_215.0) - 1.0
}

fn mix(a: f32, b: f32, weight: f32) -> f32 {
    a + (b - a) * weight
}

fn ease(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

fn smooth_range(low: f32, high: f32, value: f32) -> f32 {
    ease(((value - low) / (high - low)).clamp(0.0, 1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_and_subcell_positions_are_repeatable() {
        let a = World::new(31);
        let b = World::new(31);
        let c = World::new(31 | (1_u64 << 52));
        let mut different = 0;
        for index in -64..64 {
            let x = index as f32 * 17.25 + 0.375;
            let z = 233.5 - index as f32 * 11.75;
            let sample = a.sample(x, z);
            assert_eq!(sample, b.sample(x, z));
            assert_eq!(sample.elevation, a.elevation(x, z));
            different += usize::from(sample != c.sample(x, z));
        }
        assert!(different > 80);
    }

    #[test]
    fn lattice_and_world_heights_are_continuous_across_zero() {
        for boundary in -8..=8 {
            let world = World::new(2);
            let x = boundary as f32;
            assert_eq!(lattice_floor(x), boundary);
            assert_eq!(lattice_floor(x - 0.001), boundary - 1);
            let left = world.noise(x - 0.001, -34.25, 1.0, 27);
            let right = world.noise(x + 0.001, -34.25, 1.0, 27);
            assert!((left - right).abs() < 0.0001);
        }

        for seed in [2, 119, 8745] {
            let world = World::new(seed);
            for z in (-1600..1600).step_by(23) {
                let z = z as f32 + 0.37;
                assert!((world.elevation(-0.01, z) - world.elevation(0.01, z)).abs() < 0.15);
                assert!((world.elevation(z, -0.01) - world.elevation(z, 0.01)).abs() < 0.15);
            }
        }
    }

    #[test]
    fn new_alpine_relief_preserves_the_lowland_profile() {
        let world = World::new(31);
        for z in (-1700..=1700).step_by(31) {
            for x in (-1700..=1700).step_by(31) {
                let x = x as f32;
                let z = z as f32;
                let original = world.regional_pattern_height(x, z);
                let shaped = world.elevation(x, z);
                if original <= 68.0 {
                    assert_eq!(shaped, original);
                } else {
                    assert!(shaped > 68.0);
                }
            }
        }
        let ridge = world.regional_pattern_height(190.0, 624.0);
        let trough = world.elevation(190.0, 624.0);
        assert!(ridge - trough > 20.0);
    }

    #[test]
    fn landscape_has_all_surface_bands_and_bounded_grades() {
        let mut counts = [0_usize; 8];
        for seed in [1, 7, 31] {
            let world = World::new(seed);
            for z in (-2800..=2800).step_by(37) {
                for x in (-2800..=2800).step_by(37) {
                    let x = x as f32 + 0.125;
                    let z = z as f32 + 0.625;
                    let sample = world.sample(x, z);
                    let h = sample.elevation;
                    assert!((0.0..=200.0).contains(&h));
                    assert!((world.elevation(x + 1.0, z) - h).abs() < 4.2);
                    assert!((world.elevation(x, z + 1.0) - h).abs() < 4.2);
                    if matches!(sample.surface, Surface::DeepWater | Surface::Shallows) {
                        assert_eq!(h, SEA_LEVEL);
                    }
                    if sample.surface == Surface::Snow {
                        assert!(h > 104.0);
                    }
                    counts[sample.surface as usize] += 1;
                }
            }
        }
        assert!(counts.iter().all(|&count| count > 35), "{counts:?}");
    }
}
