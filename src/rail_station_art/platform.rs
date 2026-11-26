//! Platform fixtures in normalized, survey-approved mounting envelopes.

use super::StationLayout;
use crate::train_art::FreightRoute;

type Point = [f32; 3];
type Color = [u8; 3];

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub(super) enum Fixture {
    StationClock,
    TimetableCase,
    PlatformMarker,
    LampPair,
    DispatchHut,
    LuggageShelter,
    ParcelTrolley,
    TicketKiosk,
    WaitingScreen,
    NewsStand,
    Semaphore,
    SignalCabinet,
    ToolLocker,
    SandBox,
    TrackGaugeRack,
    JibCrane,
    LoadingHoist,
    WeighDesk,
    SackHopper,
    HoseManifold,
    TimberBunks,
    CoilCradles,
    MilkChurns,
    DrumPallet,
    StackedCrates,
}

use Fixture::*;
const PLATFORM: [Fixture; 5] = [
    StationClock,
    TimetableCase,
    PlatformMarker,
    LampPair,
    DispatchHut,
];
const PASSENGER: [Fixture; 5] = [
    LuggageShelter,
    ParcelTrolley,
    TicketKiosk,
    WaitingScreen,
    NewsStand,
];
const MAINTENANCE: [Fixture; 5] = [
    Semaphore,
    SignalCabinet,
    ToolLocker,
    SandBox,
    TrackGaugeRack,
];
const LOADING: [Fixture; 5] = [JibCrane, LoadingHoist, WeighDesk, SackHopper, HoseManifold];
const STORAGE: [Fixture; 5] = [
    TimberBunks,
    CoilCradles,
    MilkChurns,
    DrumPallet,
    StackedCrates,
];

impl Fixture {
    pub(super) fn select(
        layout: StationLayout,
        slot: usize,
        cargo: FreightRoute,
        key: u64,
    ) -> Option<Self> {
        // Established terminal equipment still appears. Bulk terminals retain
        // their dedicated feeder and stockpiles.
        if key & 7 == 0
            || matches!(
                (layout, cargo),
                (
                    StationLayout::FreightApron { .. },
                    FreightRoute::Quarry | FreightRoute::Aggregate
                )
            )
        {
            return None;
        }
        let group = match (layout, slot, cargo) {
            (StationLayout::RuralHalt, 0, _) => &MAINTENANCE[..],
            (StationLayout::RuralHalt, _, _) => &PLATFORM,
            (StationLayout::TownStop | StationLayout::RegionalStation, 0, _) => &PLATFORM,
            (StationLayout::TownStop | StationLayout::RegionalStation, _, _) => &PASSENGER,
            (_, 0, FreightRoute::Liquid) => &[HoseManifold, WeighDesk],
            (_, 0, FreightRoute::Agriculture) => &[SackHopper, WeighDesk, LoadingHoist],
            (_, 0, FreightRoute::Intermodal | FreightRoute::Forest | FreightRoute::Mill) => {
                &[JibCrane, LoadingHoist, WeighDesk]
            }
            (_, 0, FreightRoute::Perishable | FreightRoute::Automotive) => {
                &[LoadingHoist, WeighDesk]
            }
            (_, 0, _) => &LOADING,
            (_, _, FreightRoute::Forest) => &[TimberBunks],
            (_, _, FreightRoute::Mill) => &[CoilCradles, DrumPallet],
            (_, _, FreightRoute::Liquid) => &[DrumPallet],
            (_, _, FreightRoute::Agriculture | FreightRoute::Perishable) => {
                &[MilkChurns, StackedCrates]
            }
            (_, _, FreightRoute::Intermodal | FreightRoute::Automotive) => &[StackedCrates],
            _ => &STORAGE,
        };
        Some(group[(key >> 17) as usize % group.len()])
    }

    pub(super) fn paint(self, emit: impl FnMut([Point; 3], Color)) {
        const STEEL: Color = [110, 127, 134];
        const IRON: Color = [81, 95, 99];
        const WOOD: Color = [145, 113, 78];
        const ENAMEL: Color = [154, 81, 54];
        const GLASS: Color = [137, 170, 177];
        const WHITE: Color = [209, 208, 182];
        const BRASS: Color = [190, 143, 65];
        let mut mesh = Mesh(emit);
        match self {
            StationClock => {
                mesh.post(0.0, 2.06, STEEL);
                mesh.disk([0.0, 2.14, 0.0], [0.48, 0.58], [0.23, 0.29], IRON);
                mesh.disk([0.0, 2.14, 0.28], [0.38, 0.46], [0.02, 0.02], WHITE);
                mesh.block([0.0, 2.07, 0.31], [0.075, 0.35, 0.02], IRON);
                mesh.block([0.16, 2.13, 0.32], [0.21, 0.07, 0.02], IRON);
            }
            TimetableCase => {
                for x in [-0.58, 0.58] {
                    mesh.post(x, 1.86, ENAMEL);
                }
                mesh.block([0.0, 0.79, 0.0], [0.68, 1.13, 0.26], STEEL);
                mesh.block([0.0, 0.95, 0.27], [0.54, 0.81, 0.04], WHITE);
                for y in [1.10, 1.29, 1.48] {
                    mesh.block([0.0, y, 0.32], [0.41, 0.06, 0.02], IRON);
                }
                mesh.gable([0.0, 1.87, 0.0], [0.84, 0.31, 0.50], ENAMEL);
            }
            PlatformMarker => {
                mesh.post(-0.41, 1.87, IRON);
                mesh.post(0.41, 1.20, IRON);
                mesh.block([-0.17, 1.26, 0.0], [0.64, 0.59, 0.25], [62, 110, 138]);
                mesh.block([0.08, 1.02, 0.0], [0.36, 0.28, 0.18], WHITE);
                mesh.block([-0.18, 1.51, 0.26], [0.34, 0.10, 0.025], WHITE);
            }
            LampPair => {
                for x in [-0.67, 0.67] {
                    mesh.post(x, if x < 0.0 { 2.08 } else { 1.65 }, STEEL);
                    let top = if x < 0.0 { 2.04 } else { 1.61 };
                    mesh.block([x * 0.80, top, 0.20], [0.28, 0.16, 0.39], IRON);
                    mesh.block(
                        [x * 0.80, top - 0.12, 0.24],
                        [0.19, 0.12, 0.32],
                        [204, 200, 147],
                    );
                }
            }
            DispatchHut => {
                mesh.block([-0.38, -0.26, 0.0], [0.43, 1.74, 0.57], WOOD);
                mesh.block([-0.34, 0.74, 0.59], [0.28, 0.53, 0.045], GLASS);
                mesh.gable([-0.38, 1.40, 0.0], [0.55, 0.48, 0.71], ENAMEL);
                mesh.block([0.48, -0.26, 0.05], [0.20, 0.68, 0.32], IRON);
                mesh.post(0.68, 1.06, WHITE);
            }
            LuggageShelter => {
                for x in [-0.79, 0.79] {
                    mesh.post(x, 1.33, WOOD);
                }
                mesh.gable([0.0, 1.29, 0.0], [0.94, 0.47, 0.77], STEEL);
                mesh.block([-0.46, -0.24, 0.10], [0.32, 0.49, 0.43], ENAMEL);
                mesh.block([0.19, -0.24, -0.14], [0.30, 0.60, 0.42], WOOD);
            }
            ParcelTrolley => {
                mesh.block([0.0, -0.19, 0.0], [0.88, 0.27, 0.57], IRON);
                for x in [-0.61, 0.64] {
                    mesh.block([x, -0.27, 0.0], [0.19, 0.20, 0.65], IRON);
                }
                mesh.block([-0.21, 0.08, 0.0], [0.58, 0.53, 0.54], WOOD);
                mesh.block([-0.33, 0.61, -0.10], [0.37, 0.29, 0.40], BRASS);
                mesh.block([0.83, 0.06, 0.0], [0.08, 0.72, 0.39], STEEL);
            }
            TicketKiosk => {
                mesh.block([0.0, -0.26, 0.0], [0.47, 1.71, 0.58], [63, 125, 121]);
                mesh.block([0.0, 0.75, 0.60], [0.34, 0.39, 0.065], GLASS);
                mesh.block([0.0, 0.64, 0.66], [0.48, 0.10, 0.20], WHITE);
                mesh.block([0.0, 1.39, 0.0], [0.59, 0.20, 0.71], IRON);
            }
            WaitingScreen => {
                for x in [-0.82, 0.0, 0.82] {
                    mesh.post(x, 1.32, IRON);
                }
                mesh.block([-0.40, 0.36, 0.0], [0.35, 0.87, 0.13], GLASS);
                mesh.block([0.40, 0.53, 0.0], [0.35, 0.69, 0.13], GLASS);
                mesh.block([0.0, 0.35, 0.36], [0.89, 0.16, 0.34], WOOD);
            }
            NewsStand => {
                mesh.block([0.0, -0.26, -0.11], [0.79, 1.08, 0.49], ENAMEL);
                mesh.block([0.0, 0.49, 0.44], [0.74, 0.52, 0.19], IRON);
                for x in [-0.45, 0.0, 0.45] {
                    mesh.block([x, 0.60, 0.64], [0.19, 0.28, 0.08], WHITE);
                }
                mesh.gable([0.0, 0.97, 0.0], [0.91, 0.31, 0.70], ENAMEL);
            }
            Semaphore => {
                mesh.post(-0.52, 2.44, IRON);
                mesh.block([-0.09, 2.13, 0.03], [0.60, 0.18, 0.22], ENAMEL);
                mesh.block([0.22, 2.14, 0.28], [0.13, 0.13, 0.035], WHITE);
                mesh.block([-0.56, 0.13, 0.0], [0.25, 0.61, 0.39], STEEL);
            }
            SignalCabinet => {
                mesh.block([-0.49, -0.26, 0.0], [0.35, 1.37, 0.42], STEEL);
                mesh.block([0.36, -0.26, 0.02], [0.43, 0.94, 0.48], IRON);
                mesh.block([-0.49, 0.44, 0.44], [0.21, 0.23, 0.025], WHITE);
                mesh.post(0.79, 1.39, IRON);
                mesh.block([0.79, 1.38, 0.0], [0.19, 0.23, 0.20], ENAMEL);
            }
            ToolLocker => {
                mesh.block([0.05, -0.26, 0.0], [0.52, 1.41, 0.42], ENAMEL);
                mesh.block([0.66, -0.26, 0.13], [0.20, 0.49, 0.42], WOOD);
                mesh.gable([0.04, 1.09, 0.0], [0.63, 0.26, 0.52], STEEL);
                mesh.block([-0.28, 0.51, 0.43], [0.11, 0.42, 0.035], BRASS);
            }
            SandBox => {
                mesh.block([-0.18, -0.26, 0.10], [0.69, 0.63, 0.51], BRASS);
                mesh.gable([-0.18, 0.36, 0.10], [0.75, 0.35, 0.58], WOOD);
                mesh.post(0.65, 1.03, STEEL);
                mesh.block([0.65, 0.95, 0.20], [0.19, 0.20, 0.39], WHITE);
            }
            TrackGaugeRack => {
                for x in [-0.75, 0.75] {
                    mesh.post(x, 1.12, IRON);
                }
                for y in [0.35, 0.67, 1.02] {
                    mesh.block([0.0, y, 0.04], [0.89, 0.09, 0.14], STEEL);
                }
                mesh.block([-0.36, 0.12, -0.31], [0.22, 0.34, 0.37], ENAMEL);
                mesh.block([0.41, 0.15, 0.28], [0.22, 0.33, 0.37], WOOD);
            }
            JibCrane => {
                mesh.block([-0.58, -0.26, 0.0], [0.37, 0.53, 0.66], IRON);
                mesh.block([-0.58, 0.22, 0.0], [0.16, 1.80, 0.27], BRASS);
                mesh.block([0.08, 1.96, 0.0], [0.81, 0.26, 0.30], BRASS);
                mesh.block([0.63, 1.03, 0.0], [0.071, 0.96, 0.12], IRON);
                mesh.block([0.64, 1.01, 0.04], [0.22, 0.18, 0.29], IRON);
            }
            LoadingHoist => {
                for x in [-0.79, 0.79] {
                    mesh.block([x, -0.27, 0.0], [0.15, 1.83, 0.42], ENAMEL);
                }
                mesh.block([0.0, 1.47, 0.0], [0.96, 0.31, 0.46], ENAMEL);
                mesh.block([0.0, 0.48, 0.0], [0.11, 0.95, 0.21], IRON);
                mesh.block([0.0, 0.39, 0.08], [0.27, 0.18, 0.30], BRASS);
            }
            WeighDesk => {
                mesh.block([-0.39, -0.25, 0.0], [0.52, 0.27, 0.63], STEEL);
                mesh.block([0.53, -0.25, -0.16], [0.29, 1.17, 0.54], WOOD);
                mesh.block([0.53, 0.52, 0.41], [0.21, 0.26, 0.04], GLASS);
                mesh.gable([0.52, 0.87, -0.14], [0.39, 0.44, 0.73], ENAMEL);
            }
            SackHopper => {
                for x in [-0.54, 0.54] {
                    mesh.post(x, 1.19, IRON);
                }
                mesh.frustum(
                    [0.0, 0.25, 0.0],
                    [0.21, 0.26],
                    [0.0, 1.43, 0.0],
                    [0.75, 0.62],
                    [156, 149, 105],
                );
                mesh.block([-0.59, -0.24, 0.19], [0.27, 0.37, 0.45], BRASS);
            }
            HoseManifold => {
                for x in [-0.57, 0.57] {
                    mesh.post(x, 1.43, STEEL);
                }
                mesh.block([0.0, 1.13, 0.0], [0.74, 0.16, 0.18], STEEL);
                mesh.disk([-0.36, 0.61, 0.0], [0.38, 0.45], [0.47, 0.47], IRON);
                mesh.disk([-0.36, 0.61, 0.49], [0.26, 0.33], [0.12, 0.12], ENAMEL);
                mesh.block([0.63, 0.35, 0.19], [0.19, 0.43, 0.29], BRASS);
            }
            TimberBunks => {
                mesh.block([0.0, -0.26, 0.0], [0.91, 0.29, 0.72], WOOD);
                for [y, z, w, d] in [
                    [0.04, -0.36, 0.90, 0.22],
                    [0.04, 0.0, 0.86, 0.21],
                    [0.04, 0.38, 0.83, 0.20],
                    [0.30, -0.12, 0.77, 0.21],
                    [0.30, 0.25, 0.82, 0.20],
                    [0.55, 0.11, 0.70, 0.20],
                ] {
                    mesh.frustum([0.0, y, z], [w, d], [0.0, y + 0.24, z], [w, d * 0.50], WOOD);
                }
            }
            CoilCradles => {
                mesh.block([0.0, -0.26, 0.0], [0.86, 0.29, 0.74], WOOD);
                for [x, y] in [[-0.47, 0.47], [0.50, 0.44]] {
                    mesh.disk([x, y, 0.0], [0.34, 0.44], [0.58, 0.58], STEEL);
                    mesh.disk([x, y, 0.59], [0.18, 0.21], [0.04, 0.04], IRON);
                }
            }
            MilkChurns => {
                for [x, y, z] in [[-0.61, 0.0, -0.11], [0.0, 0.15, 0.18], [0.56, 0.07, -0.09]] {
                    mesh.frustum(
                        [x, -0.25, z],
                        [0.24, 0.31],
                        [x, y + 0.64, z],
                        [0.20, 0.29],
                        STEEL,
                    );
                    mesh.frustum(
                        [x, y + 0.64, z],
                        [0.20, 0.29],
                        [x, y + 0.80, z],
                        [0.115, 0.19],
                        WHITE,
                    );
                    mesh.block([x, y + 0.80, z], [0.14, 0.05, 0.23], WHITE);
                }
            }
            DrumPallet => {
                mesh.block([0.0, -0.26, 0.0], [0.85, 0.29, 0.69], WOOD);
                for [x, z, h] in [
                    [-0.52, -0.17, 0.82],
                    [0.15, 0.14, 0.70],
                    [0.61, -0.13, 0.80],
                ] {
                    mesh.frustum(
                        [x, 0.02, z],
                        [0.24, 0.36],
                        [x, h, z],
                        [0.22, 0.34],
                        [74, 112, 126],
                    );
                    mesh.block([x, 0.33, z], [0.25, 0.07, 0.38], BRASS);
                }
            }
            StackedCrates => {
                mesh.block([-0.45, -0.26, 0.0], [0.42, 0.48, 0.58], WOOD);
                mesh.block([0.48, -0.26, 0.0], [0.42, 0.59, 0.57], ENAMEL);
                mesh.block([-0.45, 0.23, -0.12], [0.37, 0.43, 0.54], WHITE);
                mesh.block([-0.50, 0.67, -0.15], [0.32, 0.36, 0.45], WOOD);
                mesh.block([0.51, 0.33, -0.05], [0.37, 0.45, 0.47], BRASS);
            }
        }
    }
}

struct Mesh<F>(F);
impl<F: FnMut([Point; 3], Color)> Mesh<F> {
    fn post(&mut self, x: f32, top: f32, color: Color) {
        self.block([x, -0.26, 0.0], [0.084, top + 0.26, 0.13], color);
    }

    fn quad(&mut self, [a, b, c, d]: [Point; 4], color: Color) {
        (self.0)([a, b, c], color);
        (self.0)([a, c, d], color);
    }

    fn block(&mut self, [x, y, z]: Point, [w, h, d]: Point, color: Color) {
        self.frustum([x, y, z], [w, d], [x, y + h, z], [w, d], color);
    }

    fn frustum(
        &mut self,
        [x, y, z]: Point,
        [w, d]: [f32; 2],
        [a, b, c]: Point,
        [v, e]: [f32; 2],
        color: Color,
    ) {
        let lower = [
            [x - w, y, z - d],
            [x + w, y, z - d],
            [x + w, y, z + d],
            [x - w, y, z + d],
        ];
        let upper = [
            [a - v, b, c - e],
            [a + v, b, c - e],
            [a + v, b, c + e],
            [a - v, b, c + e],
        ];
        for i in 0..4 {
            let j = (i + 1) & 3;
            self.quad(
                [lower[i], lower[j], upper[j], upper[i]],
                color.map(|v| v.saturating_sub([0, 15, 22, 9][i])),
            );
        }
        self.quad(upper, color.map(|v| v.saturating_add(9)));
    }

    fn gable(&mut self, [x, y, z]: Point, [w, h, d]: Point, color: Color) {
        let [a, b, c, d] = [
            [x - w, y, z - d],
            [x + w, y, z - d],
            [x + w, y, z + d],
            [x - w, y, z + d],
        ];
        let near = [x, y + h, a[2]];
        let far = [x, y + h, c[2]];
        self.quad([a, d, far, near], color.map(|v| v.saturating_add(12)));
        self.quad([b, c, far, near], color);
        (self.0)([a, b, near], color);
        (self.0)([c, d, far], color);
    }

    fn disk(&mut self, [x, y, z]: Point, [rx, ry]: [f32; 2], [near, far]: [f32; 2], color: Color) {
        let ring = [
            [1.0, 0.0],
            [0.707, 0.707],
            [0.0, 1.0],
            [-0.707, 0.707],
            [-1.0, 0.0],
            [-0.707, -0.707],
            [0.0, -1.0],
            [0.707, -0.707],
        ];
        let low = ring.map(|[u, v]| [x + rx * u, y + ry * v, z - near]);
        let high = ring.map(|[u, v]| [x + rx * u, y + ry * v, z + far]);
        for i in 0..8 {
            let j = (i + 1) & 7;
            self.quad(
                [low[i], low[j], high[j], high[i]],
                color.map(|v| v.saturating_sub(19)),
            );
            (self.0)([[x, y, z + far], high[i], high[j]], color);
            (self.0)([[x, y, z - near], low[i], low[j]], color);
        }
    }
}
