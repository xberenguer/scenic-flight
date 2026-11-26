// North American freight-car classes, cargo and dispatch patterns.
// Equipment references: BNSF, Greenbrier, TTX, and Union Pacific.
// Dimensions are represented at Scenic Flight scale.

const CONFIGURATIONS: u8 = 5;
const JOBS_PER_FAMILY: u8 = 5;
const CARS_PER_FAMILY: u8 = JOBS_PER_FAMILY * CONFIGURATIONS;
pub const DESIGN_COUNT: u8 = 10 * CARS_PER_FAMILY;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Family {
    Well,
    Boxcar,
    CoveredHopper,
    Tank,
    Deck,
    Autorack,
    OpenBulk,
    Centerbeam,
    Coil,
    Reefer,
}

const FAMILIES: [Family; 10] = [
    Family::Well,
    Family::Boxcar,
    Family::CoveredHopper,
    Family::Tank,
    Family::Deck,
    Family::Autorack,
    Family::OpenBulk,
    Family::Centerbeam,
    Family::Coil,
    Family::Reefer,
];

impl Family {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Well => "Well car",
            Self::Boxcar => "Boxcar",
            Self::CoveredHopper => "Covered hopper",
            Self::Tank => "Tank car",
            Self::Deck => "Flatcar",
            Self::Autorack => "Autorack",
            Self::OpenBulk => "Open bulk",
            Self::Centerbeam => "Centerbeam",
            Self::Coil => "Coil car",
            Self::Reefer => "Refrigerated boxcar",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FreightRoute {
    Mixed,
    Intermodal,
    Agriculture,
    Liquid,
    Mill,
    Forest,
    Automotive,
    Perishable,
    Quarry,
    Aggregate,
}

impl FreightRoute {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Mixed => "Mixed freight",
            Self::Intermodal => "Container terminal",
            Self::Agriculture => "Agricultural bulk",
            Self::Liquid => "Liquid terminal",
            Self::Mill => "Steel mill",
            Self::Forest => "Forest products",
            Self::Automotive => "Vehicle distribution",
            Self::Perishable => "Cold storage",
            Self::Quarry => "Quarry and mining",
            Self::Aggregate => "Aggregate transfer",
        }
    }
}

const CARGO: [[&str; 5]; 10] = [
    [
        "Empty well returning to a terminal",
        "Short maritime container",
        "Two short maritime containers",
        "Domestic container",
        "Double-stacked containers",
    ],
    [
        "Paper rolls",
        "Palletized merchandise",
        "Packaged beverages",
        "Bagged ingredients",
        "Lumber under cover",
    ],
    ["Grain", "Cement", "Plastic pellets", "Dry sand", "Sugar"],
    [
        "Food oils",
        "Asphalt",
        "Ethanol",
        "Liquefied petroleum gas",
        "Water",
    ],
    [
        "Empty bulkhead flat returning for another load",
        "Long pipe",
        "Heavy machinery",
        "Structural beams",
        "Logs",
    ],
    [
        "Finished vehicles in an open bi-level rack",
        "SUVs and pickups",
        "Passenger cars",
        "Finished vehicles in articulated bi-level racks",
        "Delivery trucks and tall vans in an enclosed single-level carrier",
    ],
    [
        "Coal",
        "Crushed aggregate",
        "Metal ore",
        "Scrap steel",
        "Steel plate",
    ],
    [
        "Empty centerbeam returning to a lumber yard",
        "Wrapped lumber bundles",
        "Dimensional studs",
        "Wallboard stacks",
        "Packaged railroad ties",
    ],
    [
        "Hot-rolled coils in open cradles",
        "Single weather-protected coil",
        "Two covered coils",
        "Three covered coils",
        "Heavy coils in open transverse cradles",
    ],
    [
        "Fresh produce",
        "Frozen food",
        "Chilled meat",
        "Dairy",
        "Temperature-sensitive beverages",
    ],
];

// Each load has an origin or destination in the simulated freight economy.
// The gallery pairs this description with the equipment fitted for that load.
const ASSIGNMENTS: [[&str; 5]; 10] = [
    [
        "Reposition to a terminal for its next load",
        "Transfer one overseas shipment between ship and rail",
        "Consolidate two small overseas shipments in one well",
        "Carry a domestic shipment between truck and rail hubs",
        "Carry two levels of containers on a high-clearance route",
    ],
    [
        "Carry paper from the mill to a printer or converter",
        "Deliver palletized goods to a distribution warehouse",
        "Keep packaged drinks dry on the way to a wholesaler",
        "Supply a food producer with dry, bagged ingredients",
        "Keep milled lumber under cover on the way to a warehouse",
    ],
    [
        "Move loose grain from an elevator to a processor or export terminal",
        "Deliver powdered cement to a ready-mix depot",
        "Supply a plastics plant with resin pellets",
        "Deliver dry industrial sand to a plant",
        "Move bulk sugar to a packaging or food plant",
    ],
    [
        "Supply a food plant with liquid ingredients",
        "Deliver paving material to an asphalt terminal",
        "Move ethanol from a distillery to a fuel terminal",
        "Move liquefied gas in a pressure vessel",
        "Transfer bulk water between industrial terminals",
    ],
    [
        "Return an empty deck to a loading yard",
        "Deliver long pipe to a utility or construction depot",
        "Move equipment too large for an enclosed boxcar",
        "Deliver long steel sections from a mill to a fabricator",
        "Move unprocessed timber from a log landing to a mill",
    ],
    [
        "Move finished vehicles between a plant and an unloading ramp",
        "Deliver taller pickups and utility vehicles to a regional ramp",
        "Carry smaller finished cars at higher deck density",
        "Move a block of finished vehicles through a distribution corridor",
        "Deliver high-roof vans and light trucks to a vehicle terminal",
    ],
    [
        "Haul loose coal from a mine to a bulk terminal",
        "Move crushed stone from a quarry to a construction-material yard",
        "Haul dense ore from a mine to a mill or bulk terminal",
        "Deliver recyclable steel to a mill for remelting",
        "Move flat steel plate from a mill to a fabricator",
    ],
    [
        "Return the empty truss to a mill or lumber yard",
        "Move weather-wrapped lumber from a mill to a dealer",
        "Deliver bundled framing stock to a building-material yard",
        "Move stacked wallboard to a construction distributor",
        "Deliver packaged ties to a railroad materials yard",
    ],
    [
        "Move weather-tolerant hot-rolled coils from a mill",
        "Protect one coil on the way to a fabricator",
        "Move a paired shipment of weather-sensitive coils",
        "Move smaller covered coils in separate cradles",
        "Support heavy coils crosswise for delivery to a processor",
    ],
    [
        "Keep produce cool from packing house to market",
        "Maintain the freezer chain to a distribution warehouse",
        "Keep meat chilled through delivery to cold storage",
        "Move dairy products between refrigerated facilities",
        "Protect temperature-sensitive drinks in transit",
    ],
];

include!("freight_profiles.rs");

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Design {
    pub family: Family,
    pub job: u8,
    pub build: u8,
    pub cargo: &'static str,
    pub assignment: &'static str,
    pub equipment: &'static str,
    pub purpose: &'static str,
}

impl Design {
    pub const fn reference_url(self) -> &'static str {
        match self.family {
            Family::Well | Family::Deck | Family::Autorack | Family::Centerbeam => {
                "https://www.ttx.com/about/equipment/"
            }
            Family::Boxcar => "https://www.gbrx.com/boxcars/",
            Family::CoveredHopper => "https://www.gbrx.com/covered-hoppers/",
            Family::Tank => "https://www.gbrx.com/tank-cars/",
            Family::OpenBulk => "https://bnsf.com/ship/by-method/railcar-types",
            Family::Coil => "https://www.up.com/shipping/resources/rail-equipment/definitions",
            Family::Reefer => "https://www.gbrx.com/railcars/72-3-refrigerated-boxcar/",
        }
    }

    /// Coupler-to-coupler length in flight-world units. The equipment keeps
    /// relative prototype lengths within the compact regional train scale.
    pub const fn body_length(self) -> f32 {
        let lengths = match (self.family, self.job) {
            (Family::Well, _) => [11.1, 12.0, 13.4, 13.0, 13.8],
            (Family::Boxcar, _) => [12.2, 13.8, 11.7, 13.3, 15.5],
            (Family::CoveredHopper, 1 | 3) => [10.6, 11.2, 12.0, 11.6, 12.4],
            (Family::CoveredHopper, 2) => [12.3, 13.4, 14.3, 14.7, 15.7],
            (Family::CoveredHopper, _) => [11.5, 12.6, 14.0, 13.3, 14.7],
            (Family::Tank, 0) => [11.7, 12.8, 13.9, 12.5, 14.1],
            (Family::Tank, 1) => [10.7, 11.5, 12.5, 11.9, 12.9],
            (Family::Tank, 2) => [12.2, 13.1, 14.2, 13.6, 14.6],
            (Family::Tank, 3) => [12.9, 14.2, 15.3, 14.9, 15.7],
            (Family::Tank, _) => [10.8, 11.9, 13.0, 12.4, 13.3],
            (Family::Deck, 1 | 3) => [13.5, 14.5, 15.6, 14.2, 16.5],
            (Family::Deck, 2) => [12.0, 13.5, 14.4, 12.7, 15.9],
            (Family::Deck, _) => [12.1, 13.9, 14.8, 13.4, 16.0],
            (Family::Autorack, 3) => [14.1, 14.7, 15.1, 15.7, 16.2],
            (Family::Autorack, _) => [15.2, 15.8, 16.4, 16.0, 16.8],
            (Family::OpenBulk, 2) => [10.6, 11.0, 11.6, 11.2, 12.1],
            (Family::OpenBulk, 0 | 1) => [11.5, 12.2, 13.3, 12.6, 14.0],
            (Family::OpenBulk, _) => [11.8, 13.0, 14.4, 13.6, 15.4],
            (Family::Centerbeam, _) => [13.0, 14.1, 15.6, 14.8, 16.3],
            (Family::Coil, _) => [10.9, 11.7, 13.0, 12.0, 12.5],
            (Family::Reefer, _) => [13.1, 14.3, 15.1, 14.7, 16.2],
        };
        lengths[self.build as usize]
    }

    pub const fn kind(self) -> &'static str {
        match (self.family, self.job) {
            (Family::Deck, 1) => "Pipe flatcar",
            (Family::Deck, 2) => "Machinery flatcar",
            (Family::Deck, 3) => "Structural-steel flatcar",
            (Family::Deck, 4) => "Log flatcar",
            (Family::Deck, _) => "Bulkhead flatcar",
            (Family::Autorack, 0) => "Open bi-level autorack",
            (Family::Autorack, 1) => "Enclosed bi-level autorack",
            (Family::Autorack, 2) => "Enclosed tri-level autorack",
            (Family::Autorack, 3) => "Articulated bi-level autorack",
            (Family::Autorack, _) => "Single-level vehicle carrier",
            (Family::OpenBulk, 0..=2) => "Open-top hopper",
            (Family::OpenBulk, _) => "Gondola",
            (Family::Coil, 0) => "Open longitudinal coil car",
            (Family::Coil, 4) => "Open transverse coil car",
            (Family::Coil, _) => "Covered coil car",
            _ => self.family.name(),
        }
    }
}

pub const fn design(variant: u8) -> Design {
    let family = (variant % DESIGN_COUNT / CARS_PER_FAMILY) as usize;
    let job = variant % CARS_PER_FAMILY / CONFIGURATIONS;
    let build = variant % CONFIGURATIONS;
    let profile = &PROFILES[family][job as usize][build as usize];
    Design {
        family: FAMILIES[family],
        job,
        build,
        cargo: CARGO[family][job as usize],
        assignment: ASSIGNMENTS[family][job as usize],
        equipment: profile.name,
        purpose: profile.purpose,
    }
}

const fn job(family: Family, index: u8) -> u8 {
    family as u8 * CARS_PER_FAMILY + index * CONFIGURATIONS
}

pub(super) fn grain_elevator_variant(car: u64) -> u8 {
    let cargo = car ^ car.rotate_right(19);
    job(Family::CoveredHopper, 0) + (cargo % u64::from(CONFIGURATIONS)) as u8
}

/// Select a compatible cargo job and a stable equipment frame for this wagon.
/// On mixed trains neighboring wagons can share an interchange block.
pub fn choose_variant(route: FreightRoute, train: u64, wagon: usize, car: u64) -> u8 {
    use Family::*;
    use FreightRoute::*;

    const MERCHANDISE: [u8; 4] = [
        job(Boxcar, 0),
        job(Boxcar, 1),
        job(Boxcar, 2),
        job(Boxcar, 3),
    ];
    const STEEL: [u8; 10] = [
        job(Coil, 0),
        job(Coil, 1),
        job(Coil, 2),
        job(Coil, 3),
        job(Coil, 4),
        job(Deck, 0),
        job(Deck, 1),
        job(Deck, 3),
        job(OpenBulk, 3),
        job(OpenBulk, 4),
    ];
    const LUMBER: [u8; 7] = [
        job(Centerbeam, 0),
        job(Centerbeam, 1),
        job(Centerbeam, 2),
        job(Centerbeam, 3),
        job(Centerbeam, 4),
        job(Boxcar, 4),
        job(Deck, 4),
    ];
    const MILL_INBOUND: [u8; 2] = [job(OpenBulk, 3), job(Deck, 0)];
    const MILL_OUTBOUND: [u8; 8] = [
        job(Coil, 0),
        job(Coil, 1),
        job(Coil, 2),
        job(Coil, 3),
        job(Coil, 4),
        job(Deck, 1),
        job(Deck, 3),
        job(OpenBulk, 4),
    ];
    const FOREST_EMPTIES: [u8; 2] = [job(Centerbeam, 0), job(Deck, 0)];
    const AGRICULTURAL: [u8; 3] = [job(CoveredHopper, 0), job(CoveredHopper, 4), job(Boxcar, 3)];
    const INDUSTRIAL: [u8; 5] = [
        job(CoveredHopper, 1),
        job(CoveredHopper, 2),
        job(CoveredHopper, 3),
        job(Deck, 2),
        job(Deck, 0),
    ];
    const MIXED: [&[u8]; 5] = [&MERCHANDISE, &STEEL, &LUMBER, &AGRICULTURAL, &INDUSTRIAL];

    let manifest = train ^ train.rotate_right(23);
    let cargo = car ^ car.rotate_right(19);
    let job = match route {
        Mixed => {
            let block = manifest.rotate_right((wagon / 2 * 11) as u32);
            let work = MIXED[match block & 15 {
                0..=6 => 0,
                7..=9 => 4,
                10..=11 => 1,
                12..=13 => 2,
                _ => 3,
            }];
            pick(work, cargo >> 22)
        }
        Intermodal if manifest & 7 == 0 => job(Well, 0),
        Intermodal => job(Well, 1 + ((cargo >> 22) & 3) as u8),
        Agriculture => job(CoveredHopper, if manifest & 7 == 0 { 4 } else { 0 }),
        Liquid => job(Tank, (manifest % 5) as u8),
        Mill if manifest & 3 == 0 => pick(&MILL_INBOUND, cargo >> 22),
        Mill => pick(&MILL_OUTBOUND, cargo >> 22),
        Forest if manifest & 7 == 0 => pick(&FOREST_EMPTIES, cargo >> 22),
        Forest => pick(&LUMBER[1..], cargo >> 22),
        Automotive => job(Autorack, (manifest % 5) as u8),
        Perishable => job(Reefer, (manifest % 5) as u8),
        Quarry => job(OpenBulk, (manifest % 3) as u8),
        Aggregate => job(OpenBulk, 1),
    };
    job + (cargo % u64::from(CONFIGURATIONS)) as u8
}

fn pick(jobs: &[u8], key: u64) -> u8 {
    jobs[(key % jobs.len() as u64) as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROUTES: [FreightRoute; 10] = [
        FreightRoute::Mixed,
        FreightRoute::Intermodal,
        FreightRoute::Agriculture,
        FreightRoute::Liquid,
        FreightRoute::Mill,
        FreightRoute::Forest,
        FreightRoute::Automotive,
        FreightRoute::Perishable,
        FreightRoute::Quarry,
        FreightRoute::Aggregate,
    ];

    #[test]
    fn every_purpose_and_equipment_pair_is_distinct_and_dispatched() {
        let mut reached = [false; DESIGN_COUNT as usize];
        for route in ROUTES {
            for number in 0..2048_u64 {
                let train = number.wrapping_mul(0x9e37_79b9_7f4a_7c15);
                for car in 1..7 {
                    let wagon = example_car_key(train, car);
                    reached[choose_variant(route, train, car as usize - 1, wagon) as usize] = true;
                }
            }
        }
        for index in 0..DESIGN_COUNT {
            let car = design(index);
            assert!(
                reached[index as usize],
                "undispatched freight car {index}: {car:?}"
            );
            assert!(car.job < JOBS_PER_FAMILY && car.build < CONFIGURATIONS);
            assert!(
                !car.cargo.is_empty() && !car.assignment.is_empty() && !car.equipment.is_empty()
            );
            assert!(!car.purpose.is_empty());
            assert!(car.reference_url().starts_with("https://"));
            assert!((10.6..=16.8).contains(&car.body_length()));
            assert!((0..index).all(|previous| {
                let previous = design(previous);
                car.equipment != previous.equipment && car.purpose != previous.purpose
            }));
            let first = index - car.build;
            assert!((first..index)
                .all(|previous| { design(previous).body_length() != car.body_length() }));
        }
    }

    #[test]
    fn dedicated_consists_keep_their_cargo_and_use_compatible_equipment() {
        for (route, family) in [
            (FreightRoute::Agriculture, Family::CoveredHopper),
            (FreightRoute::Automotive, Family::Autorack),
            (FreightRoute::Liquid, Family::Tank),
            (FreightRoute::Perishable, Family::Reefer),
            (FreightRoute::Quarry, Family::OpenBulk),
            (FreightRoute::Aggregate, Family::OpenBulk),
        ] {
            for train in 0..128 {
                let loaded = design(choose_variant(route, train, 0, 0));
                assert_eq!(loaded.family, family);
                assert!(route != FreightRoute::Agriculture || [0, 4].contains(&loaded.job));
                assert!(route != FreightRoute::Quarry || loaded.job <= 2);
                assert!(route != FreightRoute::Aggregate || loaded.job == 1);
                for car in 1..8 {
                    let wagon = design(choose_variant(
                        route,
                        train,
                        car,
                        example_car_key(train, car as u64),
                    ));
                    assert_eq!((loaded.family, loaded.job), (wagon.family, wagon.job));
                }
            }
        }
    }

    fn example_car_key(train: u64, car: u64) -> u64 {
        let independent = train ^ car.wrapping_mul(0x9e37_79b9_7f4a_7c15);
        (independent ^ (independent >> 29)).wrapping_mul(0xbf58_476d_1ce4_e5b9)
    }

    #[test]
    fn intermodal_dispatch_mixes_loaded_wells_between_full_empty_returns() {
        let mut found_return = false;
        let mut found_mixed_loaded = false;
        for train in 0..64 {
            let cars = [1, 2, 3].map(|car| {
                design(choose_variant(
                    FreightRoute::Intermodal,
                    train,
                    car as usize - 1,
                    example_car_key(train, car),
                ))
            });
            assert!(cars.iter().all(|car| car.family == Family::Well));
            let empty = cars[0].job == 0;
            assert!(cars.iter().all(|car| (car.job == 0) == empty));
            found_return |= empty;
            found_mixed_loaded |= !empty && cars.iter().any(|car| car.job != cars[0].job);
        }
        assert!(found_return && found_mixed_loaded);
    }

    #[test]
    fn long_mixed_trains_carry_more_than_one_interchange_block() {
        assert!((0..256_u64).any(|number| {
            let train = number.wrapping_mul(0x9e37_79b9_7f4a_7c15);
            let cars = [0, 1, 2, 3].map(|wagon| {
                design(choose_variant(
                    FreightRoute::Mixed,
                    train,
                    wagon,
                    example_car_key(train, wagon as u64),
                ))
            });
            cars[..2].iter().all(|car| car.family == Family::Boxcar)
                && cars[2..].iter().all(|car| car.family == Family::Coil)
        }));
    }

    #[test]
    fn grain_elevators_use_all_grain_hoppers_with_no_sugar_equipment() {
        let mut visited = [false; CONFIGURATIONS as usize];
        for key in 0..64 {
            let car = design(grain_elevator_variant(key));
            assert_eq!((car.family, car.cargo), (Family::CoveredHopper, "Grain"));
            visited[car.build as usize] = true;
        }
        assert!(visited.into_iter().all(|seen| seen));
    }
}
