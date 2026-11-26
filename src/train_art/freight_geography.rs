use crate::ground_details::ParcelSurvey;
use crate::math::mix;
use crate::rail::{RailAnchor, RailAnchorKind, RailLine};
use crate::rail_corridor::RailCorridor;
use crate::world::{Surface, World};

use super::FreightRoute;

/// Physical source of a dedicated load/empty cycle on the surveyed line.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum LoadingEnd {
    First,
    Second,
}

pub(super) fn local_loading_end(line: &RailLine) -> Option<LoadingEnd> {
    if line.freight_route == FreightRoute::Agriculture {
        let [x, _, z] = line.grain_terminal()?.track;
        let [a, b] = line.stations().map(|station| station.track_point);
        let squared_distance = |px: f32, pz: f32| (x - px) * (x - px) + (z - pz) * (z - pz);
        return Some(if squared_distance(a.x, a.z) <= squared_distance(b.x, b.z) {
            LoadingEnd::First
        } else {
            LoadingEnd::Second
        });
    }
    aggregate_loading_end(line.freight_route, line.stations().map(|station| station.anchor.kind))
}

pub(super) fn regional_loading_end(line: &RailCorridor) -> Option<LoadingEnd> {
    aggregate_loading_end(line.freight_route, line.stations().map(|station| station.stop.kind()))
}

fn aggregate_loading_end(route: FreightRoute, ends: [RailAnchorKind; 2]) -> Option<LoadingEnd> {
    // Aggregate is only assigned after a real quarry and shorthaul connection
    // are published. Inferred mining and general freight have no known origin.
    if route != FreightRoute::Aggregate {
        return None;
    }
    use RailAnchorKind::{Port, RoadTerminus, Town};
    match ends {
        [RoadTerminus | Town, Port] => Some(LoadingEnd::First),
        [Port, RoadTerminus | Town] => Some(LoadingEnd::Second),
        _ => None,
    }
}

/// Assign a transfer service once when the grade is surveyed. The terrain
/// around the inland terminal suggests what its catchment can supply.
pub(crate) fn freight_route(world: &World, ends: [RailAnchor; 2]) -> FreightRoute {
    let terminal = ends
        .into_iter()
        .find(|end| end.kind != RailAnchorKind::Port);
    let key = ends.into_iter().fold(0_u64, |key, end| {
        key ^ mix(u64::from(end.x.to_bits()) << 32 | u64::from(end.z.to_bits()))
    });
    let roll = (key % 16) as u8;
    let Some(terminal) = terminal else {
        return match roll {
            0..=9 => FreightRoute::Intermodal,
            10..=12 => FreightRoute::Automotive,
            _ => FreightRoute::Liquid,
        };
    };

    // Large offsets describe land feeding the terminal, outside of its apron.
    let mut forest = 0;
    let mut pasture = 0;
    let mut mineral = 0;
    for (dx, dz) in [
        (-80.0, -80.0),
        (0.0, -100.0),
        (80.0, -80.0),
        (-100.0, 0.0),
        (0.0, 0.0),
        (100.0, 0.0),
        (-80.0, 80.0),
        (0.0, 100.0),
        (80.0, 80.0),
    ] {
        match world.sample(terminal.x + dx, terminal.z + dz).surface {
            Surface::Forest => forest += 1,
            Surface::Grass => pasture += 1,
            Surface::Rock | Surface::ScreeFine | Surface::ScreeCoarse => mineral += 1,
            _ => {}
        }
    }
    inland_service(forest, pasture, mineral, terminal.kind, roll)
}

/// Replace inferred land-use work with evidence from the surveyed inland
/// terminal. A freight port can still originate other general cargo services.
/// Called after the track fits and before it is published.
pub(crate) fn contextual_freight_route(
    world: &World,
    seed: u64,
    ends: [RailAnchor; 2],
    base: FreightRoute,
    mut occupied: impl FnMut(f32, f32) -> bool,
) -> FreightRoute {
    let terminal = match ends {
        [a, b] if a.kind == RailAnchorKind::Port && !b.kind.is_port() => b,
        [a, b] if b.kind == RailAnchorKind::Port && !a.kind.is_port() => a,
        _ => return base,
    };
    use FreightRoute::*;
    // A specific waterfront service already has a destination and cargo.
    if !matches!(base, Agriculture | Forest | Quarry | Mixed | Intermodal) {
        return base;
    }

    let mut parcels = ParcelSurvey::new();
    let mut crops = 0;
    let mut timber = 0;
    let mut outcrops = 0;
    let mut low = f32::INFINITY;
    let mut high = f32::NEG_INFINITY;
    for [dx, dz] in [
        [-96.0, -72.0],
        [-19.0, -106.0],
        [58.0, -89.0],
        [116.0, -43.0],
        [91.0, 44.0],
        [59.0, 111.0],
        [-27.0, 94.0],
        [-105.0, 62.0],
        [-64.0, -20.0],
        [53.0, -35.0],
        [19.0, 52.0],
        [-42.0, 37.0],
    ] {
        let [x, z] = [terminal.x + dx, terminal.z + dz];
        if occupied(x, z) {
            continue;
        }
        let land = world.sample(x, z);
        if land.elevation <= 3.0 {
            continue;
        }
        low = low.min(land.elevation);
        high = high.max(land.elevation);
        match land.surface {
            Surface::Grass if parcels.occupies_grass(world, seed, x, z) => crops += 1,
            Surface::Forest => timber += 1,
            Surface::Rock | Surface::ScreeFine | Surface::ScreeCoarse => outcrops += 1,
            _ => {}
        }
    }
    if crops >= 2 {
        Agriculture
    } else if timber >= 5 {
        Forest
    } else if outcrops >= 3 && high - low > 12.0 {
        Quarry
    } else if matches!(base, Agriculture | Forest | Quarry) {
        Mixed
    } else {
        base
    }
}

fn inland_service(
    forest: u8,
    pasture: u8,
    mineral: u8,
    terminal: RailAnchorKind,
    roll: u8,
) -> FreightRoute {
    if forest >= 4 && roll < 10 {
        FreightRoute::Forest
    } else if mineral >= 4 && roll < 10 {
        FreightRoute::Quarry
    } else if pasture >= 5 && roll < 10 {
        if roll < 7 {
            FreightRoute::Agriculture
        } else {
            FreightRoute::Perishable
        }
    } else if terminal == RailAnchorKind::Town {
        match roll {
            0..=4 => FreightRoute::Intermodal,
            5..=7 => FreightRoute::Mill,
            8..=9 => FreightRoute::Automotive,
            10..=11 => FreightRoute::Perishable,
            12..=13 => FreightRoute::Liquid,
            _ => FreightRoute::Mixed,
        }
    } else {
        match roll {
            0..=7 => FreightRoute::Intermodal,
            8..=10 => FreightRoute::Liquid,
            _ => FreightRoute::Mixed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verified_grain_source_loads_the_same_hoppers_while_parked() {
        use crate::facilities::RegionalFacilities;
        use crate::render::Camera;
        use crate::scenery::DistrictRegion;
        use crate::settlement::Terminus;
        use crate::train_art::{Train, Vehicle};

        let world = World::new(165);
        let district = DistrictRegion::build(&world, -7, -1);
        let road = district.road().unwrap();
        let towns = [Terminus::Start, Terminus::End].map(|end| district.town(end));
        let plan = RegionalFacilities::prepare(&world, 165, road, towns);
        let line = plan.rail().unwrap();
        assert_eq!(line.freight_route, FreightRoute::Agriculture);
        assert_eq!(local_loading_end(line), Some(LoadingEnd::First));
        let (timetable, _) = Train::line_timetable(line, 165).unwrap();
        let period = 2 * timetable.trip + timetable.layover_a + timetable.layover_b;
        let arrival = (period - timetable.offset) % period;
        let departure = arrival + timetable.layover_a - 1;
        let camera = Camera {
            x: -6538.0,
            z: -690.0,
            altitude: 52.0,
            heading: -1.16,
            pitch: 0.30,
            bank: 0.0,
        };
        let freight = |ticks| {
            Train::at(line, 165, ticks, camera, (128, 128))
                .unwrap()
                .cars
                .map(|car| match car?.vehicle {
                    Vehicle::Wagon { variant, loaded, .. } => Some((variant, loaded)),
                    _ => None,
                })
        };
        let empty = freight(arrival);
        let loaded = freight(departure);
        assert!(empty.iter().flatten().count() >= 2);
        assert_eq!(empty.map(|car| car.map(|(variant, _)| (variant, true))), loaded);
        assert!(empty.into_iter().flatten().all(|(_, cargo)| !cargo));
        assert_eq!(Train::line_position(line, 165, arrival), Train::line_position(line, 165, departure));
    }

    #[test]
    fn aggregate_reaches_a_single_freight_port_from_its_served_inland_end() {
        use RailAnchorKind::{PassengerPort, Port, RoadTerminus, Town};
        assert_eq!(aggregate_loading_end(FreightRoute::Aggregate, [Town, Port]), Some(LoadingEnd::First));
        assert_eq!(aggregate_loading_end(FreightRoute::Aggregate, [Port, RoadTerminus]), Some(LoadingEnd::Second));
        assert_eq!(aggregate_loading_end(FreightRoute::Aggregate, [Port, Port]), None);
        assert_eq!(aggregate_loading_end(FreightRoute::Aggregate, [Town, PassengerPort]), None);
        for route in [FreightRoute::Quarry, FreightRoute::Agriculture, FreightRoute::Forest,
                      FreightRoute::Mixed, FreightRoute::Intermodal] {
            assert_eq!(aggregate_loading_end(route, [Town, Port]), None);
        }
    }

    #[test]
    fn retained_freight_terminals_keep_their_supported_services() {
        use crate::facilities::RegionalFacilities;
        use crate::rail::RailService;
        use crate::scenery::DistrictRegion;
        use crate::settlement::Terminus;

        for (seed, x, z, expected) in [
            (46, -5, -3, FreightRoute::Intermodal),
            (130, 11, 4, FreightRoute::Liquid),
            (1872, -2, 5, FreightRoute::Perishable),
            (1959, -3, -3, FreightRoute::Mixed),
        ] {
            let world = World::new(seed);
            let district = DistrictRegion::build(&world, x, z);
            let road = district.road().unwrap();
            let towns = [Terminus::Start, Terminus::End].map(|end| district.town(end));
            let plan = RegionalFacilities::prepare(&world, seed, road, towns);
            let line = plan.rail().expect("existing freight capture");
            assert_eq!(line.service, RailService::Freight);
            assert_eq!(line.freight_route, expected);
        }
    }

    #[test]
    fn suitable_transfer_work_uses_the_terminal_catchment() {
        let town = RailAnchorKind::Town;
        assert_eq!(inland_service(5, 1, 0, town, 3), FreightRoute::Forest);
        assert_eq!(inland_service(1, 1, 5, town, 3), FreightRoute::Quarry);
        assert_eq!(inland_service(1, 6, 0, town, 3), FreightRoute::Agriculture);
        assert_eq!(inland_service(1, 6, 0, town, 8), FreightRoute::Perishable);
        assert_eq!(inland_service(1, 1, 0, town, 7), FreightRoute::Mill);
    }

    #[test]
    fn port_to_port_routes_are_independent_of_station_order() {
        let world = World::new(19);
        let a = RailAnchor {
            x: -129.0,
            z: 31.0,
            kind: RailAnchorKind::Port,
        };
        let b = RailAnchor {
            x: -532.0,
            z: 60.0,
            kind: RailAnchorKind::Port,
        };
        let service = freight_route(&world, [a, b]);
        assert_eq!(service, freight_route(&world, [b, a]));
        assert!(matches!(
            service,
            FreightRoute::Intermodal | FreightRoute::Automotive | FreightRoute::Liquid
        ));
    }

    #[test]
    fn active_farm_parcels_justify_grain_work_if_they_are_clear_of_development() {
        let seed = 31;
        let world = World::new(seed);
        let port = RailAnchor {
            x: 3200.0,
            z: 400.0,
            kind: RailAnchorKind::Port,
        };
        let field_anchors = (-10..=10).flat_map(|nz| {
            (-10..=10).map(move |nx| RailAnchor {
                x: nx as f32 * 94.0,
                z: nz as f32 * 94.0,
                kind: RailAnchorKind::RoadTerminus,
            })
        });
        let terminal = field_anchors
            .into_iter()
            .find(|&terminal| {
                contextual_freight_route(
                    &world,
                    seed,
                    [port, terminal],
                    FreightRoute::Intermodal,
                    |_, _| false,
                ) == FreightRoute::Agriculture
            })
            .expect("no farmed terminal catchment in the fixture");
        assert_eq!(
            contextual_freight_route(
                &world,
                seed,
                [terminal, port],
                FreightRoute::Agriculture,
                |_, _| true
            ),
            FreightRoute::Mixed,
        );
        assert_eq!(
            contextual_freight_route(
                &world,
                seed,
                [terminal, port],
                FreightRoute::Aggregate,
                |_, _| false
            ),
            FreightRoute::Aggregate,
        );
    }

    #[test]
    fn passenger_harbors_do_not_take_dispatch_from_a_freight_port() {
        let world = World::new(1);
        let cargo = RailAnchor {
            x: -129.0,
            z: 31.0,
            kind: RailAnchorKind::Port,
        };
        let ferry = RailAnchor {
            x: -532.0,
            z: 60.0,
            kind: RailAnchorKind::PassengerPort,
        };
        let service =
            contextual_freight_route(&world, 1, [cargo, ferry], FreightRoute::Mixed, |_, _| {
                panic!("attempted to survey a passenger endpoint as an inland freight terminal")
            });
        assert_eq!(service, FreightRoute::Mixed);
    }
}
