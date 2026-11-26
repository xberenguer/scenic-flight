use crate::math::mix;
use crate::rail::{RailAnchor, RailAnchorKind};
use crate::world::{Surface, World};

use super::FreightRoute;

/// Assign a transfer service once when the grade is surveyed. The terrain
/// around the inland terminal suggests what its catchment can supply.
pub(crate) fn freight_route(world: &World, ends: [RailAnchor; 2]) -> FreightRoute {
    let terminal = ends.into_iter().find(|end| end.kind != RailAnchorKind::Port);
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
    for (dx, dz) in [(-80.0, -80.0), (0.0, -100.0), (80.0, -80.0),
        (-100.0, 0.0), (0.0, 0.0), (100.0, 0.0),
        (-80.0, 80.0), (0.0, 100.0), (80.0, 80.0)]
    {
        match world.sample(terminal.x + dx, terminal.z + dz).surface {
            Surface::Forest => forest += 1,
            Surface::Grass => pasture += 1,
            Surface::Rock | Surface::Desert | Surface::ScreeFine | Surface::ScreeCoarse => mineral += 1,
            _ => {}
        }
    }
    inland_service(forest, pasture, mineral, terminal.kind, roll)
}

fn inland_service(forest: u8, pasture: u8, mineral: u8, terminal: RailAnchorKind, roll: u8) -> FreightRoute {
    if forest >= 4 && roll < 10 {
        FreightRoute::Forest
    } else if mineral >= 4 && roll < 10 {
        FreightRoute::Quarry
    } else if pasture >= 5 && roll < 10 {
        if roll < 7 { FreightRoute::Agriculture } else { FreightRoute::Perishable }
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
        let a = RailAnchor { x: -129.0, z: 31.0, kind: RailAnchorKind::Port };
        let b = RailAnchor { x: -532.0, z: 60.0, kind: RailAnchorKind::Port };
        let service = freight_route(&world, [a, b]);
        assert_eq!(service, freight_route(&world, [b, a]));
        assert!(matches!(service, FreightRoute::Intermodal | FreightRoute::Automotive | FreightRoute::Liquid));
    }
}
