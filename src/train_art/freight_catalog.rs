// North American freight-car classes, cargo and dispatch patterns.
// Equipment references: Union Pacific Rail Equipment, BNSF Railcar Types,
// and CSX Railroad Equipment. Dimensions are represented at Scenic Flight scale.

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
        "Finished vehicles in articulated racks",
        "Delivery trucks and tall vans",
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

const EQUIPMENT: [[&str; 5]; 10] = [
    [
        "Short well",
        "Standard well",
        "Extended well",
        "Split-frame well",
        "Heavy-end well",
    ],
    [
        "Single sliding door",
        "Double sliding doors",
        "Compact plug door",
        "Offset plug door",
        "High-cube body",
    ],
    [
        "Compact two-bay body",
        "Standard three-bay body",
        "Four-bay centerflow body",
        "Deep three-bay body",
        "Five-bay high-capacity body",
    ],
    [
        "Short tank",
        "Standard tank",
        "Long tank",
        "Insulated jacket",
        "High-capacity tank",
    ],
    [
        "Low flat deck",
        "End bulkheads",
        "Raised ends",
        "Depressed deck",
        "Reinforced deck",
    ],
    [
        "Compact rack frame",
        "Standard rack",
        "Long corrugated-side rack",
        "Reinforced rack",
        "High-clearance rack",
    ],
    [
        "Short body",
        "Standard body",
        "Long body",
        "Reinforced ribs",
        "High-capacity body",
    ],
    [
        "Short truss",
        "Standard truss",
        "Long truss",
        "Solid spine",
        "Tall bulkheads",
    ],
    [
        "Compact cradle",
        "Raised side sill",
        "Long cradle",
        "High-sill cradle",
        "Heavy-duty cradle",
    ],
    [
        "Compact end refrigeration unit",
        "Standard end refrigeration unit",
        "Extended double doors",
        "Underfloor refrigeration unit",
        "High-cube body",
    ],
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Design {
    pub family: Family,
    pub job: u8,
    pub build: u8,
    pub cargo: &'static str,
    pub equipment: &'static str,
}

impl Design {
    pub const fn kind(self) -> &'static str {
        match (self.family, self.job) {
            (Family::Deck, 2) => "Machinery flatcar",
            (Family::Deck, 4) => "Log flatcar",
            (Family::Deck, _) => "Bulkhead flatcar",
            (Family::OpenBulk, 0..=2) => "Open-top hopper",
            (Family::OpenBulk, _) => "Gondola",
            _ => self.family.name(),
        }
    }
}

pub const fn design(variant: u8) -> Design {
    let family = (variant % DESIGN_COUNT / CARS_PER_FAMILY) as usize;
    let job = variant % CARS_PER_FAMILY / CONFIGURATIONS;
    let build = variant % CONFIGURATIONS;
    Design {
        family: FAMILIES[family],
        job,
        build,
        cargo: CARGO[family][job as usize],
        equipment: EQUIPMENT[family][build as usize],
    }
}

const fn job(family: Family, index: u8) -> u8 {
    family as u8 * CARS_PER_FAMILY + index * CONFIGURATIONS
}

/// The route selects a compatible cargo job. Each car's key then picks its frame.
/// The keys are stable for a full trip, including a stop or direction change.
pub fn choose_variant(route: FreightRoute, train: u64, car: u64) -> u8 {
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
    const MILL_INBOUND: [u8; 3] = [job(OpenBulk, 3), job(Coil, 0), job(Deck, 0)];
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
            let work = MIXED[(manifest % MIXED.len() as u64) as usize];
            pick(work, cargo >> 22)
        }
        Intermodal if manifest & 7 == 0 => job(Well, 0),
        Intermodal => job(Well, 1 + ((cargo >> 22) & 3) as u8),
        Agriculture => pick(&AGRICULTURAL, manifest),
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
                for car in 1..4 {
                    let wagon = example_car_key(train, car);
                    reached[choose_variant(route, train, wagon) as usize] = true;
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
            assert!(!car.cargo.is_empty() && !car.equipment.is_empty());
            assert!((0..index).all(|previous| {
                let previous = design(previous);
                (car.kind(), car.cargo, car.equipment)
                    != (previous.kind(), previous.cargo, previous.equipment)
            }));
        }
    }

    #[test]
    fn dedicated_consists_keep_their_cargo_and_use_compatible_equipment() {
        for (route, family) in [
            (FreightRoute::Automotive, Family::Autorack),
            (FreightRoute::Liquid, Family::Tank),
            (FreightRoute::Perishable, Family::Reefer),
            (FreightRoute::Quarry, Family::OpenBulk),
            (FreightRoute::Aggregate, Family::OpenBulk),
        ] {
            for train in 0..128 {
                let loaded = design(choose_variant(route, train, 0));
                assert_eq!(loaded.family, family);
                assert!(route != FreightRoute::Quarry || loaded.job <= 2);
                assert!(route != FreightRoute::Aggregate || loaded.job == 1);
                for car in 1..8 {
                    let wagon = design(choose_variant(route, train, car));
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
}
