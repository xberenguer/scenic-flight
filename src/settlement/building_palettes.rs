//! Paint, roofing, and masonry materials shared by the game and artwork.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BuildingPalette {
    pub wall: [u8; 3],
    pub roof: [u8; 3],
}

const fn material(wall: [u8; 3], roof: [u8; 3]) -> BuildingPalette {
    BuildingPalette { wall, roof }
}

pub const TOWN: [BuildingPalette; 25] = [
    material([199, 184, 156], [123, 73, 55]),
    material([222, 216, 197], [88, 115, 127]),
    material([181, 117, 89], [91, 57, 52]),
    material([160, 167, 155], [82, 103, 92]),
    material([229, 205, 165], [161, 95, 60]),
    material([236, 231, 213], [34, 90, 160]),
    material([153, 155, 152], [71, 86, 105]),
    material([203, 154, 103], [144, 119, 72]),
    material([181, 154, 143], [101, 69, 86]),
    material([157, 165, 123], [71, 101, 76]),
    material([176, 106, 88], [139, 73, 53]),
    material([206, 195, 160], [132, 104, 71]),
    material([156, 176, 179], [55, 99, 118]),
    material([227, 189, 160], [161, 81, 64]),
    material([139, 123, 107], [91, 73, 66]),
    material([212, 220, 219], [49, 115, 156]),
    material([211, 177, 124], [127, 86, 52]),
    material([164, 149, 163], [93, 82, 118]),
    material([185, 174, 143], [103, 103, 73]),
    material([197, 124, 103], [72, 63, 66]),
    material([142, 156, 148], [63, 97, 102]),
    material([217, 205, 181], [107, 130, 136]),
    material([145, 106, 92], [120, 61, 71]),
    material([236, 219, 192], [167, 127, 82]),
    material([184, 191, 155], [98, 120, 82]),
];

pub const RIVER: [BuildingPalette; 25] = [
    material([239, 238, 226], [40, 103, 171]),
    material([184, 167, 148], [78, 107, 117]),
    material([157, 146, 125], [107, 130, 118]),
    material([201, 157, 133], [116, 68, 47]),
    material([212, 209, 193], [43, 122, 142]),
    material([213, 186, 147], [141, 88, 65]),
    material([149, 173, 152], [94, 122, 100]),
    material([225, 232, 222], [50, 92, 137]),
    material([174, 134, 116], [121, 77, 83]),
    material([196, 204, 204], [92, 133, 156]),
    material([218, 214, 165], [146, 132, 82]),
    material([190, 182, 169], [95, 102, 118]),
    material([240, 224, 211], [42, 74, 139]),
    material([193, 123, 92], [123, 84, 77]),
    material([133, 149, 147], [67, 102, 119]),
    material([179, 157, 120], [153, 102, 63]),
    material([209, 167, 146], [161, 92, 86]),
    material([157, 166, 175], [83, 96, 136]),
    material([218, 193, 172], [103, 102, 93]),
    material([184, 171, 155], [132, 126, 117]),
    material([194, 193, 153], [114, 124, 73]),
    material([170, 139, 124], [107, 58, 69]),
    material([191, 210, 201], [58, 127, 115]),
    material([212, 177, 107], [165, 122, 58]),
    material([229, 224, 212], [56, 111, 152]),
];

pub(crate) fn from_id(id: usize) -> &'static BuildingPalette {
    if id < TOWN.len() {
        &TOWN[id]
    } else {
        &RIVER[id - TOWN.len()]
    }
}

pub(crate) fn region(world: &crate::world::World, x: f32, z: f32) -> u8 {
    let sample = world.sample(x, z);
    if world.coast_signal(x, z) < 0.18 {
        1 // whitewash, slate and blue-painted coastal roofs
    } else if matches!(
        sample.surface,
        crate::world::Surface::Desert | crate::world::Surface::Sand
    ) {
        2 // warm clay and local sandstone
    } else if sample.elevation > 37.0 {
        3 // darker stone, slate and weathered timber
    } else {
        0
    }
}

pub(crate) fn town_finish(model: u8, region: u8) -> u8 {
    const COAST: [u8; 10] = [5, 12, 15, 21, 25, 29, 32, 34, 37, 49];
    const DRY: [u8; 10] = [0, 2, 4, 7, 10, 11, 13, 16, 19, 23];
    const UPLAND: [u8; 10] = [3, 6, 8, 9, 14, 17, 18, 20, 22, 24];
    let i = usize::from(model);
    if model % 5 == 0 {
        return model;
    }
    match region {
        1 => COAST[(i * 7 + i / 10) % COAST.len()],
        2 => DRY[(i * 7 + i / 10) % DRY.len()],
        3 => UPLAND[(i * 7 + i / 10) % UPLAND.len()],
        _ => model,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_wall_and_roof_has_a_distinct_rgb565_color() {
        let rgb565 =
            |[r, g, b]: [u8; 3]| ((r as u16 >> 3) << 11) | ((g as u16 >> 2) << 5) | (b as u16 >> 3);
        for i in 0..TOWN.len() + RIVER.len() {
            for j in 0..i {
                assert_ne!(
                    rgb565(from_id(i).wall),
                    rgb565(from_id(j).wall),
                    "wall {i} {j}"
                );
                assert_ne!(
                    rgb565(from_id(i).roof),
                    rgb565(from_id(j).roof),
                    "roof {i} {j}"
                );
            }
        }
    }
}
