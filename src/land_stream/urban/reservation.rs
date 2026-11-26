//! An owned, bounded snapshot of the scenery an urban survey must preserve.

use super::REGION;
use crate::facility_stream::FacilityStream;
use crate::highway_stream::HighwayStream;
use crate::local_routes::LocalRouteStream;
use crate::natural_stream::NaturalStream;
use crate::rail_stream::RailStream;
use crate::river_stream::RiverStream;
use crate::summit_stream::SummitStream;

const SIDE: usize = 64;
const CELL: f32 = REGION / SIDE as f32;

#[derive(Clone)]
pub struct UrbanReservations {
    region: [i32; 2],
    revision: u32,
    origin: [f32; 2],
    blocked: [u64; SIDE],
    waterways: [u64; SIDE],
    next: usize,
}

impl UrbanReservations {
    pub fn new(region: [i32; 2], revision: u32) -> Self {
        Self {
            region,
            revision,
            origin: region.map(|axis| axis as f32 * REGION),
            blocked: [0; SIDE],
            waterways: [0; SIDE],
            next: 0,
        }
    }

    /// The authority callback tests the entire rectangle, including its edges.
    /// Stop a sweep if any of its authorities change, then begin a new revision.
    pub fn advance(
        &mut self,
        budget: usize,
        mut reserved: impl FnMut([f32; 4]) -> bool,
    ) -> bool {
        let end = (SIDE * SIDE).min(self.next.saturating_add(budget));
        for index in self.next..end {
            let (column, row) = (index % SIDE, index / SIDE);
            let x = self.origin[0] + column as f32 * CELL;
            let z = self.origin[1] + row as f32 * CELL;
            if reserved([x, z, x + CELL, z + CELL]) {
                self.blocked[row] |= 1 << column;
            }
        }
        self.next = end;
        self.next == SIDE * SIDE
    }

    fn scan_intersections(
        &mut self,
        column: usize,
        row: usize,
        width: usize,
        intersects: &mut impl FnMut([f32; 4]) -> bool,
    ) {
        if width == 1 && self.blocked[row] & (1 << column) != 0 {
            return;
        }
        let x = self.origin[0] + column as f32 * CELL;
        let z = self.origin[1] + row as f32 * CELL;
        let extent = width as f32 * CELL;
        if !intersects([x, z, x + extent, z + extent]) {
            return;
        }
        if width == 1 {
            self.blocked[row] |= 1 << column;
            return;
        }
        let half = width / 2;
        for y in [row, row + half] {
            for x in [column, column + half] {
                self.scan_intersections(x, y, half, intersects);
            }
        }
    }

    /// Mark every cell whose closed rectangle intersects the footprint.
    fn reserve_intersection(&mut self, [left, near, right, far]: [f32; 4]) {
        let [x, z] = self.origin;
        if left > right || near > far || right < x || far < z || left > x + REGION || near > z + REGION {
            return;
        }
        let first = |offset: f32| {
            let value = offset / CELL;
            let index = value as usize;
            index.saturating_sub(usize::from(index as f32 == value)).min(SIDE - 1)
        };
        let last = |offset: f32| ((offset / CELL) as usize).min(SIDE - 1);
        let mask = (u64::MAX << first(left - x)) & (u64::MAX >> (SIDE - 1 - last(right - x)));
        for row in &mut self.blocked[first(near - z)..=last(far - z)] {
            *row |= mask;
        }
    }

    /// Reserve a footprint independently of the incremental sweep.
    pub fn reserve_area(&mut self, [left, near, right, far]: [f32; 4]) {
        let [x, z] = self.origin;
        if left > right || near > far || right < x || far < z || left >= x + REGION || near >= z + REGION {
            return;
        }
        let index = |value: f32| ((value / CELL) as usize).min(SIDE - 1);
        let mask = (u64::MAX << index(left - x)) & (u64::MAX >> (SIDE - 1 - index(right - x)));
        for row in &mut self.blocked[index(near - z)..=index(far - z)] {
            *row |= mask;
        }
    }

    pub(super) fn matches(&self, region: [i32; 2], revision: u32) -> bool {
        self.next == SIDE * SIDE && self.region == region && self.revision == revision
    }

    /// A newly occupied cell can invalidate a survey. Released cells can
    /// remain vacant until the next survey without invalidating its geometry.
    pub(super) fn protects(&self, current: &Self) -> bool {
        self.matches(current.region, current.revision)
            && current.matches(self.region, self.revision)
            && self.blocked.iter().zip(&current.blocked).all(|(&old, &new)| new & !old == 0)
    }

    pub(super) fn blocks(&self, x: f32, z: f32) -> bool {
        let [x, z] = [x - self.origin[0], z - self.origin[1]];
        if !(0.0..REGION).contains(&x) || !(0.0..REGION).contains(&z) {
            return true;
        }
        let (column, row) = ((x / CELL) as usize, (z / CELL) as usize);
        row * SIDE + column >= self.next || self.blocked[row] & (1 << column) != 0
    }

    pub(super) fn near_waterway(&self, x: f32, z: f32) -> bool {
        let [x, z] = [x - self.origin[0], z - self.origin[1]];
        if !(0.0..REGION).contains(&x) || !(0.0..REGION).contains(&z) {
            return false;
        }
        let (column, row) = ((x / CELL) as usize, (z / CELL) as usize);
        let left = column.saturating_sub(2);
        let right = (column + 2).min(SIDE - 1);
        let mask = (u64::MAX << left) & (u64::MAX >> (SIDE - 1 - right));
        self.waterways[row.saturating_sub(2)..=(row + 2).min(SIDE - 1)]
            .iter().any(|bank| bank & mask != 0)
    }

    pub(super) fn overlaps(&self, [left, near, right, far]: [f32; 4]) -> bool {
        let [x, z] = self.origin;
        if self.next != SIDE * SIDE || left < x || near < z || right >= x + REGION || far >= z + REGION {
            return true;
        }
        let cell = |offset: f32| ((offset / CELL) as usize).min(SIDE - 1);
        let mask = (u64::MAX << cell(left - x)) & (u64::MAX >> (SIDE - 1 - cell(right - x)));
        self.blocked[cell(near - z)..=cell(far - z)].iter().any(|row| row & mask != 0)
    }
}

/// Borrowed surveyed scenery. Only the compact completed mask crosses to the
/// town worker. Additional ground features can reserve their own footprints.
pub struct UrbanAuthorities<'a> {
    pub facilities: &'a FacilityStream<8>,
    pub rivers: &'a RiverStream<3>,
    pub routes: &'a LocalRouteStream<8>,
    pub highways: &'a HighwayStream<4>,
    pub rail: &'a RailStream,
    pub natural: &'a NaturalStream,
    pub summits: Option<&'a SummitStream>,
}

impl UrbanAuthorities<'_> {
    pub fn ready(&self, [x, z]: [i32; 2]) -> bool {
        let owner = (x, z);
        self.facilities.planned(owner).is_some()
            && self.rivers.planned(owner).is_some()
            && self.routes.planned(owner).is_some()
    }

    pub fn start(&self, [x, z]: [i32; 2], revision: u32) -> Option<UrbanReservations> {
        if !self.ready([x, z]) {
            return None;
        }
        let mut mask = UrbanReservations::new([x, z], revision);
        for river in self.rivers.scenery_plans() {
            for bounds in river.scenery_rectangles() {
                mask.reserve_intersection(bounds);
            }
            for (a, b) in river.bank_chords() {
                mask.scan_intersections(0, 0, SIDE, &mut |area| river.bank_intersects(a, b, area));
            }
        }
        mask.waterways = mask.blocked;
        self.facilities.urban_footprints(|bounds| mask.reserve_area(bounds));
        self.routes.urban_footprints(|bounds| mask.reserve_area(bounds));
        Some(mask)
    }

    pub fn advance(&self, mask: &mut UrbanReservations, budget: usize, mut extra: impl FnMut([f32; 4]) -> bool) -> bool {
        mask.advance(budget, |bounds| self.intersects(bounds) || extra(bounds))
    }

    /// Rasterize the existing reservations once. Test each bank chord
    /// independently so a busy river is never re-sampled for every city cell.
    pub fn complete(&self, mask: &mut UrbanReservations) {
        for bounds in self.rail.reservation_footprints().chain(self.natural.active_bounds()) {
            mask.reserve_intersection(bounds);
        }
        if let Some(summits) = self.summits {
            summits.conflicts_with(|bounds| {
                mask.reserve_intersection(bounds);
                false
            });
        }
        mask.scan_intersections(0, 0, SIDE, &mut |area| self.highways.reserves(area));
        mask.next = SIDE * SIDE;
    }

    fn intersects(&self, bounds: [f32; 4]) -> bool {
        self.rivers.reserves_scenery(bounds)
            || self.highways.reserves(bounds)
            || self.rail.reserves(bounds)
            || self.natural.reserves(bounds)
            || self.summits.is_some_and(|summits| summits.reserves(bounds))
    }
}
