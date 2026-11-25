use crate::math::sin_cos;
use crate::world::World;

const COORDINATE_LIMIT: f32 = 25_100.0;
const DETAIL_STEP: u32 = 12;

#[derive(Clone, Copy)]
pub(crate) enum ChannelAircraft {
    Fighter,
    Racer,
}

#[derive(Clone, Copy)]
pub(crate) struct CoastChannel {
    aircraft: ChannelAircraft,
    pub(crate) origin: [f32; 2],
    pub(crate) forward: [f32; 2],
    pub(crate) wet_span: [f32; 2],
    pub(crate) inland_handoff: f32,
    pub(crate) inland_target: [f32; 2],
}

struct SurveyRules {
    headings: &'static [f32],
    coast_threshold: f32,
    coarse_step: u32,
    coarse_limit: u32,
    max_coarse_gap: u32,
    max_landfall: u32,
    dry_proof: u32,
    detailed_runout: u32,
    half_width: f32,
    max_wet_span: u32,
    handoff_distance: u32,
    runout: RunoutRules,
}

struct RunoutRules {
    turns: &'static [f32],
    width_checks: [f32; 3],
    cross_checks: [f32; 5],
    step: u32,
    limit: u32,
    half_width: f32,
    target_distance: f32,
    strict_edge: bool,
}

pub(crate) struct ChannelGuidance {
    pub(crate) follow_distance: f32,
    pub(crate) max_course_change: f32,
    pub(crate) near_water: [f32; 2],
    pub(crate) near_water_lateral_limit: f32,
    pub(crate) dry_lateral_limit: f32,
    pub(crate) wet_margin: f32,
    pub(crate) max_water_seconds: u8,
}

impl ChannelAircraft {
    pub(crate) const fn max_leg_seconds(self) -> u64 {
        match self {
            Self::Fighter => 65,
            Self::Racer => 98,
        }
    }

    fn survey_rules(self) -> &'static SurveyRules {
        match self {
            Self::Fighter => &SurveyRules {
                headings: &[
                    0.0, -0.10, 0.10, -0.20, 0.20, -0.30, 0.30, -0.42, 0.42, -0.55, 0.55,
                ],
                coast_threshold: 0.003,
                coarse_step: 40,
                coarse_limit: 1320,
                max_coarse_gap: 128,
                max_landfall: 660,
                dry_proof: 600,
                detailed_runout: 576,
                half_width: 18.0,
                max_wet_span: 180,
                handoff_distance: 414,
                runout: RunoutRules {
                    turns: &[0.0, -0.28, 0.28, -0.52, 0.52, -0.78, 0.78],
                    width_checks: [300.0, 520.0, 730.0],
                    cross_checks: [-160.0, -80.0, 0.0, 80.0, 160.0],
                    step: 36,
                    limit: 720,
                    half_width: 38.0,
                    target_distance: 720.0,
                    strict_edge: true,
                },
            },
            Self::Racer => &SurveyRules {
                headings: &[
                    0.0, -0.10, 0.10, -0.23, 0.23, -0.36, 0.36, -0.50, 0.50, -0.61, 0.61, -0.72,
                    0.72, -0.83, 0.83,
                ],
                coast_threshold: 0.004,
                coarse_step: 36,
                coarse_limit: 1332,
                max_coarse_gap: 252,
                max_landfall: 792,
                dry_proof: 490,
                detailed_runout: 540,
                half_width: 22.0,
                max_wet_span: 300,
                handoff_distance: 300,
                runout: RunoutRules {
                    turns: &[0.0, -0.28, 0.28, -0.57, 0.57, -0.86, 0.86],
                    width_checks: [270.0, 460.0, 650.0],
                    cross_checks: [-145.0, -72.0, 0.0, 72.0, 145.0],
                    step: 34,
                    limit: 648,
                    half_width: 34.0,
                    target_distance: 660.0,
                    strict_edge: false,
                },
            },
        }
    }

    fn dry_approach(self, turn: f32) -> (u32, u32) {
        match self {
            Self::Fighter => (160, 144),
            // A larger heading change needs room to center the Racer before the shore.
            Self::Racer if turn.abs() > 0.60 => (180, 168),
            Self::Racer => (108, 96),
        }
    }

    fn guidance(self) -> ChannelGuidance {
        match self {
            Self::Fighter => ChannelGuidance {
                follow_distance: 220.0,
                max_course_change: 0.46,
                near_water: [120.0, 60.0],
                near_water_lateral_limit: 18.0,
                dry_lateral_limit: 55.0,
                wet_margin: 36.0,
                max_water_seconds: 6,
            },
            Self::Racer => ChannelGuidance {
                follow_distance: 154.0,
                max_course_change: 0.42,
                near_water: [88.0, 48.0],
                near_water_lateral_limit: 22.0,
                dry_lateral_limit: 54.0,
                wet_margin: 28.0,
                max_water_seconds: 12,
            },
        }
    }
}

impl CoastChannel {
    pub(crate) fn scout(
        world: &World,
        origin: [f32; 2],
        heading: f32,
        aircraft: ChannelAircraft,
    ) -> Option<Self> {
        let rules = aircraft.survey_rules();
        rules.headings.iter().copied().find_map(|turn| {
            let (coarse_lead, detailed_lead) = aircraft.dry_approach(turn);
            Self::survey(
                world,
                origin,
                heading + turn,
                aircraft,
                rules,
                coarse_lead,
                detailed_lead,
            )
        })
    }

    pub(crate) fn guidance(self) -> ChannelGuidance {
        self.aircraft.guidance()
    }

    fn survey(
        world: &World,
        origin: [f32; 2],
        heading: f32,
        aircraft: ChannelAircraft,
        rules: &SurveyRules,
        coarse_lead: u32,
        detailed_lead: u32,
    ) -> Option<Self> {
        let (sin, cos) = sin_cos(heading);
        let coast = |along: f32, side: f32| {
            world.coast_signal(
                origin[0] + sin * along + cos * side,
                origin[1] + cos * along - sin * side,
            )
        };
        // Reject distant or split channels before surveying the full corridor.
        let mut first_water = None;
        let mut last_water = 0;
        let mut landfall = None;
        for along in (rules.coarse_step..=rules.coarse_limit).step_by(rules.coarse_step as usize) {
            if coast(along as f32, 0.0) <= rules.coast_threshold {
                if along < coarse_lead || landfall.is_some() {
                    return None;
                }
                let first = *first_water.get_or_insert(along);
                last_water = along;
                if last_water - first > rules.max_coarse_gap {
                    return None;
                }
            } else if last_water > 0 {
                landfall.get_or_insert(along);
                if along >= last_water + rules.dry_proof {
                    break;
                }
            }
        }
        let landfall = landfall?;
        if landfall > rules.max_landfall {
            return None;
        }

        let mut first_wet = None;
        let mut wet_end = 0;
        let mut dry_after = false;
        let mut open_run = 0;
        let mut longest_open_run = 0;
        for along in (DETAIL_STEP..=landfall + rules.detailed_runout).step_by(DETAIL_STEP as usize)
        {
            let lanes =
                [-rules.half_width, 0.0, rules.half_width].map(|side| coast(along as f32, side));
            if lanes
                .into_iter()
                .any(|signal| signal <= rules.coast_threshold)
            {
                if along < detailed_lead || dry_after {
                    return None;
                }
                let first = *first_wet.get_or_insert(along);
                wet_end = along;
                if wet_end - first + 2 * DETAIL_STEP > rules.max_wet_span {
                    return None;
                }
                if lanes.into_iter().all(|signal| signal < -0.012) {
                    open_run += 1;
                    longest_open_run = longest_open_run.max(open_run);
                } else {
                    open_run = 0;
                }
            } else if wet_end > 0 {
                dry_after = true;
            }
        }
        let wet_start = first_wet?;
        if !dry_after || longest_open_run < 4 {
            return None;
        }
        let inland_handoff = (wet_end + rules.handoff_distance) as f32;
        let bend = [
            origin[0] + sin * inland_handoff,
            origin[1] + cos * inland_handoff,
        ];
        let inland_target = rules
            .runout
            .turns
            .iter()
            .copied()
            .find_map(|turn| Self::dry_runout(world, bend, heading + turn, &rules.runout))?;
        Some(Self {
            aircraft,
            origin,
            forward: [sin, cos],
            inland_handoff,
            inland_target,
            wet_span: [
                wet_start.saturating_sub(DETAIL_STEP) as f32,
                (wet_end + DETAIL_STEP) as f32,
            ],
        })
    }

    fn dry_runout(
        world: &World,
        bend: [f32; 2],
        heading: f32,
        rules: &RunoutRules,
    ) -> Option<[f32; 2]> {
        let (sin, cos) = sin_cos(heading);
        let point = |along: f32, side: f32| {
            [
                bend[0] + sin * along + cos * side,
                bend[1] + cos * along - sin * side,
            ]
        };
        let target = point(rules.target_distance, 0.0);
        let edge = target[0].abs().max(target[1].abs());
        if edge > COORDINATE_LIMIT || (rules.strict_edge && edge == COORDINATE_LIMIT) {
            return None;
        }
        let coast = |along, side| {
            let [x, z] = point(along, side);
            world.coast_signal(x, z)
        };
        // The landing must connect to a wider dry area through a dry lead.
        let broad = rules.width_checks.into_iter().all(|along| {
            rules
                .cross_checks
                .into_iter()
                .all(|side| coast(along, side) >= 0.055)
        });
        if !broad {
            return None;
        }
        for along in (rules.step..=rules.limit).step_by(rules.step as usize) {
            if [-rules.half_width, 0.0, rules.half_width]
                .into_iter()
                .any(|side| coast(along as f32, side) < 0.025)
            {
                return None;
            }
        }
        Some(target)
    }
}
