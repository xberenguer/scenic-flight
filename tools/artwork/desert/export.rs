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
        // The fixed view and scale retain the proportions of the models within each sheet.
        let camera = if matches!(kind, PropKind::Habitat(Feature::Farm(_) | Feature::Mountain(_) | Feature::Freshwater(_) | Feature::Woodland(_) | Feature::Dryland(_) | Feature::Grassland(_))) {
            Camera { x: 3.7, z: -5.13, altitude: 3.80, heading: -0.624, pitch: 0.36, bank: 0.0 }
        } else {
            Camera { x: 3.1, z: -4.3, altitude: 3.35, heading: -0.624, pitch: 0.36, bank: 0.0 }
        };
        let daylight = crate::sky::Sky::from_time(41, 0, 0.5);
        let prop = Prop { origin: [0.0; 3], grade: [0.0; 2], size: 1.0, kind, variant, opacity: 1.0 };
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

    pub fn shore(out: &Path) -> std::io::Result<()> {
        use habitats::shore::ShoreFeature::*;
        let forms = [
            ShellBank, PumiceRaft, KelpStipes, TangledRoots, OysterRidge,
            IcePlant, DuneSedge, SandVerbena, BeachPea, SaltPrunedPine,
            PebbleTerrace, BoulderTripod, BasaltFingers, SplitSlab, ErodedSandstone,
            HauledSkiff, LobsterPots, NetRack, FishCrates, OarTrestle,
            RescueStand, DuneGate, ShadeSail, BeachSteps, TrailKiosk,
        ];
        model_atlas(out, "shore", forms.map(Feature::Shore))
    }

    pub fn farm(out: &Path) -> std::io::Result<()> {
        use habitats::farm::FarmFeature::*;
        let forms = [
            RoundBales, StrawStack, CoveredWagon, GrainAuger, Combine,
            GambrelBarn, PoleShed, MilkParlor, Glasshouse, RootCellar,
            TankStand, PumpCabin, HoseReel, StoneTrough, SluiceHouse,
            OrchardBins, VineTrellis, PickingLadder, PickingPlatform, ColdStore,
            ProduceStall, Weighbridge, FeedDepot, Windpump, FieldGate,
        ];
        model_atlas(out, "farm", forms.map(Feature::Farm))
    }

    pub fn mountain(out: &Path) -> std::io::Result<()> {
        use habitats::mountain::MountainFeature::*;
        let forms = [
            CreepingJuniper, StuntedSpruce, MountainMahogany, SplitPineSnag, RootPlate,
            HeatherBank, AlpineAvens, DwarfWillow, BunchgrassRing, GentianSpikes,
            QuartzSeam, SlateBlades, RockStep, SplitGranite, BasaltColumns,
            SnowCornice, RimIceFin, SnowPocket, WindCrust, RimedBoulder,
            BoulderTrain, GlacialPavement, PairedErratics, MoraineCrest, MeltwaterCobbles,
        ];
        model_atlas(out, "mountain", forms.map(Feature::Mountain))
    }

    pub fn freshwater(out: &Path) -> std::io::Result<()> {
        use habitats::freshwater::FreshwaterFeature::*;
        let forms = [
            CobbleRidge, SandRipple, ShaleShingle, FloodWrack, RootWad,
            RiverLog, ForkedDriftwood, SiltButte, PebbleFan, ChannelBoulders,
            CattailStand, BulrushUmbels, SweetFlag, BurReed, ReedWall,
            MarshMarigold, IrisStand, Meadowsweet, Loosestrife, WaterMint,
            WillowStool, AlderSaplings, RedOsierThicket, BrackenBreak, RushTussocks,
        ];
        model_atlas(out, "freshwater", forms.map(Feature::Freshwater))
    }

    pub fn woodland(out: &Path) -> std::io::Result<()> {
        use habitats::woodland::WoodlandFeature::*;
        let forms = [
            SpruceSeedling, HemlockWhorls, YewBush, HazelCoppice, BirchSaplings,
            OstrichFern, WoodSedge, BroadleafHellebore, FoamflowerMat, EvergreenHolly,
            NurseLog, SplitStump, WindthrowPit, BarkHeap, BranchTangle,
            ShelfFungi, ParasolPatch, MorelCluster, PuffballRing, CoralFungi,
            BrackenFronds, Foxglove, RoseHipBramble, BilberryMound, HoneysuckleArch,
        ];
        model_atlas(out, "woodland", forms.map(Feature::Woodland))
    }

    pub fn dryland(out: &Path) -> std::io::Result<()> {
        use habitats::dryland::DrylandFeature::*;
        let forms = [
            PiledWashCobbles, DryRootFan, GravelRibs, CutbankShelf, DriftBranch,
            DuneSlipFace, RippleField, SandGrassLine, DuneSkirt, SandVerbenaMat,
            SandstonePedestal, TwinSandstoneFins, BandedSandstone, BasaltTalus, DesertPavement,
            CreosoteRing, SaltbushHummocks, Brittlebush, Blackbrush, MojaveAster,
            SpanishBayonet, AgaveStalk, ChollaSkeleton, BeavertailClump, BarrelCactusGroup,
        ];
        model_atlas(out, "dryland", forms.map(Feature::Dryland))
    }

    pub fn grassland(out: &Path) -> std::io::Result<()> {
        use habitats::grassland::GrasslandFeature::*;
        let forms = [
            OxeyeDaisy, Knapweed, BirdsfootTrefoil, Chicory, YarrowUmbels,
            BlueStemFan, FescueTufts, OatGrass, FoxtailPatch, FeatherGrass,
            HawthornScrub, GorseCushion, BroomStand, DogRose, JuniperMound,
            LimestonePavement, ChalkHummocks, GlacialBoulder, ScatteredFlint, LowStoneBank,
            WindPrunedThorn, WeatheredLog, BrackenIsland, ThornRootBall, MeadowAnthill,
        ];
        model_atlas(out, "grassland", forms.map(Feature::Grassland))
    }

    pub fn headland(out: &Path) -> std::io::Result<()> {
        use habitats::headland::HeadlandFeature::*;
        let forms = [
            SandHopperBurrows, RazorShellFan, KelpRibbon, CorkWrack, BarnacleCobbles,
            BeachMorningGlory, SeasideDaisy, HonckenyaCushion, DuneSpurge, BeachEveningPrimrose,
            GraniteShingle, FlintSpit, SandstoneLedge, IronstoneRubble, ChalkBreccia,
            DuneWillow, BayberryThicket, SeaBuckthorn, CoastalHeather, SaltBurnedJuniper,
            SandFescue, LymeGrass, SeaRush, DuneReed, SaltMeadowHay,
        ];
        model_atlas(out, "headland", forms.map(Feature::Headland))
    }

    pub fn alpine_ground(out: &Path) -> std::io::Result<()> {
        use habitats::alpine_ground::AlpineGround::*;
        let forms = [
            WindSculptedDrift, SnowBridgeLip, SnowPillows, FrostedTalus, SnowbreakSlabs,
            BlueIceRibs, PressureHummocks, MeltwaterRunnels, RefrozenLens, LayeredIceStep,
            GarnetScree, ShaleBook, QuartziteNeedles, FrostSplitTor, VeinedSchist,
            MixedErratics, SortedStoneRing, IceMarginalBank, GlacialBoulderFan, MoraineCobbleRibs,
            WhiteSaxifrage, AlpinePoppy, WillowMat, LichenCushions, SnowbedSedge,
        ];
        model_atlas(out, "alpine-ground", forms.map(Feature::AlpineGround))
    }

    pub fn wetland(out: &Path) -> std::io::Result<()> {
        use habitats::wetland::WetlandFeature::*;
        let forms = [
            PeatHummocks, SphagnumLobes, FenMarlLedge, TussockIslands, BankOverhang,
            RaggedRobin, FenOrchid, GrassOfParnassus, YellowRattle, Globeflower,
            ReedPanicles, SpikeRushLawn, FloweringRush, ArrowheadStand, ReedCaneLitter,
            ClayScrolls, GravelTongue, SnagWithSiltApron, IronSeepCrust, SandbarRipple,
            BogMyrtle, LeatherleafMat, DwarfWillow, BuckthornFenClump, CranberryRunner,
        ];
        model_atlas(out, "wetland", forms.map(Feature::Wetland))
    }

    fn model_atlas(out: &Path, name: &str, forms: [Feature; 25]) -> std::io::Result<()> {
        const COLUMNS: usize = 5;
        let (tile, left, top) = match name {
            "farm" => (64, 16, 20),
            "mountain" | "freshwater" => (54, 21, 23),
            "woodland" | "dryland" | "grassland" => (57, 20, 20),
            "headland" | "alpine-ground" | "wetland" => (72, 12, 12),
            _ => (SIDE, 0, 0),
        };
        let width = COLUMNS * tile;
        for angle in 0..16 {
            let mut atlas = vec![0; width * width * 3];
            for (index, form) in forms.into_iter().enumerate() {
                let pixels = render_variant(PropKind::Habitat(form), angle);
                let offset_x = index % COLUMNS * tile;
                let offset_y = index / COLUMNS * tile;
                assert!(pixels.iter().any(|&pixel| pixel != 0), "{form:?} is empty");
                for (index, pixel) in pixels.into_iter().enumerate() {
                    let (x, y) = (index % SIDE, index / SIDE);
                    let inside = (left..left + tile).contains(&x) && (top..top + tile).contains(&y);
                    if pixel != 0 {
                        assert!((left + 3..left + tile - 3).contains(&x)
                            && (top + 3..top + tile - 3).contains(&y),
                            "{form:?} outside its tile at angle {angle}");
                    }
                    if inside {
                        let destination = ((offset_y + y - top) * width + offset_x + x - left) * 3;
                        atlas[destination..destination + 3].copy_from_slice(&rgb(pixel));
                    }
                }
            }
            let mut image = BufWriter::new(File::create(out.join(format!("{name}-{angle:02}.ppm")))?);
            write!(image, "P6\n{width} {width}\n255\n")?;
            image.write_all(&atlas)?;
            image.flush()?;
        }
        Ok(())
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
    let mut arguments = std::env::args_os().skip(1);
    let output = arguments.next().expect("Provide the scratch output directory");
    let output = std::path::Path::new(&output);
    let mode = arguments.next();
    match mode.as_deref().and_then(std::ffi::OsStr::to_str) {
        Some("--shore-only") => artwork::shore(output),
        Some("--farm-only") => artwork::farm(output),
        Some("--mountain-only") => artwork::mountain(output),
        Some("--freshwater-only") => artwork::freshwater(output),
        Some("--woodland-only") => artwork::woodland(output),
        Some("--dryland-only") => artwork::dryland(output),
        Some("--grassland-only") => artwork::grassland(output),
        Some("--headland-only") => artwork::headland(output),
        Some("--alpine-ground-only") => artwork::alpine_ground(output),
        Some("--wetland-only") => artwork::wetland(output),
        None => {
            artwork::write(output)?;
            artwork::shore(output)?;
            artwork::farm(output)?;
            artwork::mountain(output)?;
            artwork::freshwater(output)?;
            artwork::woodland(output)?;
            artwork::dryland(output)?;
            artwork::grassland(output)?;
            artwork::headland(output)?;
            artwork::alpine_ground(output)?;
            artwork::wetland(output)
        }
        _ => Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Unknown artwork mode")),
    }
}
