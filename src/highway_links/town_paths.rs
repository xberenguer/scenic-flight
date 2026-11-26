//! Curvature profiles for dry roads between connected town street exits.
//!
//! Fitting still checks the grade, terrain and reservations along the entire
//! shoulder. Profiles alter the centerline while keeping both exact tangents.

pub(super) struct TownPath {
    pub(super) name: &'static str,
    pub(super) curve: [f32; 3],
}

const fn path(name: &'static str, offset: f32, sweep: f32, counter_bend: f32) -> TownPath {
    TownPath { name, curve: [offset, sweep, counter_bend] }
}

pub(super) const TOWN_PATHS: [TownPath; 26] = [
    path("Direct avenue", 0.0, 0.0, 0.0),
    path("Shallow left arc", 22.0, -11.0, 0.0),
    path("Shallow right arc", -22.0, 11.0, 0.0),
    path("Late left bend", 34.0, 74.0, 0.0),
    path("Late right bend", -34.0, -74.0, 0.0),
    path("Early left bend", 34.0, -74.0, 0.0),
    path("Early right bend", -34.0, 74.0, 0.0),
    path("Left bank sweep", 64.0, 20.0, -45.0),
    path("Right bank sweep", -64.0, -20.0, 45.0),
    path("Left and right offset", 0.0, 145.0, 28.0),
    path("Right and left offset", 0.0, -145.0, -28.0),
    path("Left hillside shoulder", 62.0, -30.0, 95.0),
    path("Right hillside shoulder", -62.0, 30.0, -95.0),
    path("Left shallow S-curve", 11.0, 92.0, 78.0),
    path("Right shallow S-curve", -11.0, -92.0, -78.0),
    path("Left broad S-curve", 45.0, 180.0, -155.0),
    path("Right broad S-curve", -45.0, -180.0, 155.0),
    path("Left saddle approach", 86.0, -128.0, 110.0),
    path("Right saddle approach", -86.0, 128.0, -110.0),
    path("Left valley loop", 113.0, 52.0, -123.0),
    path("Right valley loop", -113.0, -52.0, 123.0),
    path("Left gully bypass", 136.0, -147.0, -180.0),
    path("Right gully bypass", -136.0, 147.0, 180.0),
    path("Left cliff bypass", 150.0, 147.0, 145.0),
    path("Right cliff bypass", -150.0, -147.0, -145.0),
    path("Staggered ridge crossing", 4.0, 51.0, 210.0),
];
