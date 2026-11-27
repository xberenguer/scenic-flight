# How Scenic Flight generates a world

[Play Scenic Flight](https://xberenguer.github.io/scenic-flight/play.html) · [Gameplay and artwork](../README.md)

A world seed determines where terrain, roads, cities, airfields and coastlines form. Flight time advances the sky, weather and encounters over that layout.

## Metropolises

About one in eight world seeds opens over a metropolis. Curved streets and wider avenues share intersections across the whole basin. Four business districts are surrounded by housing, mixed-use streets, industrial yards, campuses, and wooded parks. Each district uses five building-block designs, with varied roof materials, orientations, and heights.

Seed **15** spans approximately **23,936 × 24,704 world units**, measured from developed surfaces on a 32-unit grid. The footprint varies with the seed and includes public parks.

![Map of the generated metropolis in seed 15](assets/planning/metropolis-seed-15.png)

![Metropolitan flight views from the Glider, Blackbird, and Sport Jet](assets/planning/metropolis-flyover.png)

## Aerial visibility

Low flights draw terrain detail out to 800 world units. At high cruise, ground details extend to 1,440 units. The renderer samples roofs and streets more closely beneath high aircraft and spaces out samples on their far approach. Broken cloud layers drift over the terrain according to regional weather. Cloud tops and soft shadows share the same wind, with gaps over the ground in partly cloudy conditions.

![Blackbird above broken clouds and a metropolitan street grid](assets/current/blackbird-city-cruise.png)

## Metropolitan outskirts

Major city roads continue into twelve approaches through farming country. Smaller settlements form near the junctions, and separate woodland patches break up the fields. Vineyards, orchards, and pasture are placed on gentle ground. Villages include cottages, schools and workshops alongside farms, silos and glasshouses.

![The roads and cultivation in the outskirts of the metropolis in seed 15](assets/planning/outskirts-seed-15.png)

[Fly over the outskirts](https://xberenguer.github.io/scenic-flight/play.html?session=642)

## Metropolitan airport

Each metropolis has two parallel runways on a graded site. Taxiways connect both runways to a shared apron with eight gates. Parked aircraft vary by gate. Five terminal plans share a concourse so their jet bridges reach the gates. The site also has hangars, cargo stands, a control tower, parking and a road that joins a city street.

[Fly a metropolitan airport approach](https://xberenguer.github.io/scenic-flight/play.html?session=107)

The map uses the airport in seed **19**. Yellow indicates taxi guidance and white indicates runway and stand markings. The city road meets the airport at the top.

![Generated airport layout with runways, gates, cargo apron and its road connection](assets/planning/metropolitan-airport.png)

## Metropolitan rail

The Crosstown, Central, and Airport lines follow the city's curved streets. Trains stop at platforms, travel on separate tracks in each direction, and reverse at terminals. The airport station is in the terminal forecourt. Two interchanges connect the services on separate levels.

Seed **15** has **42 station sites**. Blue marks Crosstown, red marks Central, and gold marks the Airport line. The larger circles mark interchanges.

![Metropolitan rail routes, airport station, and two interchanges](assets/planning/metro-rail-seed-15.png)

[Fly alongside metropolitan rail](https://xberenguer.github.io/scenic-flight/play.html?session=215)

## City sizes

The survey of ordinary settlements ran from the Racer's starting area in seeds **0–255** using the same terrain, street, waterway and settlement preparation as the flight renderer. It found **531 settlements** in 190 of those 256 starting areas. A settlement includes its core streets, subdivisions and outer boroughs. Distances are in engine world units.

| Measurement | Result |
| --- | ---: |
| Median longest side | 474 units |
| Median streets, roofs, industrial sites and parks combined | 57,120 square units |
| Longest measured settlement | 814 × 400 units |
| Largest developed footprint | 123,116 square units within 616 × 552 units |
| Road planning region | 1,024 × 1,024 units |
| Spacing between intersections in the core street grid | 18 units |
| Outer collector grid spacing | 36 or 54 units |

Developed surfaces covered about 36% of the largest measured footprint’s bounding rectangle. This was a survey of generated starting areas. The dimensions of individual towns depend on the seed and buildable land. Open land between two settlements and the highway joining them are excluded from city coverage. Measurements use a 2-unit sampling grid, so narrow roads and small roofs introduce some measurement variation. In the raw table, road end 0 is the start of the regional road and 1 is its far terminus. [Raw measurements](assets/planning/settlement-sample.tsv)

## Generated street maps

Each map shows one 1,024-unit road planning region at the same scale. Cream and orange mark rooftops, tan marks streets, lavender marks industrial buildings, green marks civic green space and blue marks water. Seed and region coordinates identify the source. Regions can contain multiple settlements.

| Seed 149 · region −6, −4 | Seed 241 · region −2, −4 |
| --- | --- |
| ![Street map of the largest measured developed footprint](assets/planning/urban-149--6--4.png) | ![Street map of the longest measured settlement](assets/planning/urban-241--2--4.png) |

| Seed 21 · region 2, 2 | Seed 24 · region −1, 5 |
| --- | --- |
| ![Street map of a settlement with a tower cluster and peripheral districts](assets/planning/urban-21-2-2.png) | ![Street map of a settlement with shorter wings and a distinct approach](assets/planning/urban-24--1-5.png) |

Two more examples show [a hillside settlement](assets/planning/urban-16--3--7.png) and [an open-grid town](assets/planning/urban-15-1-0.png).

## Roads, districts and buildings

The road planner looks for pairs of dry approaches it can connect at a usable grade. A road terminus may grow into a town if its slope permits a connected street network and enough buildings. The center starts with a 7-column street grid that can reach 7 rows. It can gain hillside extensions and shift with the terrain. [Road planner](../src/roads.rs) · [Town generation](../src/settlement.rs)

The 25 neighborhood templates define where the next set of streets may grow. Each candidate street still has to connect and pass the ground checks. Farther out, wider-spaced collector roads support houses, shops, industry and open blocks. Other planned land uses, waterways and steep ground can cut gaps in the street network. When a dry approach reaches a usable shore, the same survey can site a waterfront extension. [Neighborhood templates](../src/settlement/district_layouts.rs) · [Outer boroughs](../src/settlement/expansion/boroughs.rs) · [Waterfront extensions](../src/settlement/expansion/waterfront.rs)

## Neighborhoods along highways

Across 320 starting seeds in v0.1.121, the road survey placed **148 highway neighborhoods** and produced all **25 layouts**. A collector starts at an existing road, climbs only where the grade allows, and grows side streets and buildable lots. The layouts include cottage rows, duplex courts, shops, workshops, and mixed-use streets. The survey leaves unsuitable ground undeveloped.

Published highways can provide new street connections across planning regions. Each roadside neighborhood reserves its own street and lots inside its region. The highway joins the regions through the existing road network.

In the examples below, cyan marks the added streets and gold marks their homes and shops.

| Shops and duplexes · seed 18 | Workshops and village streets · seed 33 |
| --- | --- |
| ![Generated highway neighborhoods below a regional town](assets/planning/urban-frontage-18.png) | ![Generated highway neighborhoods at the edge of a coastal town](assets/planning/urban-frontage-33.png) |

[Frontage planner](../src/settlement/frontage.rs) · [Layout atlas](assets/artwork/frontage-layouts.png)

## A road between two towns

When a regional road's regular end cannot reach the next town, the survey can try the outer ends of streets already connected to each town center. These routes keep their terrain grade and reserve ground around buildings, waterways, and other infrastructure. The planner tests 26 different alignments before choosing a route that fits.

The map below shows seed 93. The accepted highway is about **575 world units** between the two street connections and crosses the boundary between the two surveyed regions.

![Street grids joined by a cross-region highway](assets/planning/urban-connected-93.png)

[Road alignment atlas](assets/artwork/town-road-alignments.png) · [Highway planner](../src/highway_links.rs)

## Local building finishes

Civic landmarks use finishes selected from local terrain. Coastal areas favor whitewash, blue roofs, slate, and weathered finishes. Dry inland districts favor clay and sandstone. Upland areas use stone, slate, and weathered wood tones. The town and riverfront atlases show 50 individual roof and wall combinations.

## Dry terrain

Scrub, dry washes, dunes, sandstone and salt flats each have five surface patterns. Scrub gathers in patches and thins on steep ground. Washes show gravel bars and silt, dunes have drift and lee marks, sandstone shows bedding, and salt basins show crust and cracks. Broad markings remain visible after small details fade with distance. [Dry surface atlas](assets/artwork/dry-surfaces.png)

Dry foothills use 25 three-dimensional rock and plant groups. Gravel bars and driftwood sit in wash floors. Dune grasses and evening primrose grow on sand, while fragmented ledges and scree occupy rocky slopes. The footprint follows the local grade and clears pavement and water. Larger groups start fading into view before the smaller foreground objects. [Foothill model atlas](assets/artwork/dry-foothill-models.png)

Sandstone ridges form on surveyed dry slopes. There are 25 profiles, including paired fins, stepped escarpments, sawtooth crests, and detached towers. The same profiles shape the near terrain and the distant horizon. Their distant relief fades between 1,050 and 1,400 world units. The survey checks for conflicting land uses before publishing a ridge. [Sandstone ridge atlas](assets/artwork/sandstone-ridges.png)

## What can appear where

| System | Placement |
| --- | --- |
| Terrain and ground cover | Elevation and climate shape coasts, open land, forest, rock, desert and snow cover. Plants and ground details vary with habitat, land form and seed. [Terrain](../src/world.rs) · [Ground details](../src/ground_details.rs) |
| Rivers and alpine lakes | Drainage follows the terrain. The regional river planner places banks and crossings where they fit. Alpine lake basins have their own survey. [Watersheds](../src/watershed.rs) · [Alpine lakes](../src/alpine_lakes.rs) |
| Ports, vessels and lighthouses | Port candidates need a coastal road approach, compatible shoreline, enough water depth and a pier site. Moorings and lighthouse headlands use separate coastal surveys. [Ports](../src/ports.rs) · [Anchorages](../src/maritime.rs) · [Lighthouses](../src/lighthouses.rs) |
| Airfields | Country strips need enough open, suitable ground for a runway. Regional airfields also plan their access from the road network. [Airfields](../src/airfield.rs) |
| Rail and industry | Regional rail corridors join suitable endpoints and can serve quarries, terminals and stations. Trains follow the surveyed corridors. [Rail surveys](../src/rail_survey.rs) · [Regional rail](../src/rail_stream.rs) |
| Sky and weather | The sun follows a seasonal path. The Moon changes phase. Local cloud cover, haze and elevation affect which showers form and where they fall as snow. Individual flakes fall through each snow band. [Sky](../src/sky.rs) · [Precipitation](../src/weather_effects.rs) |

## Where a flight starts

The launch survey scores headings and positions for land, relief, terrain ahead and the aircraft's clearance. Some coastal starts are retained. If the aircraft would spend too long leaving open water, the survey can look farther away for land. Eligible flights can start on a town approach when a suitable town is already prepared. The Blackbird starts at high altitude. [Aircraft and launch survey](../src/flight.rs)
