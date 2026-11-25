//! Owned requests and replies exchanged with the display core.

use core::mem::MaybeUninit;

use scenic_flight::facility_stream::{FacilityRequest, FacilityResult};
#[cfg(feature = "coastal-highways")]
use scenic_flight::coastal_highways::CoastalHighwayWork;
#[cfg(feature = "coastal-highways")]
use scenic_flight::highway_stream::HighwayTowns;
#[cfg(feature = "local-routes")]
use scenic_flight::local_routes::{RouteRequest, RouteResult};
#[cfg(feature = "natural-scenery")]
use scenic_flight::natural_stream::{NaturalRequest, NaturalResult};
#[cfg(feature = "natural-scenery")]
use scenic_flight::summit_stream::{SummitRequest, SummitResult};
#[cfg(feature = "highway-stream")]
use scenic_flight::highway_stream::{HighwayRequest, HighwayResult};
#[cfg(feature = "regional-rail")]
use scenic_flight::{
    rail_corridor::RailCorridor,
    rail_stream::RailRequest,
    rail_survey::{RailSurveyData, RailSurveyWork},
};
#[cfg(feature = "river-stream")]
use scenic_flight::{
    facilities::RegionalFacilities,
    river_stream::{RiverRequest, RiverResult},
    scenery::DistrictRegion,
    watershed::WatershedWork,
};

pub enum Survey {
    Facility(FacilityRequest),
    #[cfg(feature = "local-routes")]
    Route(RouteRequest),
    #[cfg(feature = "natural-scenery")]
    Natural(NaturalRequest),
    #[cfg(feature = "natural-scenery")]
    Summit(SummitRequest),
    #[cfg(feature = "highway-stream")]
    Highway {
        request: HighwayRequest,
        #[cfg(feature = "coastal-highways")]
        towns: HighwayTowns,
    },
    #[cfg(feature = "regional-rail")]
    RegionalRail(RailRequest),
    #[cfg(feature = "river-stream")]
    River {
        request: RiverRequest,
        district: Option<DistrictRegion>,
        facilities: Option<RegionalFacilities>,
    },
}

pub enum SurveyResult {
    Facility(FacilityResult),
    #[cfg(feature = "local-routes")]
    Route(RouteResult),
    #[cfg(feature = "natural-scenery")]
    Natural(NaturalResult),
    #[cfg(feature = "natural-scenery")]
    Summit(SummitResult),
    #[cfg(feature = "highway-stream")]
    Highway(HighwayResult),
    #[cfg(feature = "regional-rail")]
    RegionalRail { request: RailRequest, corridor: Option<RailCorridor> },
    #[cfg(feature = "river-stream")]
    River(RiverResult),
}

pub struct SurveySlots {
    pub request: MaybeUninit<Survey>,
    pub result: MaybeUninit<SurveyResult>,
    #[cfg(feature = "regional-rail")]
    pub rail: RailSurveyData,
}

impl SurveySlots {
    pub const fn new() -> Self {
        Self {
            request: MaybeUninit::uninit(), result: MaybeUninit::uninit(),
            #[cfg(feature = "regional-rail")]
            rail: RailSurveyData::new(),
        }
    }
}

impl Survey {
    pub fn prepare_into(
        &self,
        slot: &mut MaybeUninit<SurveyResult>,
        #[cfg(feature = "river-stream")] water: &mut WatershedWork,
        #[cfg(feature = "coastal-highways")] coast: &mut CoastalHighwayWork,
        #[cfg(feature = "regional-rail")] rail: (&RailSurveyData, &mut RailSurveyWork),
        pulse: impl FnMut(),
    ) {
        match self {
            Self::Facility(request) => prepare_one(slot, || request.prepare(pulse), SurveyResult::Facility),
            #[cfg(feature = "local-routes")]
            Self::Route(request) => prepare_one(slot, || request.prepare(pulse), SurveyResult::Route),
            #[cfg(feature = "natural-scenery")]
            Self::Natural(request) => prepare_one(slot, || request.prepare(pulse), SurveyResult::Natural),
            #[cfg(feature = "natural-scenery")]
            Self::Summit(request) => prepare_one(slot, || request.prepare(pulse), SurveyResult::Summit),
            #[cfg(feature = "highway-stream")]
            Self::Highway {
                request,
                #[cfg(feature = "coastal-highways")]
                towns,
            } => {
                #[cfg(feature = "coastal-highways")]
                prepare_one(slot, || request.prepare_with_towns(coast, towns, pulse), SurveyResult::Highway);
                #[cfg(not(feature = "coastal-highways"))]
                prepare_one(slot, || request.prepare(pulse), SurveyResult::Highway);
            }
            #[cfg(feature = "regional-rail")]
            Self::RegionalRail(request) => prepare_one(
                slot,
                || rail.0.prepare(&request.world, request.seed, rail.1, water, pulse),
                |corridor| SurveyResult::RegionalRail { request: *request, corridor },
            ),
            #[cfg(feature = "river-stream")]
            Self::River { request, district, facilities } => {
                prepare_one(
                    slot,
                    || request.prepare(water, district.as_ref(), facilities.as_ref(), pulse),
                    SurveyResult::River,
                );
            }
        }
    }
}

// Each survey gets its own stack frame and publishes into the shared result
// slot before returning. This keeps unused survey scratch off the worker stack.
#[inline(never)]
fn prepare_one<T>(
    slot: &mut MaybeUninit<SurveyResult>,
    prepare: impl FnOnce() -> T,
    wrap: impl FnOnce(T) -> SurveyResult,
) {
    slot.write(wrap(prepare()));
}
