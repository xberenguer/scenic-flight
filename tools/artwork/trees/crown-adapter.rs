
// Expose only the private geometry needed by this standalone artwork renderer.
pub(super) struct ArtworkCrown(TreeCrown);

impl ArtworkCrown {
    pub(super) fn new(variant: u8) -> Self {
        Self(TreeCrown::with_variant((0, 0), [0.0; 2], variant))
    }

    pub(super) fn height(&self) -> f32 {
        self.0.height
    }

    pub(super) fn measurements(&self) -> (&'static str, f32, f32, f32) {
        let form = match self.0.form {
            CrownForm::Rounded => "Rounded",
            CrownForm::Pointed => "Pointed",
            CrownForm::Leaning => "Leaning",
            CrownForm::Forked => "Forked",
            CrownForm::Spire => "Spire",
            CrownForm::Umbrella => "Umbrella",
            CrownForm::Vase => "Vase",
            CrownForm::Fan => "Fan",
            CrownForm::Cluster => "Cluster",
            CrownForm::Crescent => "Crescent",
            CrownForm::Tiered => "Tiered",
            CrownForm::WindSwept => "Wind swept",
            CrownForm::ThreeLobed => "Three lobed",
            CrownForm::CutCrown => "Cut crown",
            CrownForm::SplitRidge => "Split ridge",
            CrownForm::Cushion => "Cushion",
            CrownForm::Bare => unreachable!(),
        };
        (
            form,
            2.0 / self.0.inverse_span[0],
            2.0 / self.0.inverse_span[1],
            self.0.height,
        )
    }

    pub(super) fn palette(&self) -> [u8; 3] {
        self.0.form.foliage().0.canopy_color().unwrap()
    }

    pub(super) fn sample(&self, x: f32, z: f32) -> (f32, bool) {
        const FLAT_FOREST: Sample = Sample { elevation: 0.0, surface: Surface::Forest };
        let sample = self.0.overlay(x, z, FLAT_FOREST);
        (sample.elevation, sample.surface.is_tree_canopy())
    }
}
