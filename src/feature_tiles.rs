//! Bounded tile preparation for sparse, world-anchored scenery.

#[derive(Clone, Copy)]
struct Tile {
    location: [i32; 2],
    resident: u8,
}

const EMPTY: u8 = u8::MAX;
const UNSEEN: Tile = Tile { location: [i32::MIN; 2], resident: EMPTY };

pub struct FeatureTiles<T, const SIDE: usize, const CAPACITY: usize> {
    map: [[Tile; SIDE]; SIDE],
    residents: [Option<T>; CAPACITY],
    center: [i32; 2],
    next: usize,
    tile_size: f32,
}

impl<T, const SIDE: usize, const CAPACITY: usize> FeatureTiles<T, SIDE, CAPACITY> {
    pub const fn new(tile_size: i32) -> Self {
        assert!(SIDE > 0 && SIDE % 2 == 1 && CAPACITY < EMPTY as usize && tile_size > 0);
        Self {
            map: [[UNSEEN; SIDE]; SIDE], residents: [const { None }; CAPACITY],
            center: [0; 2], next: 0, tile_size: tile_size as f32,
        }
    }

    pub fn reset(&mut self, x: f32, z: f32, prepare: impl FnMut(i32, i32) -> Option<T>) {
        self.map.fill([UNSEEN; SIDE]);
        for item in &mut self.residents { *item = None; }
        self.next = 0;
        self.follow(x, z, SIDE * SIDE, prepare);
    }

    pub fn follow(&mut self, x: f32, z: f32, budget: usize, prepare: impl FnMut(i32, i32) -> Option<T>) {
        self.follow_in_order::<false>(x, z, budget, prepare);
    }

    pub(crate) fn follow_centered(
        &mut self,
        x: f32,
        z: f32,
        budget: usize,
        prepare: impl FnMut(i32, i32) -> Option<T>,
    ) {
        if budget >= SIDE * SIDE {
            // A complete sweep can visit every tile. Keep the established
            // resident order when the fixed site capacity is reached.
            self.next = 0;
            self.follow_in_order::<false>(x, z, budget, prepare);
        } else {
            self.follow_in_order::<true>(x, z, budget, prepare);
        }
    }

    fn follow_in_order<const CENTERED: bool>(
        &mut self,
        x: f32,
        z: f32,
        budget: usize,
        mut prepare: impl FnMut(i32, i32) -> Option<T>,
    ) {
        self.recenter(x, z);
        for _ in 0..budget {
            let Some([tx, tz]) = self.next_in_order::<CENTERED>() else { break };
            let site = prepare(tx, tz);
            self.complete([tx, tz], site);
        }
    }

    pub(crate) fn recenter(&mut self, x: f32, z: f32) {
        let center = [x, z].map(|value| tile_axis(value, self.tile_size));
        if self.center != center {
            self.center = center;
            self.next = 0;
        }
    }

    /// Leaves a missing slot untouched until its survey has returned.
    pub(crate) fn next_unknown_centered(&mut self) -> Option<[i32; 2]> {
        self.next_in_order::<true>()
    }

    fn next_in_order<const CENTERED: bool>(&mut self) -> Option<[i32; 2]> {
        let half = SIDE as i32 / 2;
        while self.next < SIDE * SIDE {
            let [dx, dz] = if CENTERED {
                centered_offset(self.next)
            } else {
                [(self.next % SIDE) as i32 - half, (self.next / SIDE) as i32 - half]
            };
            let [tx, tz] = [self.center[0] + dx, self.center[1] + dz];
            if self.map[wrap::<SIDE>(tz)][wrap::<SIDE>(tx)].location != [tx, tz] {
                return Some([tx, tz]);
            }
            self.next += 1;
        }
        None
    }

    pub(crate) fn wants(&self, [x, z]: [i32; 2]) -> bool {
        self.center[0].abs_diff(x) <= SIDE as u32 / 2
            && self.center[1].abs_diff(z) <= SIDE as u32 / 2
            && self.map[wrap::<SIDE>(z)][wrap::<SIDE>(x)].location != [x, z]
    }

    pub(crate) fn complete(&mut self, [x, z]: [i32; 2], site: Option<T>) -> bool {
        if !self.wants([x, z]) { return false; }
        let slot = &mut self.map[wrap::<SIDE>(z)][wrap::<SIDE>(x)];
        if let Some(previous) = self.residents.get_mut(slot.resident as usize) {
            *previous = None;
        }
        *slot = Tile { location: [x, z], resident: EMPTY };
        if let Some(place) = site {
            if let Some(index) = self.residents.iter().position(Option::is_none) {
                self.residents[index] = Some(place);
                slot.resident = index as u8;
            }
        }
        true
    }

    /// Let a separately retained site be surveyed again after it leaves range.
    pub(crate) fn forget_empty(&mut self, [x, z]: [i32; 2]) {
        let slot = &mut self.map[wrap::<SIDE>(z)][wrap::<SIDE>(x)];
        if slot.location == [x, z] && slot.resident == EMPTY {
            *slot = UNSEEN;
            self.next = 0;
        }
    }

    pub fn at(&self, x: f32, z: f32) -> Option<&T> {
        let [tx, tz] = [x, z].map(|value| tile_axis(value, self.tile_size));
        let tile = &self.map[wrap::<SIDE>(tz)][wrap::<SIDE>(tx)];
        if tile.location != [tx, tz] { return None; }
        self.residents.get(tile.resident as usize).and_then(Option::as_ref)
    }

    pub fn sites(&self) -> impl Iterator<Item = &T> {
        self.residents.iter().flatten()
    }

    pub(crate) fn sites_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.residents.iter_mut().flatten()
    }
}

/// Visit the center and then each surrounding square ring. The first nine
/// indices cover the center tile and all of its immediate neighbors.
fn centered_offset(index: usize) -> [i32; 2] {
    if index == 0 { return [0, 0]; }
    let mut edge = 2;
    while index >= (edge + 1) * (edge + 1) {
        edge += 2;
    }
    let step = index - (edge - 1) * (edge - 1);
    let radius = (edge / 2) as i32;
    let along = (step % edge) as i32;
    match step / edge {
        0 => [along - radius, -radius],
        1 => [radius, along - radius],
        2 => [radius - along, radius],
        _ => [-radius, radius - along],
    }
}

fn wrap<const SIDE: usize>(coordinate: i32) -> usize {
    coordinate.rem_euclid(SIDE as i32) as usize
}

fn tile_axis(value: f32, tile_size: f32) -> i32 {
    let scalar = value / tile_size;
    let integer = scalar as i32;
    integer.saturating_sub(i32::from(scalar < integer as f32))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn centered_sweeps_start_with_all_nine_local_tiles() {
        let mut cache = FeatureTiles::<(i32, i32), 7, 49>::new(32);
        let center = [-3, 2];
        let mut first = [(0, 0); 9];
        let mut count = 0;
        cache.follow_centered(-90.0, 72.0, first.len(), |x, z| {
            first[count] = (x, z);
            count += 1;
            Some((x, z))
        });
        assert_eq!(first[0], (center[0], center[1]));
        for z in center[1] - 1..=center[1] + 1 {
            for x in center[0] - 1..=center[0] + 1 {
                assert!(first.contains(&(x, z)));
            }
        }
        let next = cache.next_unknown_centered().unwrap();
        assert_eq!(cache.next_unknown_centered(), Some(next));
        assert!(cache.complete(next, Some((next[0], next[1]))));
        cache.follow_centered(-90.0, 72.0, 49, |x, z| Some((x, z)));
        assert_eq!(cache.sites().count(), 49);
    }

    #[test]
    fn complete_sweeps_keep_the_existing_capacity_policy() {
        let mut ordinary = FeatureTiles::<(i32, i32), 7, 3>::new(32);
        let mut bounded = FeatureTiles::<(i32, i32), 7, 3>::new(32);
        ordinary.follow(-90.0, 72.0, 49, |x, z| Some((x, z)));
        bounded.follow_centered(-90.0, 72.0, 49, |x, z| Some((x, z)));
        assert!(ordinary.sites().eq(bounded.sites()));
    }

    #[test]
    fn signed_tile_boundaries_preserve_ready_sites_and_discard_reused_slots() {
        let mut cache = FeatureTiles::<(i32, i32), 3, 9>::new(32);
        let site = |x, z| Some((x, z));
        cache.reset(-0.3, -0.3, site);
        assert_eq!(cache.at(-0.01, -0.01), Some(&(-1, -1)));
        assert_eq!(cache.at(-32.01, 0.0), Some(&(-2, 0)));
        assert_eq!(cache.at(32.01, 0.0), None);
        cache.follow(0.01, -0.3, 1, site);
        assert_eq!(cache.at(-0.01, 0.0), Some(&(-1, 0)));
        for _ in 0..3 { cache.follow(0.01, -0.3, 1, site); }
        assert_eq!(cache.at(32.01, 0.0), Some(&(1, 0)));
        assert_eq!(cache.at(-32.01, 0.0), None);
        assert_eq!(cache.sites().count(), 9);
    }
}
