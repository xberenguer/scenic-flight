//! Planned development envelopes for metropolitan extensions. Streets can
//! continue through the open edge to connect surrounding neighborhoods.

pub const COLUMNS: usize = 19;
pub const ROWS: usize = 17;

#[derive(Clone, Copy, Debug)]
pub struct NeighborhoodLayout {
    pub name: &'static str,
    node_rows: [u32; ROWS],
}

impl NeighborhoodLayout {
    pub const ALL: [Self; 25] = [
        Self::plan(
            "Broad side crescent",
            [4, 1, 0, 1, 4],
            [1, 0, 1, 3, 5],
            [6, 3, 0, 3, 6],
        ),
        Self::plan(
            "West ridge terraces",
            [5, 2, 0, 0, 1],
            [0, 1, 2, 3, 5],
            [3, 1, 1, 3, 6],
        ),
        Self::plan(
            "East ridge terraces",
            [0, 1, 3, 4, 5],
            [5, 2, 0, 0, 1],
            [6, 3, 1, 1, 3],
        ),
        Self::plan(
            "Two-junction saddle",
            [0, 3, 2, 0, 4],
            [0, 2, 3, 0, 4],
            [5, 2, 5, 1, 5],
        ),
        Self::plan(
            "Long central spur",
            [2, 0, 2, 4, 5],
            [2, 0, 2, 4, 5],
            [7, 4, 0, 4, 7],
        ),
        Self::plan(
            "Stepped western arm",
            [5, 4, 3, 1, 0],
            [0, 1, 1, 2, 4],
            [1, 0, 2, 4, 6],
        ),
        Self::plan(
            "Stepped eastern arm",
            [0, 1, 1, 2, 4],
            [5, 4, 3, 1, 0],
            [6, 4, 2, 0, 1],
        ),
        Self::plan(
            "Greenbelt fork",
            [3, 0, 0, 0, 2],
            [3, 0, 0, 1, 2],
            [4, 0, 5, 0, 4],
        ),
        Self::plan(
            "Asymmetric fan",
            [4, 3, 1, 0, 0],
            [5, 1, 0, 0, 2],
            [2, 1, 0, 1, 4],
        ),
        Self::plan(
            "Tapered avenue",
            [0, 1, 2, 3, 4],
            [0, 1, 2, 3, 4],
            [4, 2, 1, 2, 4],
        ),
        Self::plan(
            "Wide rear gardens",
            [4, 2, 1, 0, 1],
            [4, 2, 1, 0, 1],
            [2, 0, 1, 0, 2],
        ),
        Self::plan(
            "Upper shoulder crescent",
            [1, 1, 0, 1, 4],
            [4, 3, 0, 0, 2],
            [5, 4, 2, 0, 3],
        ),
        Self::plan(
            "Western cutback",
            [0, 0, 1, 4, 5],
            [3, 0, 0, 2, 3],
            [7, 4, 0, 0, 4],
        ),
        Self::plan(
            "Eastern cutback",
            [3, 0, 0, 2, 3],
            [0, 0, 1, 4, 5],
            [4, 0, 0, 4, 7],
        ),
        Self::plan(
            "Yard district wedge",
            [0, 3, 0, 1, 2],
            [4, 3, 2, 1, 2],
            [2, 0, 2, 5, 6],
        ),
        Self::plan(
            "Sheltered garden elbow",
            [5, 1, 1, 3, 4],
            [1, 1, 3, 0, 2],
            [4, 2, 1, 0, 5],
        ),
        Self::plan(
            "Pocket-valley edge",
            [1, 0, 2, 0, 4],
            [3, 1, 2, 1, 4],
            [5, 0, 4, 1, 6],
        ),
        Self::plan(
            "Double outer shoulder",
            [5, 1, 0, 2, 1],
            [5, 1, 0, 2, 1],
            [4, 1, 3, 1, 4],
        ),
        Self::plan(
            "Shallow sloping bench",
            [2, 0, 1, 2, 2],
            [1, 0, 1, 2, 3],
            [2, 2, 3, 4, 6],
        ),
        Self::plan(
            "Avenue and side spur",
            [3, 2, 0, 0, 1],
            [0, 2, 4, 4, 4],
            [1, 3, 0, 3, 6],
        ),
        Self::plan(
            "Green inlet steps",
            [0, 0, 3, 2, 4],
            [4, 1, 0, 2, 3],
            [6, 1, 4, 0, 5],
        ),
        Self::plan(
            "Hill-foot crescent",
            [4, 0, 0, 1, 3],
            [4, 0, 0, 1, 3],
            [5, 3, 1, 3, 5],
        ),
        Self::plan(
            "Swale-side dogleg",
            [0, 0, 2, 4, 4],
            [4, 3, 0, 1, 1],
            [7, 4, 2, 0, 2],
        ),
        Self::plan(
            "Terraced garden lobes",
            [2, 0, 2, 0, 3],
            [2, 0, 2, 0, 3],
            [3, 0, 3, 1, 3],
        ),
        Self::plan(
            "Long outer arch",
            [5, 3, 0, 0, 2],
            [4, 2, 0, 0, 2],
            [3, 1, 0, 1, 3],
        ),
    ];

    const fn plan(name: &'static str, west: [u8; 5], east: [u8; 5], rear: [u8; 5]) -> Self {
        let mut node_rows = [0; ROWS];
        let mut row = 0;
        while row < ROWS {
            let left = interpolate(west, row, ROWS - 1);
            let right = COLUMNS - 1 - interpolate(east, row, ROWS - 1);
            let mut col = left;
            while col <= right {
                if row + interpolate(rear, col, COLUMNS - 1) < ROWS {
                    node_rows[row] |= 1 << col;
                }
                col += 1;
            }
            row += 1;
        }
        Self { name, node_rows }
    }

    pub fn allows_block(&self, col: usize, row: usize) -> bool {
        if col >= COLUMNS - 1 || row >= ROWS - 1 {
            return false;
        }
        let corners = 3 << col;
        let upper = self.node_rows[row] & corners;
        let lower = self.node_rows[row + 1] & corners;
        upper.count_ones() + lower.count_ones() >= 3
    }

    pub(super) fn for_town(key: u32) -> &'static Self {
        &Self::ALL[(key.rotate_right(19) as usize) % Self::ALL.len()]
    }
}

const fn interpolate(values: [u8; 5], coordinate: usize, span: usize) -> usize {
    let scaled = 4 * coordinate;
    let index = scaled / span;
    if index == 4 {
        return values[4] as usize;
    }
    let offset = scaled % span;
    (values[index] as usize * (span - offset) + values[index + 1] as usize * offset + span / 2)
        / span
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plans_have_distinct_edges_and_keep_room_for_large_connected_districts() {
        for (i, design) in NeighborhoodLayout::ALL.iter().enumerate() {
            assert!(
                !NeighborhoodLayout::ALL[..i]
                    .iter()
                    .any(|previous| previous.node_rows == design.node_rows)
            );
            assert!(
                design
                    .node_rows
                    .iter()
                    .map(|row| row.count_ones())
                    .sum::<u32>()
                    >= 180,
                "{}",
                design.name
            );
            let core = ((1 << 7) - 1) << 6;
            assert!(design.node_rows[..=6].iter().all(|row| row & core == core));
            assert!(
                design
                    .node_rows
                    .windows(2)
                    .take_while(|rows| rows[1] != 0)
                    .all(|rows| rows[0] & rows[1] != 0)
            );
        }
    }
}
