#[allow(dead_code)]
mod math {
    include!(env!("SCENIC_MATH_SOURCE"));
}

#[allow(dead_code)]
mod render {
    pub use scenic_flight::render::{Camera, ScreenProjection, ScreenRect};
    include!(env!("SCENIC_FLOOR_SOURCE"));
}

use scenic_flight::{sky, world, world_mesh};

#[allow(dead_code)]
mod artwork {
    include!(env!("SCENIC_DESERT_SOURCE"));

    use std::{collections::BTreeSet, fs::File, io::{BufWriter, Write}, path::Path};

    const SIDE: usize = 96;
    const GRID: usize = 16;
    const ATLAS_SIDE: usize = SIDE * GRID;
    const BACKGROUND: [u8; 3] = [19, 24, 30];

    fn render_variant(kind: PropKind, variant: u8) -> [u16; SIDE * SIDE] {
        // The fixed view and scale retain the proportions of all three kinds.
        let camera = Camera { x: 3.1, z: -4.3, altitude: 3.35, heading: -0.624, pitch: 0.36, bank: 0.0 };
        let daylight = crate::sky::Sky::from_time(41, 0, 0.5);
        let prop = Prop { origin: [0.0; 3], size: 1.0, kind, variant, opacity: 1.0 };
        let mut scene = DesertProps { props: [None; 72], region: None };
        scene.props[0] = Some(prop);
        let mut pixels = [0; SIDE * SIDE];
        let mut depth = [u16::MAX; SIDE * SIDE];

        let mut mesh = WorldMesh::new(&mut pixels, &mut depth, (SIDE, SIDE), camera, &daylight).unwrap();
        scene.draw(&mut mesh);
        pixels
    }

    fn rgb(pixel: u16) -> [u8; 3] {
        if pixel == 0 { return BACKGROUND; }
        [
            (((pixel >> 11) & 31) * 255 / 31) as u8,
            (((pixel >> 5) & 63) * 255 / 63) as u8,
            ((pixel & 31) * 255 / 31) as u8,
        ]
    }

    pub fn write(out: &Path) -> std::io::Result<()> {
        for (name, kind) in [
            ("desert-rocks", PropKind::Rock),
            ("desert-bushes", PropKind::Bush),
            ("desert-cacti", PropKind::Cactus),
            ("wetland-plants", PropKind::WetlandPlant),
        ] {
            let mut atlas = vec![0; ATLAS_SIDE * ATLAS_SIDE * 3];
            let mut masks = BTreeSet::new();
            let mut colors = BTreeSet::new();
            let mut smallest = usize::MAX;
            let mut largest = 0;
            for id in 0..=u8::MAX {
                let pixels = render_variant(kind, id);
                let offset_x = usize::from(id) % GRID * SIDE;
                let offset_y = usize::from(id) / GRID * SIDE;
                let mut mask = [0_u64; (SIDE * SIDE).div_ceil(64)];
                let mut occupied = 0;
                for (index, pixel) in pixels.into_iter().enumerate() {
                    let x = index % SIDE;
                    let y = index / SIDE;
                    if pixel != 0 {
                        assert!((4..SIDE - 4).contains(&x) && (4..SIDE - 4).contains(&y), "{name} {id} outside its tile");
                        occupied += 1;
                        mask[index / 64] |= 1 << (index % 64);
                    }
                    let atlas_index = ((offset_y + y) * ATLAS_SIDE + offset_x + x) * 3;
                    atlas[atlas_index..atlas_index + 3].copy_from_slice(&rgb(pixel));
                }
                assert_ne!(occupied, 0, "{name} {id} is empty");
                smallest = smallest.min(occupied);
                largest = largest.max(occupied);
                masks.insert(mask);
                colors.insert(pixels);
            }
            assert_eq!(colors.len(), 256, "duplicate game-rendered tiles for {name}");
            assert_eq!(masks.len(), 256, "duplicate silhouette masks for {name}");
            let mut image = BufWriter::new(File::create(out.join(format!("{name}.ppm")))?);
            write!(image, "P6\n{ATLAS_SIDE} {ATLAS_SIDE}\n255\n")?;
            image.write_all(&atlas)?;
            image.flush()?;
            println!("{name}: {} distinct RGB565 sprites, {} silhouette masks, {smallest}..{largest} art pixels per sprite", colors.len(), masks.len());
        }
        Ok(())
    }
}

fn main() -> std::io::Result<()> {
    let output = std::env::args_os().nth(1).expect("Provide the scratch output directory");
    artwork::write(std::path::Path::new(&output))
}
