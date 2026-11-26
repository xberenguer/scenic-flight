//! Satellite neighborhoods that share the surveyed highway joining two regions.

use super::{
    Area, Building, DRY_HEIGHT, ENTRY_COL, Form, Frontage, Local, SPACING, STREET_HALF_WIDTH,
    SettlementSample, SettlementSurface, foundation, horizontal_length,
};
use crate::highway_links::HighwayLink;
use crate::math::mix;
use crate::roads::{ROAD_REGION_SIZE, RoadPoint};
use crate::world::{SEA_LEVEL, World};

const CENTRE: f32 = ENTRY_COL as f32 * SPACING;
const PROBE_SPACING: f32 = 94.0;
pub(crate) const MAX_SITES: usize = 6;
const MAX_LOTS: usize = 12;
pub(crate) type FrontageAnchors = [Option<Anchor>; MAX_SITES];

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Anchor {
    point: [f32; 2],
    direction: [f32; 2],
    key: u64,
}

pub(crate) fn collect(anchors: &mut FrontageAnchors, region: [i32; 2], link: &HighwayLink) {
    let [a, b] = link.contacts();
    if a.region != region && b.region != region {
        return;
    }
    let address = |[x, z]: [i32; 2]| mix((u64::from(x as u32) << 32) | u64::from(z as u32));
    collect_anchors(
        &mut anchors[..3],
        region,
        link.polyline(),
        address(a.region) ^ address(b.region),
        48.0,
    );
}

pub(crate) fn collect_regional(
    anchors: &mut FrontageAnchors,
    region: [i32; 2],
    road: &[RoadPoint],
) {
    let [x, z] = region;
    let key = mix((u64::from(x as u32) << 32) | u64::from(z as u32)) ^ 0xbb0e_feaf_b646_c84d;
    collect_anchors(&mut anchors[3..], region, road, key, 107.0);
}

fn collect_anchors(
    anchors: &mut [Option<Anchor>],
    region: [i32; 2],
    road: &[RoadPoint],
    route_key: u64,
    mut next: f32,
) {
    let origin = region.map(|v| v as f32 * ROAD_REGION_SIZE as f32);
    let mut distance = 0.0;
    for pair in road.windows(2) {
        let [from, to] = [pair[0], pair[1]];
        let vector = [to.x - from.x, to.z - from.z];
        let length = horizontal_length(vector[0], vector[1]);
        if length < 0.001 {
            continue;
        }
        while next <= distance + length {
            let t = (next - distance) / length;
            let point = [from.x + vector[0] * t, from.z + vector[1] * t];
            if point
                .iter()
                .zip(origin)
                .all(|(&p, o)| (4.0..ROAD_REGION_SIZE as f32 - 4.0).contains(&(p - o)))
            {
                let anchor = Anchor {
                    point,
                    direction: vector.map(|v| v / length),
                    key: mix(route_key ^ next.to_bits() as u64),
                };
                if let Some(slot) = anchors
                    .iter()
                    .position(|saved| saved.is_none_or(|saved| anchor.key < saved.key))
                {
                    anchors.copy_within(slot..anchors.len() - 1, slot + 1);
                    anchors[slot] = Some(anchor);
                }
            }
            next += PROBE_SPACING;
        }
        distance += length;
    }
}

#[derive(Clone, Copy, Debug)]
struct Layout {
    name: &'static str,
    // Width to the left and right at three intersections along the entrance.
    arms: [[u8; 2]; 3],
    use_kind: Use,
}

#[derive(Clone, Copy, Debug)]
enum Use {
    Houses,
    Duplexes,
    Shops,
    Workshops,
    Mixed,
}

const fn layout(name: &'static str, arms: [[u8; 2]; 3], use_kind: Use) -> Layout {
    Layout {
        name,
        arms,
        use_kind,
    }
}

use Use::*;
const LAYOUTS: [Layout; 25] = [
    layout("Offset cottage rows", [[18, 0], [29, 17], [0, 23]], Houses),
    layout("Long garden lanes", [[0, 24], [28, 0], [27, 25]], Houses),
    layout("Paired courts", [[22, 22], [0, 0], [21, 21]], Houses),
    layout("Shoulder terraces", [[32, 12], [20, 0], [0, 12]], Houses),
    layout(
        "Back-to-back cottages",
        [[0, 0], [28, 28], [23, 29]],
        Houses,
    ),
    layout("Duplex crescent", [[15, 12], [26, 18], [32, 10]], Duplexes),
    layout("Duplex comb", [[0, 26], [0, 27], [0, 24]], Duplexes),
    layout("Setback duplexes", [[0, 0], [34, 0], [25, 19]], Duplexes),
    layout("Pair of short mews", [[19, 0], [0, 0], [0, 24]], Duplexes),
    layout("Corner villas", [[26, 0], [21, 20], [0, 0]], Duplexes),
    layout("Market crossroads", [[24, 29], [0, 16], [23, 0]], Shops),
    layout("Setback shops", [[0, 0], [19, 26], [0, 24]], Shops),
    layout("Two shopping lanes", [[27, 0], [0, 21], [0, 0]], Shops),
    layout("Garden storefronts", [[19, 20], [0, 0], [0, 33]], Shops),
    layout("Corner main street", [[29, 24], [0, 12], [0, 0]], Shops),
    layout("Machine shops", [[18, 26], [27, 0], [0, 31]], Workshops),
    layout("Loading courts", [[30, 30], [0, 0], [0, 0]], Workshops),
    layout("Long workshop lane", [[0, 33], [0, 12], [0, 19]], Workshops),
    layout("Works at the back", [[0, 0], [0, 0], [27, 32]], Workshops),
    layout("Twin works yards", [[0, 20], [0, 0], [30, 0]], Workshops),
    layout(
        "Shops and garden rows",
        [[28, 26], [16, 0], [16, 28]],
        Mixed,
    ),
    layout(
        "Staggered village streets",
        [[12, 24], [26, 16], [12, 28]],
        Mixed,
    ),
    layout(
        "Housing behind workshops",
        [[30, 15], [19, 0], [29, 29]],
        Mixed,
    ),
    layout("Deep main street", [[0, 15], [0, 18], [32, 32]], Mixed),
    layout("Greenbelt corner", [[26, 0], [0, 0], [32, 25]], Mixed),
];

#[derive(Clone)]
pub(crate) struct CorridorDistrict {
    local: Local,
    layout: u8,
    arms: [[u8; 2]; 3],
    depth: f32,
    occupied_lots: u16,
    lot_levels: [f32; MAX_LOTS],
    lot_variants: [u8; MAX_LOTS],
    bounds: [f32; 4],
}

const INTERSECTIONS: [f32; 3] = [24.0, 43.0, 62.0];

impl CorridorDistrict {
    pub(crate) fn survey(
        world: &World,
        region: [i32; 2],
        anchor: Anchor,
        occupied: &impl Fn(f32, f32) -> bool,
        highway: &impl Fn(f32, f32) -> bool,
    ) -> Option<Self> {
        let origin = region.map(|axis| axis as f32 * ROAD_REGION_SIZE as f32);
        let permitted = |x, z| {
            [x, z]
                .into_iter()
                .zip(origin)
                .all(|(v, o)| (0.5..ROAD_REGION_SIZE as f32 - 0.5).contains(&(v - o)))
                && !occupied(x, z)
        };
        let [dx, dz] = anchor.direction;
        let order = if anchor.key & 1 == 0 {
            [1.0, -1.0]
        } else {
            [-1.0, 1.0]
        };
        let mut best: Option<Self> = None;
        for side in order {
            let local = Local {
                x: anchor.point[0],
                z: anchor.point[1],
                fx: -dz * side,
                fz: dx * side,
            };
            let index = (anchor.key % LAYOUTS.len() as u64) as u8;
            let layout = &LAYOUTS[usize::from(index)];
            let open = |x, z| {
                let (wx, wz) = local.world(x, z);
                permitted(wx, wz)
                    && ((x - CENTRE).abs() <= STREET_HALF_WIDTH && z < 25.0 || !highway(wx, wz))
            };
            let mut result = Self {
                local,
                layout: index,
                arms: [[0; 2]; 3],
                depth: 0.0,
                occupied_lots: 0,
                lot_levels: [0.0; MAX_LOTS],
                lot_variants: [0; MAX_LOTS],
                bounds: [
                    f32::INFINITY,
                    f32::INFINITY,
                    f32::NEG_INFINITY,
                    f32::NEG_INFINITY,
                ],
            };
            for (row, ahead) in INTERSECTIONS.into_iter().enumerate() {
                if !street(
                    world,
                    local,
                    [CENTRE, result.depth],
                    [CENTRE, ahead + 2.0],
                    &open,
                ) {
                    break;
                }
                result.depth = ahead + 2.0;
                for (wing, direction) in [-1.0_f32, 1.0].into_iter().enumerate() {
                    let reach = layout.arms[row][wing];
                    if reach < 8
                        || !street(
                            world,
                            local,
                            [CENTRE, ahead],
                            [CENTRE + direction * f32::from(reach), ahead],
                            &open,
                        )
                    {
                        continue;
                    }
                    result.arms[row][wing] = reach;
                    for side in 0..2 {
                        let lot = row * 4 + wing * 2 + side;
                        result.lot_variants[lot] = mix(anchor.key ^ lot as u64) as u8;
                        let area = result.building(lot).area;
                        if f32::from(reach) + 5.0 < area.half_width * 2.0
                            || !area_clear(area, &open)
                        {
                            continue;
                        }
                        if let Some(base) = foundation(world, local, area, 1.8) {
                            result.lot_levels[lot] = base;
                            result.occupied_lots |= 1 << lot;
                        }
                    }
                }
            }
            if result.occupied_lots.count_ones() < 4 {
                continue;
            }
            result.bounds = result.compute_bounds();
            if best.as_ref().is_none_or(|prior| {
                result.occupied_lots.count_ones() > prior.occupied_lots.count_ones()
            }) {
                best = Some(result);
            }
        }
        best
    }

    pub(crate) fn name(&self) -> &'static str {
        LAYOUTS[usize::from(self.layout)].name
    }

    pub(crate) fn bounds(&self) -> [f32; 4] {
        self.bounds
    }

    pub(crate) fn sample(&self, x: f32, z: f32) -> Option<SettlementSample> {
        let [left, near, right, far] = self.bounds;
        if x < left || x > right || z < near || z > far {
            return None;
        }
        let (across, ahead) = self.local.grid(x, z);
        let centerline = across - CENTRE;
        if centerline.abs() <= STREET_HALF_WIDTH && (0.0..=self.depth).contains(&ahead)
            || INTERSECTIONS
                .into_iter()
                .zip(self.arms)
                .any(|(row, [left, right])| {
                    left + right > 0
                        && (ahead - row).abs() <= STREET_HALF_WIDTH
                        && (-f32::from(left)..=f32::from(right)).contains(&centerline)
                })
        {
            return Some(SettlementSample {
                surface: SettlementSurface::Street,
                elevation: None,
            });
        }
        self.lot_indices()
            .find_map(|slot| self.building(slot).sample(across, ahead))
    }

    fn lot_indices(&self) -> impl Iterator<Item = usize> + '_ {
        let mut bits = self.occupied_lots;
        core::iter::from_fn(move || {
            if bits == 0 {
                return None;
            }
            let slot = bits.trailing_zeros() as usize;
            bits &= bits - 1;
            Some(slot)
        })
    }

    fn building(&self, slot: usize) -> Building {
        let row = slot / 4;
        let wing = (slot / 2) % 2;
        let variant = self.lot_variants[slot];
        let kind = match LAYOUTS[usize::from(self.layout)].use_kind {
            Mixed if row == 0 => Shops,
            Mixed if row == 1 && self.layout == 22 => Workshops,
            Mixed => Houses,
            single => single,
        };
        let (half_width, height, form) = match kind {
            Houses => (5.0, 4.3, Form::Cottage(variant)),
            Duplexes => (7.2, 6.0, Form::Duplex(variant)),
            Shops => (7.8, 7.0, Form::Shop(Frontage::Near, variant)),
            Workshops => (8.5, 7.7, Form::Industrial(Frontage::Near, variant)),
            Mixed => unreachable!(),
        };
        let reach = f32::from(self.arms[row][wing]);
        let area = Area {
            x: CENTRE + [-1.0, 1.0][wing] * (reach * 0.48).max(half_width + 3.0),
            z: INTERSECTIONS[row] + [-6.15, 6.15][slot % 2],
            half_width,
            half_depth: 3.3,
        };
        Building::new(area, self.lot_levels[slot], height, form)
    }

    fn footprints(&self, mut visit: impl FnMut([f32; 2], [f32; 2])) {
        visit(
            [CENTRE - STREET_HALF_WIDTH, 0.0],
            [CENTRE + STREET_HALF_WIDTH, self.depth],
        );
        for (ahead, [left, right]) in INTERSECTIONS.into_iter().zip(self.arms) {
            if left + right > 0 {
                visit(
                    [CENTRE - f32::from(left), ahead - STREET_HALF_WIDTH],
                    [CENTRE + f32::from(right), ahead + STREET_HALF_WIDTH],
                );
            }
        }
        for slot in self.lot_indices() {
            let a = self.building(slot).area;
            visit(
                [a.x - a.half_width, a.z - a.half_depth],
                [a.x + a.half_width, a.z + a.half_depth],
            );
        }
    }

    fn compute_bounds(&self) -> [f32; 4] {
        let mut bounds = [
            f32::INFINITY,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NEG_INFINITY,
        ];
        self.footprints(|[left, near], [right, far]| {
            for a in [left, right] {
                for b in [near, far] {
                    let (x, z) = self.local.world(a, b);
                    bounds = [
                        bounds[0].min(x),
                        bounds[1].min(z),
                        bounds[2].max(x),
                        bounds[3].max(z),
                    ];
                }
            }
        });
        bounds
    }
}

fn area_clear(a: Area, open: &impl Fn(f32, f32) -> bool) -> bool {
    [-1.0, -0.5, 0.0, 0.5, 1.0].into_iter().all(|x| {
        [-1.0, 0.0, 1.0].into_iter().all(|z| {
            open(
                a.x + x * (a.half_width + 0.45),
                a.z + z * (a.half_depth + 0.45),
            )
        })
    })
}

fn street(
    world: &World,
    local: Local,
    a: [f32; 2],
    b: [f32; 2],
    open: &impl Fn(f32, f32) -> bool,
) -> bool {
    let [dx, dz] = [b[0] - a[0], b[1] - a[1]];
    let distance = horizontal_length(dx, dz);
    if distance < 0.01 {
        return false;
    }
    let steps = (distance / 2.25).max(1.0) as usize + 1;
    let spacing = distance / steps as f32;
    let mut prior = [0.0_f32; 3];
    for step in 0..=steps {
        let t = step as f32 / steps as f32;
        let mut heights = [0.0; 3];
        for (i, side) in [-STREET_HALF_WIDTH, 0.0, STREET_HALF_WIDTH]
            .into_iter()
            .enumerate()
        {
            let x = a[0] + dx * t - dz / distance * side;
            let z = a[1] + dz * t + dx / distance * side;
            let h = local.ground(world, x, z);
            if !open(x, z)
                || !(DRY_HEIGHT..=SEA_LEVEL + 51.0).contains(&h)
                || (step > 0 && (h - prior[i]).abs() > spacing * 0.18)
            {
                return false;
            }
            heights[i] = h;
        }
        if (heights[0] - heights[2]).abs() > 0.90 {
            return false;
        }
        prior = heights;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::roads::RoadNetwork;

    #[test]
    fn authored_layouts_have_distinct_street_footprints() {
        for (i, layout) in LAYOUTS.iter().enumerate() {
            assert!(
                LAYOUTS[..i]
                    .iter()
                    .all(|earlier| earlier.arms != layout.arms),
                "{}",
                layout.name
            );
        }
    }

    #[test]
    fn a_real_highway_supports_neighborhoods_on_both_sides_of_the_region_boundary() {
        let fixtures = [
            (31, [-5, -17], [-4, -17]),
            (7911, [-2, 3], [-2, 4]),
            (503_821, [6, 3], [6, 4]),
        ];
        let mut joined = false;
        for (seed, a, b) in fixtures {
            let w = World::new(seed);
            let roads = [a, b].map(|[x, z]| RoadNetwork::generate(&w, x, z).unwrap());
            let link = HighwayLink::fit(&w, &roads[0], &roads[1]).unwrap();
            let counts = [a, b].map(|region| {
                let mut anchors = [None; MAX_SITES];
                collect(&mut anchors, region, &link);
                anchors
                    .into_iter()
                    .flatten()
                    .filter_map(|anchor| {
                        let district = CorridorDistrict::survey(
                            &w,
                            region,
                            anchor,
                            &|x, z| roads.iter().any(|road| road.proximity(x, z).is_some()),
                            &|x, z| link.proximity(x, z).is_some(),
                        )?;
                        let [x, z] = anchor.point;
                        let [left, near, right, far] = district.bounds();
                        assert!(district.occupied_lots.count_ones() >= 4);
                        assert!(link.proximity(x, z).is_some());
                        assert_eq!(
                            district.sample(x, z).unwrap().surface,
                            SettlementSurface::Street
                        );
                        assert!(left >= (region[0] * ROAD_REGION_SIZE) as f32);
                        assert!(right <= ((region[0] + 1) * ROAD_REGION_SIZE) as f32);
                        assert!(near >= (region[1] * ROAD_REGION_SIZE) as f32);
                        assert!(far <= ((region[1] + 1) * ROAD_REGION_SIZE) as f32);
                        Some(district)
                    })
                    .count()
            });
            joined |= counts.into_iter().all(|count| count > 0);
        }
        assert!(joined, "no fixture developed a neighborhood on both sides");
    }
}
