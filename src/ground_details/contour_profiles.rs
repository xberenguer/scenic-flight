//! Cropped contour strips and grass margins within a surveyed lowland parcel.

use crate::world::Surface;

#[derive(Clone, Copy)]
pub struct ContourProfile {
    pub name: &'static str,
    headland: [u8; 2],
    across: [u8; 2],
    taper: [i8; 2],
    buffers: [u8; 3],
    widths: [u8; 3],
    cross_lane: [u8; 2],
    crops: [Surface; 4],
}

const G: Surface = Surface::FieldGreen;
const Y: Surface = Surface::FieldGold;
const S: Surface = Surface::FieldSoil;

impl ContourProfile {
    pub const ALL: [Self; 25] = [
        Self::new(
            "Three buffered strips",
            [5, 5],
            [6, 58],
            [0, 0],
            [20, 40, 64],
            [7, 7, 0],
            [0, 0],
            [S, S, S, S],
        ),
        Self::new(
            "Split hay shoulder",
            [5, 8],
            [6, 58],
            [-1, 1],
            [16, 36, 48],
            [6, 8, 7],
            [0, 0],
            [G, G, Y, S],
        ),
        Self::new(
            "Broad turn meadow",
            [8, 6],
            [5, 57],
            [-2, 2],
            [24, 43, 64],
            [12, 6, 0],
            [0, 0],
            [G, Y, S, S],
        ),
        Self::new(
            "Upper bank pasture",
            [7, 11],
            [11, 61],
            [0, 0],
            [28, 47, 64],
            [6, 9, 0],
            [0, 0],
            [Y, G, Y, S],
        ),
        Self::new(
            "Traction lane strips",
            [4, 7],
            [7, 59],
            [0, 0],
            [18, 33, 48],
            [6, 7, 7],
            [26, 3],
            [G, S, Y, G],
        ),
        Self::new(
            "Hay and fallow shelves",
            [6, 8],
            [8, 57],
            [1, -1],
            [19, 45, 64],
            [7, 12, 0],
            [0, 0],
            [Y, S, G, S],
        ),
        Self::new(
            "Three narrow barley steps",
            [4, 5],
            [8, 56],
            [0, 0],
            [19, 31, 43],
            [5, 5, 5],
            [0, 0],
            [Y, S, Y, G],
        ),
        Self::new(
            "Open ridge shoulder",
            [9, 13],
            [6, 59],
            [-2, -2],
            [20, 39, 64],
            [9, 12, 0],
            [0, 0],
            [G, Y, S, S],
        ),
        Self::new(
            "Angled turning strips",
            [5, 9],
            [6, 58],
            [3, -3],
            [23, 38, 50],
            [7, 6, 7],
            [0, 0],
            [Y, S, G, Y],
        ),
        Self::new(
            "Grass spine and hay belts",
            [7, 6],
            [6, 58],
            [0, 0],
            [17, 32, 49],
            [6, 16, 6],
            [0, 0],
            [Y, G, Y, S],
        ),
        Self::new(
            "Old grass terraces",
            [5, 6],
            [5, 59],
            [1, 1],
            [18, 42, 64],
            [13, 13, 0],
            [0, 0],
            [Y, G, G, S],
        ),
        Self::new(
            "Offset hill garden",
            [8, 6],
            [10, 60],
            [2, -1],
            [22, 34, 46],
            [7, 6, 7],
            [35, 3],
            [S, G, Y, G],
        ),
        Self::new(
            "Short west headland",
            [4, 13],
            [5, 56],
            [2, 0],
            [22, 41, 64],
            [6, 8, 0],
            [0, 0],
            [G, S, Y, S],
        ),
        Self::new(
            "Short east headland",
            [12, 4],
            [8, 60],
            [0, -2],
            [25, 44, 64],
            [8, 6, 0],
            [0, 0],
            [Y, G, S, S],
        ),
        Self::new(
            "Contour shelter belt",
            [6, 6],
            [7, 58],
            [0, 0],
            [15, 29, 50],
            [7, 15, 8],
            [0, 0],
            [G, Y, G, S],
        ),
        Self::new(
            "Wide stubble shoulder",
            [10, 5],
            [6, 57],
            [-1, 2],
            [29, 43, 64],
            [6, 10, 0],
            [0, 0],
            [S, G, Y, S],
        ),
        Self::new(
            "Diagonal fallow stock lane",
            [5, 8],
            [10, 60],
            [-3, 2],
            [24, 39, 50],
            [6, 9, 8],
            [39, 4],
            [S, Y, G, Y],
        ),
        Self::new(
            "Tucked upper plots",
            [10, 11],
            [6, 59],
            [1, -3],
            [19, 35, 50],
            [8, 7, 9],
            [0, 0],
            [G, Y, S, G],
        ),
        Self::new(
            "Narrow green ledges",
            [4, 6],
            [13, 58],
            [0, 2],
            [24, 36, 47],
            [8, 8, 8],
            [0, 0],
            [G, S, G, Y],
        ),
        Self::new(
            "Two long meadow strips",
            [7, 7],
            [10, 56],
            [0, 0],
            [32, 64, 64],
            [13, 0, 0],
            [0, 0],
            [G, Y, S, S],
        ),
        Self::new(
            "Half-length barley banks",
            [6, 8],
            [5, 59],
            [-2, 2],
            [16, 40, 51],
            [7, 11, 6],
            [22, 4],
            [Y, G, S, Y],
        ),
        Self::new(
            "Cross-track high ground",
            [8, 9],
            [8, 58],
            [2, 1],
            [23, 37, 64],
            [8, 8, 0],
            [39, 4],
            [S, G, Y, S],
        ),
        Self::new(
            "Fan-shaped oat contour",
            [6, 7],
            [5, 60],
            [-3, 3],
            [18, 34, 49],
            [7, 10, 7],
            [0, 0],
            [Y, Y, G, S],
        ),
        Self::new(
            "Asymmetric hay ledge",
            [10, 8],
            [10, 60],
            [-2, -2],
            [22, 43, 65],
            [10, 7, 0],
            [32, 3],
            [G, Y, S, S],
        ),
        Self::new(
            "Broad lower rotation",
            [6, 5],
            [6, 59],
            [1, -2],
            [19, 31, 64],
            [8, 11, 0],
            [27, 2],
            [S, G, Y, S],
        ),
    ];

    const fn new(
        name: &'static str,
        headland: [u8; 2],
        across: [u8; 2],
        taper: [i8; 2],
        buffers: [u8; 3],
        widths: [u8; 3],
        cross_lane: [u8; 2],
        crops: [Surface; 4],
    ) -> Self {
        Self {
            name,
            headland,
            across,
            taper,
            buffers,
            widths,
            cross_lane,
            crops,
        }
    }

    /// The second coordinate has already been rotated and curved along the slope.
    pub fn sample(self, along: f32, contour: f32) -> Surface {
        let shift = (contour - 32.0) * (1.0 / 28.0);
        let start = f32::from(self.headland[0]) + shift * f32::from(self.taper[0]);
        let end = 64.0 - f32::from(self.headland[1]) + shift * f32::from(self.taper[1]);
        if !(start..end).contains(&along)
            || !(f32::from(self.across[0])..f32::from(self.across[1])).contains(&contour)
            || (along - f32::from(self.cross_lane[0])).abs() < f32::from(self.cross_lane[1])
        {
            return Surface::Grass;
        }
        let mut crop = 0;
        for (buffer, width) in self.buffers.into_iter().zip(self.widths) {
            let distance = contour - f32::from(buffer);
            if distance.abs() < f32::from(width) * 0.25 {
                return Surface::Grass;
            }
            crop += usize::from(distance > 0.0);
        }
        self.crops[crop]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contour_strips_have_distinct_footprints_at_chase_camera_scale() {
        let mut seen = std::collections::HashSet::new();
        for profile in ContourProfile::ALL {
            let mut pattern = [0u16; 16];
            let mut populated = 0;
            for (row, mask) in pattern.iter_mut().enumerate() {
                for column in 0..16 {
                    let along = 2.0 + 4.0 * column as f32;
                    let across = 2.0 + 4.0 * row as f32;
                    if profile.sample(along, across) != Surface::Grass {
                        populated += 1;
                        *mask |= 1 << column;
                    }
                }
            }
            assert!(seen.insert(pattern), "{}", profile.name);
            assert!(
                populated >= 90,
                "{} has {populated} crop samples",
                profile.name
            );
            assert_eq!(profile.sample(0.0, 31.0), Surface::Grass);
            assert_eq!(profile.sample(64.0, 31.0), Surface::Grass);
            assert_eq!(profile.sample(32.0, 0.0), Surface::Grass);
            assert_eq!(profile.sample(32.0, 64.0), Surface::Grass);
        }
    }
}
