//! Prepared intermodal destinations on an existing regional highway.

use crate::airfield::{AirfieldMaterial, AirfieldSample, AirfieldSite};
use crate::ports::{Port, PortSample, PortSurface};
use crate::quayside::Quayside;
use crate::rail::{RailAnchorKind, RailLine, RailMaterial, RailSample, RailService};
use crate::roads::{RoadNetwork, RoadPoint};
use crate::scenic_turnout::ScenicTurnout;
use crate::settlement::{Settlement, SettlementSurface, Terminus};
use crate::solar_sites::SolarSite;
use crate::train_art::{contextual_freight_route, FreightRoute};
use crate::water_tower::WaterTower;
use crate::working_quarry::Quarry;
use crate::world::{Sample, Surface, World};

// Port hit-testing for dry land only. Hard obstacles and contact geometry have
// already been sited against the real world by the port planner.
const DRY_FOOTPRINT: Sample = Sample {
    elevation: 4.0,
    surface: Surface::Grass,
};
const FOOTPRINT_SPACING: f32 = 0.8;
const QUARRY_HAUL_REACH: f32 = 512.0;

pub(crate) fn short_haul(road: &RoadNetwork, [x, _, z]: [f32; 3], end: Terminus) -> bool {
    let spans = road.polyline().windows(2);
    let mut distance = 0.0;
    let mut junction = None;
    for span in spans {
        let [a, b] = [span[0], span[1]];
        let dx = b.x - a.x;
        let dz = b.z - a.z;
        let squared = dx * dx + dz * dz;
        let mut length = dx.abs().max(dz.abs());
        for _ in 0..3 {
            length = 0.5 * (length + squared / length);
        }
        if (a.x + b.x) * 0.5 == x && (a.z + b.z) * 0.5 == z {
            junction = Some(distance + length * 0.5);
        }
        distance += length;
    }
    junction.is_some_and(|junction| match end {
        Terminus::Start => junction <= QUARRY_HAUL_REACH,
        Terminus::End => distance - junction <= QUARRY_HAUL_REACH,
    })
}

pub(crate) fn train_route_near(camera: crate::render::Camera, bounds: [f32; 4]) -> bool {
    let [left, near, right, far] = bounds;
    let dx = camera.x - camera.x.clamp(left, right);
    let dz = camera.z - camera.z.clamp(near, far);
    // The bounds already include the whole grade. Keep a small allowance for a car's width.
    let reach = crate::train_art::TRAIN_REACH + 8.0;
    dx * dx + dz * dz <= reach * reach
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FacilitySample {
    Port(PortSample),
    Airfield(AirfieldSample),
    Rail(RailSample),
    Quarry,
    Solar(Sample),
    Quayside(Sample),
    WaterTower,
    ScenicTurnout(Sample),
}

#[derive(Clone, Copy)]
pub(crate) enum RoadsideSite {
    Quarry { site: Quarry, local_transfer: bool },
    Solar(SolarSite),
    Quayside(Quayside),
    WaterTower(WaterTower),
    ScenicTurnout(ScenicTurnout),
}

impl RoadsideSite {
    pub(crate) fn bounds(&self) -> [f32; 4] {
        match self {
            Self::Quarry { site, .. } => site.bounds(),
            Self::Solar(site) => site.bounds(),
            Self::Quayside(site) => site.bounds(),
            Self::WaterTower(site) => site.bounds(),
            Self::ScenicTurnout(site) => site.bounds(),
        }
    }
}

#[derive(Clone, Default)]
pub struct RegionalFacilities {
    ports: [Option<Port>; 2],
    airfield: Option<AirfieldSite>,
    rail: Option<RailLine>,
    roadside: Option<RoadsideSite>,
}

impl RegionalFacilities {
    /// Synchronous planning for a prepared highway and its optional towns in
    /// start/end order. Call during regional preparation, outside drawing.
    pub fn prepare(
        world: &World,
        seed: u64,
        road: &RoadNetwork,
        towns: [Option<&Settlement>; 2],
    ) -> Self {
        Self::prepare_cooperatively(world, seed, road, towns, || {})
    }

    /// Run the same survey while allowing the caller to service frame transfers.
    pub fn prepare_cooperatively(
        world: &World, seed: u64, road: &RoadNetwork, towns: [Option<&Settlement>; 2],
        mut service: impl FnMut(),
    ) -> Self {
        let mut planner = RegionalFacilityPlanner::new(*world, seed);
        while planner.step_cooperatively(road, towns, &mut service) != FacilityPhase::Ready {}
        planner.finish().unwrap_or_default()
    }

    pub fn is_empty(&self) -> bool {
        !self.has_primary() && self.roadside.is_none()
    }

    pub(crate) fn has_primary(&self) -> bool {
        self.ports.iter().any(Option::is_some) || self.airfield.is_some() || self.rail.is_some()
    }

    pub fn port(&self, end: Terminus) -> Option<&Port> {
        self.ports[match end {
            Terminus::Start => 0,
            Terminus::End => 1,
        }]
        .as_ref()
    }

    pub fn ports(&self) -> impl Iterator<Item = &Port> {
        self.ports.iter().flatten()
    }

    pub fn airfield(&self) -> Option<&AirfieldSite> {
        self.airfield.as_ref()
    }

    pub fn rail(&self) -> Option<&RailLine> {
        self.rail.as_ref()
    }

    pub fn quarry(&self) -> Option<&Quarry> {
        match self.roadside.as_ref()? {
            RoadsideSite::Quarry { site, .. } => Some(site),
            _ => None,
        }
    }

    pub fn solar_site(&self) -> Option<&SolarSite> {
        match self.roadside.as_ref()? {
            RoadsideSite::Solar(site) => Some(site),
            _ => None,
        }
    }

    pub fn quayside(&self) -> Option<&Quayside> {
        match self.roadside.as_ref()? {
            RoadsideSite::Quayside(quay) => Some(quay),
            _ => None,
        }
    }

    pub fn water_tower(&self) -> Option<&WaterTower> {
        match self.roadside.as_ref()? {
            RoadsideSite::WaterTower(tower) => Some(tower),
            _ => None,
        }
    }

    pub fn scenic_turnout(&self) -> Option<&ScenicTurnout> {
        match self.roadside.as_ref()? {
            RoadsideSite::ScenicTurnout(turnout) => Some(turnout),
            _ => None,
        }
    }

    pub(crate) fn roadside(&self) -> Option<&RoadsideSite> {
        self.roadside.as_ref()
    }

    pub(crate) fn install_roadside(&mut self, site: RoadsideSite) {
        if matches!(site, RoadsideSite::Quarry { local_transfer: true, .. }) {
            if let Some(rail) = self.rail.as_mut().filter(|line| line.service == RailService::Freight) {
                rail.freight_route = FreightRoute::Aggregate;
            }
        }
        self.roadside = Some(site);
    }

    pub(crate) fn roadside_quarry(&self, site: Quarry, road: &RoadNetwork) -> RoadsideSite {
        let points = road.polyline();
        let local_transfer = self.rail.as_ref()
            .filter(|rail| rail.service == RailService::Freight && rail.grain_terminal().is_none())
            .and_then(|rail| rail.stations().iter().find(|station| !station.anchor.kind.is_port()))
            .is_some_and(|station| {
                [Terminus::Start, Terminus::End].into_iter().zip([points.first(), points.last()])
                    .any(|(end, point)| point.is_some_and(|point| {
                        station.road_contact() == (point.x, point.z)
                            && short_haul(road, site.junction(), end)
                    }))
            });
        RoadsideSite::Quarry { site, local_transfer }
    }

    /// Conservative world bounds in minimum-X, minimum-Z, maximum-X,
    /// maximum-Z order, including approach roads and moored vessels.
    pub fn bounds(&self) -> Option<[f32; 4]> {
        self.ports()
            .map(Port::bounds)
            .chain(self.airfield().map(|field| airfield_bounds(field).0))
            .chain(self.rail().map(RailLine::bounds))
            .chain(self.roadside().map(RoadsideSite::bounds))
            .reduce(|a, b| {
                [
                    a[0].min(b[0]),
                    a[1].min(b[1]),
                    a[2].max(b[2]),
                    a[3].max(b[3]),
                ]
            })
    }

    /// The paved highway cell shared by a port road and rail platform access.
    pub fn port_transfer(&self, end: Terminus, road: &RoadNetwork) -> Option<(f32, f32)> {
        self.port(end)?;
        let points = road.polyline();
        let point = match end {
            Terminus::Start => points.first()?,
            Terminus::End => points.last()?,
        };
        self.rail()?.stations().iter().find_map(|station| {
            let (x, z) = station.road_contact();
            (station.anchor.kind.is_port() && x == point.x && z == point.z)
                .then_some((x, z))
        })
    }

    /// Probe retained geometry using the unmodified world sample. No world
    /// lookups, route planning, or allocation runs here. Joined access lanes
    /// share the port surface where they overlap.
    pub fn sample(&self, x: f32, z: f32, raw: Sample) -> Option<FacilitySample> {
        if let Some(port) = self.ports().find_map(|port| port.sample(x, z, raw)) {
            return Some(FacilitySample::Port(port));
        }
        if let Some(airfield) = self.airfield().and_then(|field| field.sample(x, z)) {
            return Some(FacilitySample::Airfield(airfield));
        }
        self.rail()
            .and_then(|line| line.sample(x, z))
            .map(FacilitySample::Rail)
            .or_else(|| match self.roadside()? {
                RoadsideSite::Quarry { site, .. } => site.occupies(x, z).then_some(FacilitySample::Quarry),
                RoadsideSite::Solar(site) => site.sample(x, z, raw).map(FacilitySample::Solar),
                RoadsideSite::Quayside(site) => site.sample(x, z, raw).map(FacilitySample::Quayside),
                RoadsideSite::WaterTower(site) => site.reserves(x, z).then_some(FacilitySample::WaterTower),
                RoadsideSite::ScenicTurnout(site) => site.sample(x, z, raw).map(FacilitySample::ScenicTurnout),
            })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FacilityPhase {
    PortStart,
    PortEnd,
    Airfield,
    Rail,
    Ready,
}

/// Plan a retained set of facilities without borrowing a cache slot across
/// service calls. Each step runs at most one existing facility proposal.
pub struct RegionalFacilityPlanner {
    world: World,
    seed: u64,
    facilities: RegionalFacilities,
    phase: FacilityPhase,
}

impl RegionalFacilityPlanner {
    pub fn new(world: World, seed: u64) -> Self {
        Self {
            world,
            seed,
            facilities: RegionalFacilities::default(),
            phase: FacilityPhase::PortStart,
        }
    }

    pub fn phase(&self) -> FacilityPhase {
        self.phase
    }

    /// Advance one job and return the next phase. Keep the same road and
    /// start/end town slots for all calls to this planner.
    pub fn step(&mut self, road: &RoadNetwork, towns: [Option<&Settlement>; 2]) -> FacilityPhase {
        self.step_cooperatively(road, towns, &mut || {})
    }

    fn step_cooperatively(
        &mut self, road: &RoadNetwork, towns: [Option<&Settlement>; 2], pulse: &mut impl FnMut(),
    ) -> FacilityPhase {
        use FacilityPhase::{Airfield, PortEnd, PortStart, Rail, Ready};

        if self.phase == Ready {
            return Ready;
        }
        pulse();
        let points = road.polyline();
        let (Some(&first), Some(&last)) = (points.first(), points.last()) else {
            self.phase = Ready;
            return Ready;
        };
        let termini = [first, last];
        let districts = core::array::from_fn(|index| {
            towns[index].map(|town| District {
                town,
                bounds: Bounds::around(termini[index].x, termini[index].z, 108.0),
            })
        });
        let ports = &mut self.facilities.ports;
        self.phase = match self.phase {
            PortStart | PortEnd => {
                let index = usize::from(self.phase == PortEnd);
                let end = termini[index];
                ports[index] =
                    Port::generate_cooperatively(&self.world, self.seed, end.x, end.z, pulse).filter(|port| {
                        let bounds = Bounds(port.bounds());
                        ports
                            .iter()
                            .flatten()
                            .all(|other| bounds.intersect(Bounds(other.bounds())).is_none())
                            && port_contact(&self.world, port, end, pulse)
                            && !port_conflicts(port, road, &districts, pulse)
                    });
                if index == 0 {
                    PortEnd
                } else {
                    Airfield
                }
            }
            Airfield => {
                self.facilities.airfield = AirfieldSite::for_road_cooperatively(&self.world, self.seed, road, pulse)
                    .filter(|field| {
                        field.road_contact().is_some_and(|(x, z)| {
                            termini.iter().any(|p| p.x == x && p.z == z)
                                && airfield_material(field, x, z)
                                    == Some(AirfieldMaterial::AccessRoad)
                        }) && !airfield_conflicts(field, ports, &districts, pulse)
                    });
                Rail
            }
            Rail => {
                let kinds = core::array::from_fn(|index| {
                    if let Some(port) = ports[index] {
                        RailAnchorKind::for_port(port.kind())
                    } else if towns[index].is_some() {
                        RailAnchorKind::Town
                    } else {
                        RailAnchorKind::RoadTerminus
                    }
                });
                self.facilities.rail =
                    RailLine::for_road_cooperatively(&self.world, self.seed, road, kinds, pulse).filter(
                        |line| {
                            line.stations().iter().all(|station| {
                                let (x, z) = station.road_contact();
                                termini.iter().any(|p| p.x == x && p.z == z)
                                    && rail_material(line, x, z)
                                        == Some(RailMaterial::StationAccess)
                            }) && !rail_conflicts(
                                line,
                                self.facilities.airfield.as_ref(),
                                ports,
                                &districts,
                                pulse,
                            )
                        },
                    );
                if let Some(line) = self.facilities.rail.as_mut().filter(|line| line.service == RailService::Commuter) {
                    line.survey_grain_terminal(&self.world, self.seed, road, |x, z| {
                        districts.iter().flatten().any(|district| district.town.sample(x, z).is_some())
                            || self.facilities.airfield.as_ref().is_some_and(|site| site.sample(x, z).is_some())
                            || ports.iter().flatten().any(|port| port.sample(x, z, self.world.sample(x, z)).is_some())
                    }, &mut *pulse);
                }
                if let Some(line) = self.facilities.rail.as_mut().filter(|line| line.service == RailService::Freight
                    && line.grain_terminal().is_none())
                {
                    let ends = line.stations().map(|station| station.anchor);
                    line.freight_route = contextual_freight_route(&self.world, self.seed, ends, line.freight_route, |x, z| {
                        road.proximity(x, z).is_some()
                            || line.sample(x, z).is_some()
                            || districts.iter().flatten().any(|district| district.town.sample(x, z).is_some())
                            || self.facilities.airfield.as_ref().is_some_and(|site| site.sample(x, z).is_some())
                            || ports.iter().flatten().any(|port| port.sample(x, z, self.world.sample(x, z)).is_some())
                    });
                }
                Ready
            }
            Ready => Ready,
        };
        self.phase
    }

    /// Only a complete plan is available for publication.
    pub fn finish(self) -> Option<RegionalFacilities> {
        (self.phase == FacilityPhase::Ready).then_some(self.facilities)
    }
}

#[derive(Clone, Copy)]
struct District<'a> {
    town: &'a Settlement,
    // A settlement extends at most 54 across and 90 out from its road end.
    // This radius covers any orientation, its streets and parcel margins.
    bounds: Bounds,
}

#[derive(Clone, Copy)]
struct Bounds([f32; 4]);

impl Bounds {
    fn around(x: f32, z: f32, radius: f32) -> Self {
        Self([x - radius, z - radius, x + radius, z + radius])
    }

    fn contains(self, x: f32, z: f32) -> bool {
        let [left, top, right, bottom] = self.0;
        (left..=right).contains(&x) && (top..=bottom).contains(&z)
    }

    fn intersect(self, other: Self) -> Option<Self> {
        let [left, top, right, bottom] = self.0;
        let [x0, z0, x1, z1] = other.0;
        let bounds = Self([left.max(x0), top.max(z0), right.min(x1), bottom.min(z1)]);
        (bounds.0[0] <= bounds.0[2] && bounds.0[1] <= bounds.0[3]).then_some(bounds)
    }

    fn any(self, pulse: &mut impl FnMut(), mut hit: impl FnMut(f32, f32) -> bool) -> bool {
        let [left, top, right, bottom] = self.0;
        let cols = ((right - left) / FOOTPRINT_SPACING) as usize + 1;
        let rows = ((bottom - top) / FOOTPRINT_SPACING) as usize + 1;
        let dx = (right - left) / cols as f32;
        let dz = (bottom - top) / rows as f32;
        for row in 0..=rows {
            let z = top + dz * row as f32;
            for col in 0..=cols {
                if col & 15 == 0 { pulse(); }
                let x = left + dx * col as f32;
                if hit(x, z) {
                    return true;
                }
            }
        }
        false
    }
}

fn port_contact(world: &World, port: &Port, end: RoadPoint, pulse: &mut impl FnMut()) -> bool {
    let [x, z] = port.shore_origin();
    let [dx, dz] = port.seaward();
    let distance = (x - end.x) * dx + (z - end.z) * dz;
    let count = (distance / FOOTPRINT_SPACING) as usize + 1;
    for index in 0..=count {
        pulse();
        let along = distance * index as f32 / count as f32;
        let mut reaches_apron = false;
        for side in [-1.1, 0.0, 1.1] {
            let x = end.x + along * dx - side * dz;
            let z = end.z + along * dz + side * dx;
            match port.sample(x, z, world.sample(x, z)).map(|p| p.surface) {
                Some(PortSurface::AccessRoad) => (),
                Some(PortSurface::Apron) if index > 0 => {
                    reaches_apron = true;
                }
                _ => return false,
            }
        }
        if reaches_apron {
            return true;
        }
    }
    false
}

fn port_material(port: &Port, x: f32, z: f32) -> Option<PortSurface> {
    port.sample(x, z, DRY_FOOTPRINT).map(|part| part.surface)
}

fn airfield_material(field: &AirfieldSite, x: f32, z: f32) -> Option<AirfieldMaterial> {
    field.sample(x, z).map(|part| part.material)
}

fn rail_material(line: &RailLine, x: f32, z: f32) -> Option<RailMaterial> {
    line.sample(x, z).map(|part| part.material)
}

fn port_access(surface: PortSurface) -> bool {
    matches!(surface, PortSurface::AccessRoad | PortSurface::Apron)
}

fn port_conflicts(
    port: &Port, road: &RoadNetwork, districts: &[Option<District<'_>>; 2], pulse: &mut impl FnMut(),
) -> bool {
    Bounds(port.bounds()).any(pulse, |x, z| {
        let Some(surface) = port_material(port, x, z) else {
            return false;
        };
        // Port access and the apron can share a paved junction. Quays,
        // cargo and buildings have to sit outside official roadway.
        if !port_access(surface) && road.proximity(x, z).is_some() {
            return true;
        }
        districts.iter().flatten().any(|district| {
            district.bounds.contains(x, z)
                && district.town.sample(x, z).is_some_and(|part| {
                    !port_access(surface) || part.surface != SettlementSurface::Street
                })
        })
    })
}

fn airfield_bounds(field: &AirfieldSite) -> Bounds {
    let [(a, b), (c, d)] = field.runway_endpoints();
    let (x, z) = field.road_contact().unwrap_or_else(|| field.center());
    // Runway-end box covers the apron and roofs with this halo, and the
    // road connection can have one knee up to 38 units off the direct leg.
    Bounds([
        a.min(c).min(x) - 54.0,
        b.min(d).min(z) - 54.0,
        a.max(c).max(x) + 54.0,
        b.max(d).max(z) + 54.0,
    ])
}

fn airfield_conflicts(
    field: &AirfieldSite,
    ports: &[Option<Port>; 2],
    districts: &[Option<District<'_>>; 2],
    pulse: &mut impl FnMut(),
) -> bool {
    let bounds = airfield_bounds(field);
    for port in ports.iter().flatten() {
        if bounds.intersect(Bounds(port.bounds())).is_some_and(|area| {
            area.any(pulse, |x, z| {
                let Some(air) = airfield_material(field, x, z) else {
                    return false;
                };
                port_material(port, x, z)
                    .is_some_and(|port| air != AirfieldMaterial::AccessRoad || !port_access(port))
            })
        }) {
            return true;
        }
    }
    districts.iter().flatten().any(|district| {
        bounds.intersect(district.bounds).is_some_and(|area| {
            area.any(pulse, |x, z| {
                let Some(air) = airfield_material(field, x, z) else {
                    return false;
                };
                district.town.sample(x, z).is_some_and(|town| {
                    air != AirfieldMaterial::AccessRoad || town.surface != SettlementSurface::Street
                })
            })
        })
    })
}

fn rail_conflicts(
    line: &RailLine,
    field: Option<&AirfieldSite>,
    ports: &[Option<Port>; 2],
    districts: &[Option<District<'_>>; 2],
    pulse: &mut impl FnMut(),
) -> bool {
    let bounds = Bounds(line.bounds());
    for port in ports.iter().flatten() {
        if bounds.intersect(Bounds(port.bounds())).is_some_and(|area| {
            area.any(pulse, |x, z| {
                let Some(port) = port_material(port, x, z) else {
                    return false;
                };
                rail_material(line, x, z)
                    .is_some_and(|rail| rail != RailMaterial::StationAccess || !port_access(port))
            })
        }) {
            return true;
        }
    }
    if field.is_some_and(|field| {
        bounds
            .intersect(airfield_bounds(field))
            .is_some_and(|area| {
                area.any(pulse, |x, z| {
                    let Some(air) = airfield_material(field, x, z) else {
                        return false;
                    };
                    rail_material(line, x, z).is_some_and(|rail| {
                        rail != RailMaterial::StationAccess || air != AirfieldMaterial::AccessRoad
                    })
                })
            })
    }) {
        return true;
    }
    districts.iter().flatten().any(|district| {
        bounds.intersect(district.bounds).is_some_and(|area| {
            area.any(pulse, |x, z| {
                let Some(town) = district.town.sample(x, z) else {
                    return false;
                };
                rail_material(line, x, z).is_some_and(|rail| {
                    rail != RailMaterial::StationAccess || town.surface != SettlementSurface::Street
                })
            })
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::PortKind;
    use crate::rail::RailService;
    use std::collections::VecDeque;

    #[test]
    fn passenger_harbors_get_passenger_rail_and_shared_road_access() {
        let world = World::new(1);
        let road = RoadNetwork::generate(&world, -1, -3).unwrap();
        let plan = RegionalFacilities::prepare(&world, 1, &road, [None, None]);
        let line = plan.rail().unwrap();
        assert_eq!(line.service, RailService::Commuter);
        for (end, kind, position) in [
            (Terminus::Start, PortKind::Ferry, (-680.0, -2464.0)),
            (Terminus::End, PortKind::Marina, (-368.0, -2584.0)),
        ] {
            let port = plan.port(end).unwrap();
            assert_eq!(port.kind(), kind);
            assert_eq!(plan.port_transfer(end, &road), Some(position));
            let (x, z) = position;
            assert_eq!(road.proximity(x, z).unwrap().distance_squared, 0.0);
            assert_eq!(rail_material(line, x, z), Some(RailMaterial::StationAccess));
            assert!(matches!(
                plan.sample(x, z, world.sample(x, z)),
                Some(FacilitySample::Port(PortSample {
                    surface: PortSurface::AccessRoad,
                    ..
                }))
            ));
            assert!(station_path_reaches_platform(line, x, z, RailMaterial::StationPlatform));
        }

        let district = Settlement::generate(&world, &road, Terminus::Start).unwrap();
        let preserved = RegionalFacilities::prepare(&world, 1, &road, [Some(&district), None]);
        assert!(preserved.port(Terminus::Start).is_none());
        assert_eq!(preserved.port(Terminus::End), plan.port(Terminus::End));
    }

    #[test]
    fn retained_cargo_dock_has_a_freight_train_and_loading_platform() {
        use crate::scenery::DistrictRegion;

        let seed = 130;
        let world = World::new(seed);
        let district = DistrictRegion::build(&world, 11, 4);
        let road = district.road().unwrap();
        let towns = [Terminus::Start, Terminus::End].map(|end| district.town(end));
        let plan = RegionalFacilities::prepare(&world, seed, road, towns);
        assert!(plan.port(Terminus::Start).is_none());
        assert_eq!(plan.port(Terminus::End).unwrap().kind(), PortKind::Freight);
        let rail = plan.rail().unwrap();
        assert_eq!(rail.service, RailService::Freight);
        assert_eq!(rail.freight_route, crate::train_art::FreightRoute::Liquid);
        let (x, z) = plan.port_transfer(Terminus::End, road).unwrap();
        assert!(station_path_reaches_platform(rail, x, z, RailMaterial::FreightPlatform));
    }

    fn station_path_reaches_platform(line: &RailLine, start_x: f32, start_z: f32, platform: RailMaterial) -> bool {
        const SIZE: usize = 161;
        const CENTER: usize = SIZE / 2;
        let mut visited = [[false; SIZE]; SIZE];
        let mut queue = VecDeque::from([(CENTER, CENTER)]);
        visited[CENTER][CENTER] = true;
        while let Some((col, row)) = queue.pop_front() {
            let x = start_x + (col as f32 - CENTER as f32) * 0.5;
            let z = start_z + (row as f32 - CENTER as f32) * 0.5;
            match rail_material(line, x, z) {
                Some(material) if material == platform => return true,
                Some(RailMaterial::StationAccess) => {}
                _ => continue,
            }
            for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let x = col.wrapping_add_signed(dx);
                let z = row.wrapping_add_signed(dz);
                if x < SIZE && z < SIZE && !visited[z][x] {
                    visited[z][x] = true;
                    queue.push_back((x, z));
                }
            }
        }
        false
    }

    #[test]
    fn phased_preparation_matches_the_airport_towns_and_ports_fixture() {
        let world = World::new(9);
        let road = RoadNetwork::generate(&world, -6, -7).unwrap();
        let start = Settlement::generate(&world, &road, Terminus::Start).unwrap();
        let end = Settlement::generate(&world, &road, Terminus::End).unwrap();
        let towns = [Some(&start), Some(&end)];
        let expected = RegionalFacilities::prepare(&world, 9, &road, towns);

        use FacilityPhase::{Airfield, PortEnd, PortStart, Rail, Ready};
        let mut planner = RegionalFacilityPlanner::new(world, 9);
        assert!(RegionalFacilityPlanner::new(world, 9).finish().is_none());
        assert_eq!(planner.phase(), PortStart);
        for next in [PortEnd, Airfield, Rail, Ready, Ready] {
            assert_eq!(planner.step(&road, towns), next);
        }
        let plan = planner.finish().unwrap();
        assert_eq!(plan.bounds(), expected.bounds());
        for end in [Terminus::Start, Terminus::End] {
            assert_eq!(plan.port(end), expected.port(end));
            assert!(plan.port(end).is_some());
        }
        assert_eq!(plan.airfield(), expected.airfield());
        let airfield = plan.airfield().unwrap();
        let (x, z) = airfield.road_contact().unwrap();
        assert_eq!((x, z), (-5728.0, -6488.0));
        assert_eq!(road.proximity(x, z).unwrap().distance_squared, 0.0);
        assert_eq!(
            airfield_material(airfield, x, z),
            Some(AirfieldMaterial::AccessRoad)
        );
        assert_eq!(plan.rail(), expected.rail());
    }
}
