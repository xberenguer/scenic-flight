# Scenic Flight

A Rust flight simulator for the round Waveshare ESP32-S3-Touch-AMOLED-1.75C. Pick an aircraft and watch it fly through a new procedural world.

[Play in browser](https://xberenguer.github.io/scenic-flight/play.html) · [Interactive gallery](https://xberenguer.github.io/scenic-flight/) · [Install on Waveshare](https://xn--sg8hhb.ws/flasher/?project=scenic-flight)

<!-- BEGIN GALLERY -->

## Gameplay showcase

Select a preview to open the full-size clip or screenshot.

<table>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/mountain-pass.mp4"><img src="docs/assets/readme/mountain-pass.gif" width="220" alt="Mountain pass"></a><br>Mountain pass</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/coastal-soaring.mp4"><img src="docs/assets/readme/coastal-soaring.gif" width="220" alt="Eagle over the foothills"></a><br>Eagle over the foothills</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/city-approach.mp4"><img src="docs/assets/readme/city-approach.gif" width="220" alt="Racer approaching the skyline"></a><br>Racer approaching the skyline</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/high-cruise.mp4"><img src="docs/assets/readme/high-cruise.gif" width="220" alt="High-altitude cruise"></a><br>High-altitude cruise</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/river-valley.mp4"><img src="docs/assets/readme/river-valley.gif" width="220" alt="Ultralight over a river town"></a><br>Ultralight over a river town</td>
  </tr>
</table>

### Biomes

<table>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/glider-coast.mp4"><img src="docs/assets/readme/glider-coast.gif" width="220" alt="Glider on the coast"></a><br>Glider on the coast</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/woodland-stands.mp4"><img src="docs/assets/readme/woodland-stands.gif" width="220" alt="Bush Plane along the wooded slopes"></a><br>Bush Plane along the wooded slopes</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/bay-crossing.mp4"><img src="docs/assets/readme/bay-crossing.gif" width="220" alt="Flight over the bay"></a><br>Flight over the bay</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/current/cliffside.png"><img src="docs/assets/current/cliffside.png" width="220" alt="Cliffside flight"></a><br>Cliffside flight</td>
    <td align="center"><a href="docs/assets/current/bay-view.png"><img src="docs/assets/current/bay-view.png" width="220" alt="Bay and headlands"></a><br>Bay and headlands</td>
    <td align="center"><a href="docs/assets/current/ridge-view.png"><img src="docs/assets/current/ridge-view.png" width="220" alt="Ridge in daylight"></a><br>Ridge in daylight</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/current/coastal-sea-stack.png"><img src="docs/assets/current/coastal-sea-stack.png" width="220" alt="Sea stack"></a><br>Sea stack</td>
    <td align="center"><a href="docs/assets/current/coastal-skerries.png"><img src="docs/assets/current/coastal-skerries.png" width="220" alt="Skerries"></a><br>Skerries</td>
    <td align="center"><a href="docs/assets/current/coastal-stone-arch.png"><img src="docs/assets/current/coastal-stone-arch.png" width="220" alt="Sea arch"></a><br>Sea arch</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/current/oxbow-lake.png"><img src="docs/assets/current/oxbow-lake.png" width="220" alt="Oxbow lake"></a><br>Oxbow lake</td>
    <td align="center"><a href="docs/assets/current/alpine-tarn.png"><img src="docs/assets/current/alpine-tarn.png" width="220" alt="Alpine tarn"></a><br>Alpine tarn</td>
    <td align="center"><a href="docs/assets/current/columnar-rock.png"><img src="docs/assets/current/columnar-rock.png" width="220" alt="Columnar rock above an alpine scree slope"></a><br>Columnar rock above an alpine scree slope</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/current/salt-pan.png"><img src="docs/assets/current/salt-pan.png" width="220" alt="Salt pan"></a><br>Salt pan</td>
    <td align="center"><a href="docs/assets/current/inland-dunes.png"><img src="docs/assets/current/inland-dunes.png" width="220" alt="Inland dunes"></a><br>Inland dunes</td>
    <td align="center"><a href="docs/assets/current/badlands-ravine.png"><img src="docs/assets/current/badlands-ravine.png" width="220" alt="Badlands ravine"></a><br>Badlands ravine</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/current/alluvial-fan.png"><img src="docs/assets/current/alluvial-fan.png" width="220" alt="Desert alluvial fan"></a><br>Desert alluvial fan</td>
    <td align="center"><a href="docs/assets/current/cove-dune.png"><img src="docs/assets/current/cove-dune.png" width="220" alt="Cove dune"></a><br>Cove dune</td>
    <td align="center"><a href="docs/assets/current/riverside-town.png"><img src="docs/assets/current/riverside-town.png" width="220" alt="River through the foothills"></a><br>River through the foothills</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/coastal-details-flight.mp4"><img src="docs/assets/readme/coastal-details-flight.gif" width="220" alt="Bald eagle along the sandy coast"></a><br>Bald eagle along the sandy coast</td>
  </tr>
</table>

### Built World

<table>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/marina-piers.mp4"><img src="docs/assets/readme/marina-piers.gif" width="220" alt="Marina and sailboats"></a><br>Marina and sailboats</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/ferry-terminal.mp4"><img src="docs/assets/readme/ferry-terminal.gif" width="220" alt="Ferry terminal"></a><br>Ferry terminal</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/freight-containers.gif"><img src="docs/assets/readme/freight-containers.gif" width="220" alt="Container train below the Ultralight"></a><br>Container train below the Ultralight</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/current/freight-tankers.png"><img src="docs/assets/current/freight-tankers.png" width="220" alt="Food-oil tank cars"></a><br>Food-oil tank cars</td>
    <td align="center"><a href="docs/assets/current/freight-cold-storage.png"><img src="docs/assets/current/freight-cold-storage.png" width="220" alt="Frozen-food reefers"></a><br>Frozen-food reefers</td>
    <td align="center"><a href="docs/assets/current/freight-steel-coils.png"><img src="docs/assets/current/freight-steel-coils.png" width="220" alt="Steel coils and machinery freight"></a><br>Steel coils and machinery freight</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/current/freight-platforms.png"><img src="docs/assets/current/freight-platforms.png" width="220" alt="Rail loading apron and inland transfer shed"></a><br>Rail loading apron and inland transfer shed</td>
    <td align="center"><a href="docs/assets/current/freight-loading.png"><img src="docs/assets/current/freight-loading.png" width="220" alt="Gravel and container loading aprons"></a><br>Gravel and container loading aprons</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/grain-terminal.gif"><img src="docs/assets/readme/grain-terminal.gif" width="220" alt="Rural grain elevator and hopper train"></a><br>Rural grain elevator and hopper train</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/current/contour-fields.png"><img src="docs/assets/current/contour-fields.png" width="220" alt="Contour strips and grass buffers"></a><br>Contour strips and grass buffers</td>
    <td align="center"><a href="docs/assets/current/gravel-freight.png"><img src="docs/assets/current/gravel-freight.png" width="220" alt="Gravel pit and aggregate freight"></a><br>Gravel pit and aggregate freight</td>
    <td align="center"><a href="docs/assets/current/aggregate-service.png"><img src="docs/assets/current/aggregate-service.png" width="220" alt="Loaded gravel train and empty return"></a><br>Loaded gravel train and empty return</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/current/rail-earthworks.png"><img src="docs/assets/current/rail-earthworks.png" width="220" alt="Rail cutting and gravel embankment"></a><br>Rail cutting and gravel embankment</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/freight-harbor.mp4"><img src="docs/assets/readme/freight-harbor.gif" width="220" alt="Freight harbor"></a><br>Freight harbor</td>
    <td align="center"><a href="docs/assets/current/grass-airstrip.png"><img src="docs/assets/current/grass-airstrip.png" width="220" alt="Country airstrip"></a><br>Country airstrip</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/current/regional-airfield.png"><img src="docs/assets/current/regional-airfield.png" width="220" alt="Regional airfield"></a><br>Regional airfield</td>
    <td align="center"><a href="docs/assets/current/river-road-bridge.png"><img src="docs/assets/current/river-road-bridge.png" width="220" alt="River road bridge"></a><br>River road bridge</td>
    <td align="center"><a href="docs/assets/current/hillside-town.png"><img src="docs/assets/current/hillside-town.png" width="220" alt="Hillside town"></a><br>Hillside town</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/current/coastal-lighthouse.png"><img src="docs/assets/current/coastal-lighthouse.png" width="220" alt="Coastal lighthouse"></a><br>Coastal lighthouse</td>
    <td align="center"><a href="docs/assets/current/beach-umbrellas.png"><img src="docs/assets/current/beach-umbrellas.png" width="220" alt="Beach umbrellas and towels"></a><br>Beach umbrellas and towels</td>
    <td align="center"><a href="docs/assets/current/ferry-quayside.png"><img src="docs/assets/current/ferry-quayside.png" width="220" alt="Ferry quay and workshops"></a><br>Ferry quay and workshops</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/current/river-boardwalk.png"><img src="docs/assets/current/river-boardwalk.png" width="220" alt="River boardwalk and lookout"></a><br>River boardwalk and lookout</td>
    <td align="center"><a href="docs/assets/current/coastal-boardwalk.png"><img src="docs/assets/current/coastal-boardwalk.png" width="220" alt="Coastal boardwalk village"></a><br>Coastal boardwalk village</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/farm-details-flyover.mp4"><img src="docs/assets/readme/farm-details-flyover.gif" width="220" alt="Harvest equipment beside the fields"></a><br>Harvest equipment beside the fields</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/station-platform-preview.mp4"><img src="docs/assets/readme/station-platform-preview.gif" width="220" alt="Passenger station platform"></a><br>Passenger station platform</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/freight-apron-preview.mp4"><img src="docs/assets/readme/freight-apron-preview.gif" width="220" alt="Freight siding and loading apron"></a><br>Freight siding and loading apron</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/road-overlook.mp4"><img src="docs/assets/readme/road-overlook.gif" width="220" alt="Mountain road overlook"></a><br>Mountain road overlook</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/woodland-trailhead.mp4"><img src="docs/assets/readme/woodland-trailhead.gif" width="220" alt="Woodland trailhead"></a><br>Woodland trailhead</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/road-rest-stop.mp4"><img src="docs/assets/readme/road-rest-stop.gif" width="220" alt="Roadside rest stop"></a><br>Roadside rest stop</td>
  </tr>
</table>

### Encounters

<table>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/fighter-afterburners.mp4"><img src="docs/assets/readme/fighter-afterburners.gif" width="220" alt="Fighter orange afterburner"></a><br>Fighter orange afterburner</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/blackbird-reheat.mp4"><img src="docs/assets/readme/blackbird-reheat.gif" width="220" alt="Blackbird purple reheat"></a><br>Blackbird purple reheat</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/formation-pair.mp4"><img src="docs/assets/readme/formation-pair.gif" width="220" alt="Fighter line abreast"></a><br>Fighter line abreast</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/formation-four.mp4"><img src="docs/assets/readme/formation-four.gif" width="220" alt="Fighter wedge · 4 wingmates"></a><br>Fighter wedge · 4 wingmates</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/formation-echelon.mp4"><img src="docs/assets/readme/formation-echelon.gif" width="220" alt="Fighter echelon and peel-away"></a><br>Fighter echelon and peel-away</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/blackbird-refueling.mp4"><img src="docs/assets/readme/blackbird-refueling.gif" width="220" alt="Blackbird refueling"></a><br>Blackbird refueling</td>
  </tr>
</table>

### Aircraft

<table>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/glider-reflection.gif"><img src="docs/assets/readme/glider-reflection.gif" width="220" alt="Sun glint in a thermal turn"></a><br>Sun glint in a thermal turn</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/racer-bank.mp4"><img src="docs/assets/readme/racer-bank.gif" width="220" alt="Racer banks around a ridge"></a><br>Racer banks around a ridge</td>
  </tr>
</table>

### Sky & Weather

<table>
  <tr>
    <td align="center"><a href="docs/assets/current/dusk-sky.png"><img src="docs/assets/current/dusk-sky.png" width="220" alt="Moon over the city"></a><br>Moon over the city</td>
    <td align="center"><a href="docs/assets/current/shower-rainbow.png"><img src="docs/assets/current/shower-rainbow.png" width="220" alt="Rainbow in a coastal shower"></a><br>Rainbow in a coastal shower</td>
  </tr>
</table>

### Interface

<table>
  <tr>
    <td align="center"><a href="docs/assets/current/new-world-ui.png"><img src="docs/assets/current/new-world-ui.png" width="220" alt="New World hold progress"></a><br>New World hold progress</td>
    <td align="center"><a href="docs/assets/current/flight-info.png"><img src="docs/assets/current/flight-info.png" width="220" alt="Flight information overlay"></a><br>Flight information overlay</td>
  </tr>
</table>

## Artwork gallery

### Aircraft and picker screens

<table>
  <tr>
    <td align="center"><a href="docs/assets/turntable-0.gif"><img src="docs/assets/turntable-0.gif" width="168" alt="Bush Plane turntable"></a><br><a href="docs/assets/picker-0.png"><img src="docs/assets/picker-0.png" width="168" alt="Bush Plane picker screen"></a><br>Bush Plane</td>
    <td align="center"><a href="docs/assets/turntable-1.gif"><img src="docs/assets/turntable-1.gif" width="168" alt="Racer turntable"></a><br><a href="docs/assets/picker-1.png"><img src="docs/assets/picker-1.png" width="168" alt="Racer picker screen"></a><br>Racer</td>
    <td align="center"><a href="docs/assets/turntable-2.gif"><img src="docs/assets/turntable-2.gif" width="168" alt="Sport Jet turntable"></a><br><a href="docs/assets/picker-2.png"><img src="docs/assets/picker-2.png" width="168" alt="Sport Jet picker screen"></a><br>Sport Jet</td>
    <td align="center"><a href="docs/assets/turntable-3.gif"><img src="docs/assets/turntable-3.gif" width="168" alt="Ultralight turntable"></a><br><a href="docs/assets/picker-3.png"><img src="docs/assets/picker-3.png" width="168" alt="Ultralight picker screen"></a><br>Ultralight</td>
  </tr>
</table>

<table>
  <tr>
    <td align="center"><a href="docs/assets/turntable-4.gif"><img src="docs/assets/turntable-4.gif" width="168" alt="Glider turntable"></a><br><a href="docs/assets/picker-4.png"><img src="docs/assets/picker-4.png" width="168" alt="Glider picker screen"></a><br>Glider</td>
    <td align="center"><a href="docs/assets/turntable-5.gif"><img src="docs/assets/turntable-5.gif" width="168" alt="F/A-18 Fighter turntable"></a><br><a href="docs/assets/picker-5.png"><img src="docs/assets/picker-5.png" width="168" alt="F/A-18 Fighter picker screen"></a><br>F/A-18 Fighter</td>
    <td align="center"><a href="docs/assets/turntable-6.gif"><img src="docs/assets/turntable-6.gif" width="168" alt="SR-71 Blackbird turntable"></a><br><a href="docs/assets/picker-6.png"><img src="docs/assets/picker-6.png" width="168" alt="SR-71 Blackbird picker screen"></a><br>SR-71 Blackbird</td>
    <td align="center"><a href="docs/assets/turntable-7.gif"><img src="docs/assets/turntable-7.gif" width="168" alt="Bald Eagle turntable"></a><br><a href="docs/assets/picker-7.png"><img src="docs/assets/picker-7.png" width="168" alt="Bald Eagle picker screen"></a><br>Bald Eagle</td>
  </tr>
</table>

### Scenery sprites

#### Roadside models · 30 variants

<a href="docs/assets/artwork/roadside-models.png"><img src="docs/assets/artwork/roadside-models.png" width="780" alt="Roadside models"></a>

<a href="docs/assets/artwork/roadside-models.gif"><img src="docs/assets/artwork/roadside-models.gif" width="550" alt="Roadside models rotating"></a>

#### Harbor models · 30 variants

<a href="docs/assets/artwork/harbor-models.png"><img src="docs/assets/artwork/harbor-models.png" width="780" alt="Harbor models"></a>

<a href="docs/assets/artwork/harbor-models.gif"><img src="docs/assets/artwork/harbor-models.gif" width="550" alt="Harbor models rotating"></a>

#### Railway fixtures · 25 variants

<a href="docs/assets/artwork/rail-stations.png"><img src="docs/assets/artwork/rail-stations.png" width="780" alt="Railway fixtures"></a>

<a href="docs/assets/artwork/rail-stations.gif"><img src="docs/assets/artwork/rail-stations.gif" width="550" alt="Railway fixtures rotating"></a>

#### Farm details · 25 variants

<a href="docs/assets/artwork/farm-details.png"><img src="docs/assets/artwork/farm-details.png" width="780" alt="Farm details"></a>

<a href="docs/assets/artwork/farm-details.gif"><img src="docs/assets/artwork/farm-details.gif" width="550" alt="Farm details rotating"></a>

#### Coastal details · 25 variants

<a href="docs/assets/artwork/coastal-details.png"><img src="docs/assets/artwork/coastal-details.png" width="780" alt="Coastal details"></a>

<a href="docs/assets/artwork/coastal-details.gif"><img src="docs/assets/artwork/coastal-details.gif" width="550" alt="Coastal details rotating"></a>

#### Skyline landmarks · 25 variants

<a href="docs/assets/artwork/skyline-landmarks.png"><img src="docs/assets/artwork/skyline-landmarks.png" width="780" alt="Skyline landmarks"></a>

<a href="docs/assets/artwork/skyline-landmarks.gif"><img src="docs/assets/artwork/skyline-landmarks.gif" width="550" alt="Skyline landmarks rotating"></a>

#### Neighborhood buildings · 27 variants

<a href="docs/assets/artwork/neighborhood-buildings.png"><img src="docs/assets/artwork/neighborhood-buildings.png" width="780" alt="Neighborhood buildings"></a>

#### Habitat details · 30 variants

<a href="docs/assets/artwork/habitat-details.png"><img src="docs/assets/artwork/habitat-details.png" width="780" alt="Habitat details"></a>

#### City towers · 250 variants

<a href="docs/assets/artwork/towers.png"><img src="docs/assets/artwork/towers.png" width="780" alt="City towers"></a>

#### Trees · 256 variants

<a href="docs/assets/artwork/trees.png"><img src="docs/assets/artwork/trees.png" width="780" alt="Trees"></a>

#### Rocks · 256 variants

<a href="docs/assets/artwork/rocks.png"><img src="docs/assets/artwork/rocks.png" width="780" alt="Rocks"></a>

#### Bushes · 256 variants

<a href="docs/assets/artwork/bushes.png"><img src="docs/assets/artwork/bushes.png" width="780" alt="Bushes"></a>

#### Cacti · 256 variants

<a href="docs/assets/artwork/cacti.png"><img src="docs/assets/artwork/cacti.png" width="780" alt="Cacti"></a>

#### Freight wagons · 250 variants

<a href="docs/assets/artwork/wagons.png"><img src="docs/assets/artwork/wagons.png" width="780" alt="Freight wagons"></a>

#### Vessels · 250 variants

<a href="docs/assets/artwork/vessels.png"><img src="docs/assets/artwork/vessels.png" width="780" alt="Vessels"></a>

#### Wetland plants · 256 variants

<a href="docs/assets/artwork/wetlands.png"><img src="docs/assets/artwork/wetlands.png" width="780" alt="Wetland plants"></a>

#### Lighthouse forms · 3 variants

<a href="docs/assets/artwork/lighthouses.png"><img src="docs/assets/artwork/lighthouses.png" width="780" alt="Lighthouse forms"></a>

#### Locomotives · 6 variants

<a href="docs/assets/artwork/locomotives.png"><img src="docs/assets/artwork/locomotives.png" width="780" alt="Locomotives"></a>

#### Waterfront landmarks · 4 variants

<a href="docs/assets/artwork/waterfront-landmarks.png"><img src="docs/assets/artwork/waterfront-landmarks.png" width="780" alt="Waterfront landmarks"></a>

<!-- END GALLERY -->

## Controls

| When | Small button | Large button |
| --- | --- | --- |
| Aircraft picker | Browse the eight aircraft | Take off |
| In flight | Tap for flight info. Hold until the progress bar fills to return to the picker and start a new world. | — |

The first picker launches a random aircraft after ten seconds if no buttons are pressed.

## Aircraft

| Aircraft | Flight style |
| --- | --- |
| Bush Plane | Low, deliberate flying and early climbs around ridges |
| Racer | Tighter turns over lowland terrain |
| Sport Jet | Smooth jet handling and occasional coastal passes |
| Ultralight | Slow sightseeing and occasional train follows |
| Glider | Thermal circles, ridge lift and downdrafts |
| F/A-18 Fighter | Formation flybys and short orange reheat bursts |
| SR-71 Blackbird | High cruise, purple reheat and a rare refueling rendezvous |
| Bald Eagle | Low soaring, measured wingbeats and occasional train follows |

## Install and build

Generate the scenery artwork directly from the game’s geometry with `./tools/export-artwork.sh`. The freight gallery shows 250 cars across 10 equipment families. Each car has a named load or return assignment, an explanation of its equipment, and a reference to the real railcars it draws on.

[Open the firmware flasher](https://xn--sg8hhb.ws/flasher/?project=scenic-flight) for compatible round Waveshare units. See the [latest GitHub release](https://github.com/xberenguer/scenic-flight/releases/latest) for the firmware and checksum.

Test the Rust simulation and render a seeded desktop frame.

```sh
cargo test --locked --features terrain-fork,small-raster-ray-budget,ground-cover
cargo run --locked --release --features ground-cover --bin flight-preview -- render 11 6 10 0.46 240 preview.ppm detailed
```

The Waveshare app is built with `board/build-app.sh` using an ESP32-S3 Xtensa Rust toolchain. See the [board build notes](board/BUILD-NOTES.md) for the toolchain and app partition details.

Original inspiration: [Voxel Flight by lewtheegg](https://github.com/lewtheegg/voxel-flight).
