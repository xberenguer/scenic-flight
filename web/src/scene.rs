use scenic_flight::bridge_silhouette::BridgeSilhouettes;
use scenic_flight::coastal_highways::CoastalHighwayWork;
use scenic_flight::coastal_landforms::CoastalLandmarks;
use scenic_flight::distant_roads::DistantRoads;
use scenic_flight::facility_stream::{roadside_reservation, FacilityStream, RoadsideReservation};
use scenic_flight::flight::{Flight, Game, View};
use scenic_flight::ground_details::GroundDetails;
use scenic_flight::highway_art::HighwayRibbon;
use scenic_flight::highway_stream::HighwayStream;
use scenic_flight::horizon_cache::HorizonCache;
use scenic_flight::land_stream::{LandBudget, LandStream};
use scenic_flight::local_routes::{route_reservation, LocalRouteStream};
use scenic_flight::natural_stream::{
    highway_reservation_with_nature, scenery_reservation_with_highways, NaturalBudget,
    NaturalStream,
};
use scenic_flight::rail_art::{
    corridor_screen_region, draw_rail, draw_rail_corridor, rail_screen_region,
};
use scenic_flight::rail_stream::{scene_blocks, RailStream};
use scenic_flight::rail_survey::{RailSurveyData, RailSurveyWork};
use scenic_flight::render::{
    render_terrain_layer_region_with_horizon, view_distance, Camera, ScreenRect,
};
use scenic_flight::river_stream::RiverStream;
use scenic_flight::scenery::CoastalScenery;
use scenic_flight::sky::Sky;
use scenic_flight::skyline::Skyline;
use scenic_flight::terrain_cache::TerrainCache;
use scenic_flight::train_art::{Train, TRAIN_REACH};
use scenic_flight::watershed::WatershedWork;
use scenic_flight::weather_effects::{ValleyMist, WeatherEffects};
use scenic_flight::world::World;
use scenic_flight::world_mesh::{bounds_in_view, WorldMesh};

use super::{DIMENSIONS, PIXELS};

pub(super) struct Scene {
    world: World,
    seed: u64,
    terrain: TerrainCache,
    horizon: HorizonCache,
    ground: GroundDetails,
    coast: CoastalScenery,
    cliffs: CoastalLandmarks,
    land: LandStream<8>,
    facilities: FacilityStream<8>,
    rivers: RiverStream<3>,
    river_work: WatershedWork,
    routes: LocalRouteStream<8>,
    regional_rail: Box<RailStream>,
    rail_data: Box<RailSurveyData>,
    rail_work: Box<RailSurveyWork>,
    highways: HighwayStream<4>,
    highway_work: CoastalHighwayWork,
    natural: NaturalStream,
    skyline: Skyline,
    distant_roads: DistantRoads,
    distant_bridges: BridgeSilhouettes,
    weather: WeatherEffects,
    valley_mist: ValleyMist,
    work: Box<[u16]>,
    depth: Box<[u16]>,
}

impl Scene {
    pub(super) fn new(game: &mut Game) -> Box<Self> {
        let seed = game.world_seed().unwrap();
        let (world, flight) = game.active().unwrap();
        let world = *world;
        let mut view = camera(flight.view);
        let mut scene = Box::new(Self {
            world,
            seed,
            terrain: TerrainCache::new(),
            horizon: HorizonCache::new(),
            ground: GroundDetails::new(),
            coast: CoastalScenery::new(),
            cliffs: CoastalLandmarks::new(),
            land: LandStream::new(),
            facilities: FacilityStream::new(world, seed),
            rivers: RiverStream::new(world, seed),
            river_work: WatershedWork::new(),
            routes: LocalRouteStream::new(world, seed),
            regional_rail: Box::new(RailStream::new(world, seed)),
            rail_data: Box::new(RailSurveyData::new()),
            rail_work: Box::new(RailSurveyWork::new()),
            highways: HighwayStream::new(world, seed),
            highway_work: CoastalHighwayWork::new(),
            natural: NaturalStream::new(world, seed),
            skyline: Skyline::new(),
            distant_roads: DistantRoads::new(),
            distant_bridges: BridgeSilhouettes::new(),
            weather: WeatherEffects::new(),
            valley_mist: ValleyMist::new(),
            work: vec![0; 2 * PIXELS].into_boxed_slice(),
            depth: vec![0; PIXELS].into_boxed_slice(),
        });
        scene.land.reset(world);
        scene.land.preflight(view, false);
        if game.stage_town_approach(&scene.land) {
            view = camera(game.active().unwrap().1.view);
            scene.land.preflight(view, false);
        }
        scene
            .terrain
            .reset_for_altitude(world, view.x, view.z, view.altitude);
        scene.horizon.reset(world, view.x, view.z);
        scene.ground.reset(&world, seed, view.x, view.z);
        scene.coast.reset(world, seed, view.x, view.z);
        scene.cliffs.reset(&world, seed, view.x, view.z);
        scene.facilities.preflight(&scene.land);
        scene
            .rivers
            .preflight(&scene.land, &scene.facilities, view, &mut scene.river_work);
        scene.routes.preflight(&scene.land, view, |key, route| {
            route_reservation(&world, &scene.facilities, &scene.rivers, key, route)
        });
        scene.highways.preflight_defer_coast(
            &scene.land,
            view,
            |pair, link| {
                highway_reservation_with_nature(
                    &scene.land,
                    &scene.facilities,
                    &scene.rivers,
                    &scene.routes,
                    &scene.natural,
                    pair,
                    link,
                )
            },
            || {},
        );
        scene.natural.preflight(
            view,
            NaturalBudget {
                alpine_tiles: 12,
                dry_tiles: 12,
            },
            |bounds| {
                scenery_reservation_with_highways(
                    &scene.land,
                    &scene.facilities,
                    &scene.rivers,
                    &scene.routes,
                    &scene.highways,
                    bounds,
                )
            },
        );
        scene.publish_roadside(view, true);
        scene
    }

    pub(super) fn advance(&mut self, game: &mut Game) {
        let (_, flight) = game.active().unwrap();
        if flight.should_observe_trains() {
            let view = camera(flight.view);
            let nearest = self
                .facilities
                .rails(view, 112.0)
                .filter_map(|line| Train::line_motion(line, self.seed, flight.ticks))
                .chain(
                    self.regional_rail
                        .rails(view, 112.0)
                        .filter_map(|line| Train::corridor_motion(line, self.seed, flight.ticks)),
                )
                .filter_map(|train| flight.train_score(train).map(|score| (score, train)))
                .min_by(|left, right| left.0.total_cmp(&right.0))
                .map(|(_, train)| train);
            game.observe_train(nearest);
        }
        game.tick_with_obstacles(|x, z| {
            self.land
                .obstruction(x, z)
                .max(self.facilities.obstruction(x, z))
                .max(self.cliffs.obstruction(x, z))
                .max(self.rivers.obstruction(x, z))
                .max(self.natural.obstruction(x, z))
                .max(self.routes.obstruction(x, z))
                .max(self.highways.obstruction(x, z))
        });
        let (_, flight) = game.active().unwrap();
        let ticks = flight.ticks;
        let view = camera(flight.view);
        if ticks.is_multiple_of(2) {
            self.land.service(
                view,
                LandBudget {
                    expansions: 8,
                    terrain_samples: 32,
                    settlement_jobs: u8::from(ticks.is_multiple_of(16)),
                },
            );
        }
        if ticks.is_multiple_of(16) {
            self.facilities.service(&self.land, view);
        }
        self.cliffs
            .follow(&self.world, self.seed, view.x, view.z, 1);
        self.rivers.follow(&self.land, view);
        if ticks % 16 == 4 {
            if let Some(request) = self.rivers.request(&self.land, &self.facilities, view) {
                let key = request.key();
                let result = request.prepare(
                    &mut self.river_work,
                    self.land.planned(key).flatten(),
                    self.facilities.planned(key),
                    || {},
                );
                self.rivers
                    .commit_avoiding(result, &self.land, view, |river| {
                        self.natural.reserves_river(river)
                            || self.facilities.reserves_roadside_river(river)
                            || self
                                .regional_rail
                                .conflicts_with(|bounds| river.intersects_scenery(bounds))
                            || self.highways.reserves_river(river)
                            || self.routes.routes().any(|(_, route)| {
                                route.ground_conflicts_with(|bounds| {
                                    river.intersects_scenery(bounds)
                                })
                            })
                    });
            }
        }
        self.routes
            .service(&self.land, view, ticks % 16 == 12, |key, route| {
                if route.ground_conflicts_with(|bounds| self.regional_rail.reserves(bounds)) {
                    return scenic_flight::local_routes::Reservation::Blocked;
                }
                route_reservation(&self.world, &self.facilities, &self.rivers, key, route)
            });
        if ticks % 16 == 8 {
            self.highways
                .service_with_reservations(&self.land, view, true, |pair, link| {
                    if self
                        .regional_rail
                        .conflicts_with(|bounds| link.intersects_bounds(bounds))
                    {
                        return scenic_flight::highway_stream::Reservation::Blocked;
                    }
                    highway_reservation_with_nature(
                        &self.land,
                        &self.facilities,
                        &self.rivers,
                        &self.routes,
                        &self.natural,
                        pair,
                        link,
                    )
                });
            if let Some(request) = self.highways.request(&self.land, view, true) {
                let result = request.prepare_with_land(&mut self.highway_work, &self.land, || {});
                self.highways.commit(result, &self.land, view);
            }
        }
        if ticks.is_multiple_of(4) {
            self.service_rail(view);
            self.natural.follow(
                view,
                NaturalBudget {
                    alpine_tiles: 1,
                    dry_tiles: 1,
                },
                |bounds| {
                    if self.regional_rail.reserves(bounds) {
                        return scenic_flight::natural_stream::NaturalReservation::Reserved;
                    }
                    scenery_reservation_with_highways(
                        &self.land,
                        &self.facilities,
                        &self.rivers,
                        &self.routes,
                        &self.highways,
                        bounds,
                    )
                },
            );
        }
        // Survey on a tick away from the other regional planners.
        if ticks % 32 == 30 {
            self.survey_rail(view);
        }
        self.publish_roadside(view, false);
        self.regional_rail.connect_quarries(&self.land, &self.facilities, view);
    }

    fn publish_roadside(&mut self, view: Camera, before_first_frame: bool) {
        self.facilities.publish_roadside_sites(view, before_first_frame, |bounds| {
            if self.regional_rail.reserves(bounds) {
                RoadsideReservation::Reserved
            } else {
                roadside_reservation(&self.land, &self.rivers, &self.routes, &self.highways, &self.natural, bounds)
            }
        });
    }

    fn service_rail(&mut self, view: Camera) {
        self.regional_rail.service(&self.land, view, |bounds| {
            self.facilities.reserves_roadside_site(bounds)
                || scene_blocks(
                    &self.land,
                    &self.rivers,
                    &self.routes,
                    &self.natural,
                    &self.highways,
                    bounds,
                )
        });
    }

    fn survey_rail(&mut self, view: Camera) {
        let Some(request) =
            self.regional_rail
                .request(&self.land, &self.facilities, &self.rivers, view)
        else {
            return;
        };
        let [a, b] = request.pair.regions();
        let owners = (
            self.land.planned(a).flatten(),
            self.land.planned(b).flatten(),
            self.facilities.planned(a),
            self.facilities.planned(b),
        );
        if let (Some(a), Some(b), Some(af), Some(bf)) = owners {
            if self
                .rail_data
                .load([a, b], [af, bf], &self.highways, request.pair)
            {
                let corridor = self.rail_data.prepare(
                    &self.world,
                    self.seed,
                    &mut self.rail_work,
                    &mut self.river_work,
                    || {},
                );
                self.regional_rail
                    .commit(request, corridor, &self.land, view);
                return;
            }
        }
        self.regional_rail.cancel(request);
    }

    pub(super) fn draw(&mut self, flight: &Flight, pixels: &mut [u16]) -> Sky {
        let view = camera(flight.view);
        self.terrain.follow(view.x, view.z);
        self.horizon.follow(view.x, view.z, 32);
        self.ground.follow(&self.world, view.x, view.z);
        self.coast.follow(view.x, view.z);

        let sky = Sky::at_position(self.seed, flight.ticks, [view.x, view.altitude, view.z]);
        let conditions = self.weather.observe(self.seed, flight.ticks, view, &sky);
        let sky = sky.with_shower(conditions.shower);
        let mist =
            self.valley_mist
                .observe(&self.world, self.seed, flight.ticks, view, &sky, DIMENSIONS);
        let skyline =
            self.skyline
                .prepare(&self.land, view, flight.ticks, &sky, DIMENSIONS, |x, z| {
                    self.horizon.backdrop(x, z)
                });
        self.distant_roads.prepare(
            &self.land,
            &self.highways,
            view,
            flight.ticks,
            DIMENSIONS,
            |x, z| self.horizon.backdrop(x, z),
        );
        let far_bridges = self.distant_bridges.prepare(
            view,
            flight.ticks,
            &sky,
            DIMENSIONS,
            self.rivers
                .bridges()
                .chain(self.highways.coastal_links().map(|(_, link)| link.bridge())),
            |x, z| self.horizon.backdrop(x, z),
        );
        let visible_cliff = |form: &&scenic_flight::coastal_landforms::CoastalLandform| {
            bounds_in_view(view, form.bounds()) && self.facilities.retains_landmark(form)
        };
        let has_cliffs = self.cliffs.forms().any(|form| visible_cliff(&form));
        let bridge_region = self.rivers.bridge_region(view, DIMENSIONS);
        let crossing_region = self.highways.crossing_region(view, DIMENSIONS);
        let has_highways = self.highways.has_visible_links(view);
        let inland_ribbons: [_; 4] = {
            let mut links = self
                .highways
                .links()
                .filter(|_| has_highways)
                .filter_map(|(_, link)| HighwayRibbon::new(link, view, DIMENSIONS));
            std::array::from_fn(|_| links.next())
        };
        let coast_ribbons: [_; 8] = {
            let mut approaches = self
                .highways
                .coastal_links()
                .filter(|_| has_highways)
                .flat_map(|(_, link)| link.approaches())
                .filter_map(|points| HighwayRibbon::for_approach(points, view, DIMENSIONS));
            std::array::from_fn(|_| approaches.next())
        };
        let ribbons = || inland_ribbons.iter().chain(coast_ribbons.iter()).flatten();
        let waterfall_region = self.natural.waterfall_region(view, DIMENSIONS);
        let route_region = self.routes.depth_region(view, DIMENSIONS);
        let tracks_region = self
            .facilities
            .rails(view, 78.0)
            .filter_map(|line| rail_screen_region(line, view, DIMENSIONS))
            .reduce(ScreenRect::union);
        let corridors = || self.regional_rail.rails(view, view_distance(view.altitude));
        let corridor_region = corridors()
            .filter_map(|line| corridor_screen_region(line, view, DIMENSIONS))
            .reduce(ScreenRect::union);
        let trains: [_; 2] = {
            let mut trains = self
                .facilities
                .rails(view, TRAIN_REACH)
                .filter_map(|line| Train::at(line, self.seed, flight.ticks, view, DIMENSIONS))
                .chain(
                    self.regional_rail
                        .rails(view, TRAIN_REACH)
                        .filter_map(|line| {
                            Train::on_corridor(line, self.seed, flight.ticks, view, DIMENSIONS)
                        }),
                );
            std::array::from_fn(|_| trains.next())
        };
        let train_region = trains
            .iter()
            .flatten()
            .map(Train::depth_region)
            .reduce(ScreenRect::union);
        let land = self
            .land
            .has_visible_scenery(view)
            .then(|| self.land.paint_sampler());
        let facilities = self
            .facilities
            .has_visible_facilities(view)
            .then(|| self.facilities.sampler());
        let rivers = self
            .rivers
            .has_visible_rivers(view)
            .then(|| self.rivers.sampler());
        let natural = self
            .natural
            .has_visible_features(view)
            .then(|| self.natural.sampler());
        let highways = has_highways.then(|| self.highways.sampler());
        let rail = corridors().next().map(|_| self.regional_rail.sampler());
        let has_routes = self.routes.routes().next().is_some();
        let trees = self.ground.tree_sampler().for_view(view.x, view.altitude, view.z);
        let sample = |x, z| {
            let raw = self.terrain.sample(x, z);
            let road = highways
                .as_ref()
                .map_or(raw, |layer| layer.overlay(x, z, raw));
            let ground = natural
                .as_ref()
                .map_or(road, |layer| layer.overlay(x, z, road));
            let covered = trees.geometry(x, z, ground);
            let developed = land
                .as_ref()
                .map_or(covered, |layer| layer.overlay(x, z, covered));
            let water = rivers
                .as_ref()
                .map_or(developed, |layer| layer.overlay(x, z, developed));
            let coast = self.coast.overlay(x, z, water);
            let built = facilities
                .as_ref()
                .map_or(coast, |layer| layer.overlay(x, z, raw, coast));
            let routed = if has_routes {
                self.routes.overlay(x, z, built)
            } else {
                built
            };
            rail.as_ref()
                .map_or(routed, |rail| rail.overlay(x, z, routed))
        };

        let desert_props = scenic_flight::desert_props::DesertProps::gather_with_wetlands(
            view,
            DIMENSIONS,
            self.seed,
            rivers.is_some(),
            |x, z| self.terrain.sample(x, z),
            &sample,
        );
        let depth_region = if has_cliffs || conditions.distant_rain.is_some() {
            Some(ScreenRect::full(DIMENSIONS))
        } else {
            skyline
                .bounds()
                .into_iter()
                .chain(bridge_region)
                .chain(crossing_region)
                .chain(far_bridges.bounds())
                .chain(self.distant_roads.bounds())
                .chain(ribbons().map(HighwayRibbon::depth_region))
                .chain(waterfall_region)
                .chain(route_region)
                .chain(desert_props.region())
                .chain(tracks_region)
                .chain(corridor_region)
                .chain(train_region)
                .chain(mist.depth_region())
                .reduce(ScreenRect::union)
        };

        render_terrain_layer_region_with_horizon(
            (&sample, |x, z, geometry| {
                self.ground.visible_material(x, z, geometry)
            }),
            |x, z| self.horizon.backdrop(x, z),
            view,
            &sky,
            pixels,
            &mut self.work,
            depth_region.map(|region| (&mut self.depth[..], region)),
            DIMENSIONS,
            |_| {},
            |_| None,
        )
        .expect("valid browser framebuffer and flight camera");

        if depth_region.is_some() {
            self.distant_roads
                .draw(pixels, &self.depth, &mut self.work, DIMENSIONS);
            far_bridges.draw(pixels, &self.depth);
            skyline.draw(pixels, &self.depth);
            if route_region.is_some() {
                self.routes
                    .open_portals(view, pixels, &mut self.depth, DIMENSIONS);
            }
            let mut mesh = WorldMesh::new(pixels, &mut self.depth, DIMENSIONS, view, &sky).unwrap();
            desert_props.draw(&mut mesh);
            for ribbon in ribbons() {
                ribbon.draw(&mut mesh, &sample);
            }
            if crossing_region.is_some() {
                self.highways
                    .draw_crossings_fading(view, DIMENSIONS, &mut mesh, |bridge| {
                        far_bridges.detail_opacity(bridge)
                    });
            }
            for form in self.cliffs.forms().filter(visible_cliff) {
                form.draw(&mut mesh);
            }
            for line in self.facilities.rails(view, 78.0) {
                draw_rail(&mut mesh, line, view, DIMENSIONS);
            }
            for line in corridors() {
                draw_rail_corridor(&mut mesh, line, view, DIMENSIONS);
            }
            for train in trains.iter().flatten() {
                train.draw(&mut mesh, sky.daylight);
            }
            if bridge_region.is_some() {
                self.rivers
                    .draw_fading(view, &mut mesh, |bridge| far_bridges.detail_opacity(bridge));
            }
            if waterfall_region.is_some() {
                self.natural.draw_waterfalls(view, flight.ticks, &mut mesh);
            }
            if route_region.is_some() {
                self.routes.draw(view, flight.ticks, &mut mesh);
            }
        }
        if let Some(rain) = conditions.distant_rain {
            rain.paint(view, DIMENSIONS, pixels, &self.depth);
        }
        mist.paint(view, DIMENSIONS, pixels, &self.depth);
        sky
    }
}

fn camera(view: View) -> Camera {
    Camera {
        x: view.x,
        z: view.z,
        altitude: view.altitude,
        heading: view.heading,
        pitch: view.pitch,
        bank: view.bank,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scenic_flight::controls::Buttons;

    #[test]
    fn browser_scene_surveys_publishes_and_draws_a_regional_train() {
        let mut game = Game::new(3_239_381_244_091_398_388);
        let start = Buttons {
            small: false,
            large: true,
        };
        game.read_buttons(0, start);
        game.read_buttons(30, start);
        let mut flight = game.active().unwrap().1.clone();
        let mut scene = Scene::new(&mut game);
        assert_eq!(scene.seed, 7_911);
        let view = Camera {
            x: -1_510.0,
            z: 3_970.0,
            altitude: 72.0,
            heading: -0.16,
            pitch: 0.12,
            bank: 0.0,
        };
        scene.land.reset(scene.world);
        assert!(scene.land.preflight(view, true));
        scene.facilities.preflight(&scene.land);
        scene.rivers = RiverStream::new(scene.world, scene.seed);
        scene
            .rivers
            .preflight(&scene.land, &scene.facilities, view, &mut scene.river_work);
        scene.routes = LocalRouteStream::new(scene.world, scene.seed);
        scene.routes.preflight(&scene.land, view, |key, route| {
            route_reservation(&scene.world, &scene.facilities, &scene.rivers, key, route)
        });
        scene.highways = HighwayStream::new(scene.world, scene.seed);
        scene.natural = NaturalStream::new(scene.world, scene.seed);
        scene.survey_rail(view);
        for _ in 0..64 {
            scene.service_rail(view);
        }
        let away = Camera {
            heading: core::f32::consts::PI,
            z: view.z - 1_000.0,
            ..view
        };
        scene.service_rail(away);
        let line = scene.regional_rail.rails(view, 512.0).next().unwrap();
        assert!(line.length() > 500.0);
        let midpoint = line.position_at(line.length() * 0.5);
        let watch = Camera {
            x: midpoint.x - 20.0,
            z: midpoint.z - 76.0,
            altitude: midpoint.elevation + 24.0,
            heading: 0.26,
            pitch: 0.30,
            bank: 0.07,
        };
        flight.view = View {
            x: watch.x,
            z: watch.z,
            altitude: watch.altitude,
            heading: watch.heading,
            pitch: watch.pitch,
            bank: watch.bank,
        };
        flight.ticks = (0..12_000)
            .step_by(11)
            .find(|&tick| {
                Train::corridor_motion(line, scene.seed, tick).is_some()
                    && Train::on_corridor(line, scene.seed, tick, watch, DIMENSIONS).is_some()
            })
            .unwrap();
        scene
            .terrain
            .reset_for_altitude(scene.world, watch.x, watch.z, watch.altitude);
        scene.horizon.reset(scene.world, watch.x, watch.z);
        let mut composed = vec![0; PIXELS];
        scene.draw(&flight, &mut composed);
        scene.regional_rail.reset(scene.world, scene.seed);
        let mut base = vec![0; PIXELS];
        scene.draw(&flight, &mut base);
        assert!(composed.iter().zip(base).filter(|(&a, b)| a != *b).count() > 20);
    }
}
