# Scenic Flight

A Rust flight simulator for the round Waveshare ESP32-S3-Touch-AMOLED-1.75C. Pick an aircraft and watch it fly through a new procedural world.

[Play in browser](https://xberenguer.github.io/scenic-flight/play.html) · [Interactive gallery](https://xberenguer.github.io/scenic-flight/) · [Install on Waveshare](https://xn--sg8hhb.ws/flasher/?project=scenic-flight)

<!-- BEGIN GALLERY -->

## Gameplay showcase

Select a preview to open the full-size clip or screenshot.

<table>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/airfield-visitor-stands.mp4"><img src="docs/assets/readme/airfield-visitor-stands.gif" width="220" alt="Racer approaching a regional airfield"></a><br><a href="docs/assets/current/airfield-visitor-stands.png">Still</a><br>Racer approaching a regional airfield</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/sedge-strand-tarn.mp4"><img src="docs/assets/readme/sedge-strand-tarn.gif" width="220" alt="Bald Eagle over a sedge-lined tarn"></a><br><a href="docs/assets/current/sedge-strand-tarn.png">Still</a><br>Bald Eagle over a sedge-lined tarn</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/lantern-causeway-skyline.mp4"><img src="docs/assets/readme/lantern-causeway-skyline.gif" width="220" alt="Racer approaching Lantern Causeway"></a><br><a href="docs/assets/current/lantern-causeway-skyline.png">Still</a><br>Racer approaching Lantern Causeway</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/high-cruise.mp4"><img src="docs/assets/readme/high-cruise.gif" width="220" alt="High-altitude cruise"></a><br><a href="docs/assets/current/high-cruise.png">Still</a><br>High-altitude cruise</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/quay-customs-annex.mp4"><img src="docs/assets/readme/quay-customs-annex.gif" width="220" alt="Ultralight past the harbor customs annex"></a><br><a href="docs/assets/current/quay-customs-annex.png">Still</a><br>Ultralight past the harbor customs annex</td>
  </tr>
</table>

### Biomes

<table>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/twin-cove-crests.mp4"><img src="docs/assets/readme/twin-cove-crests.gif" width="220" alt="Bald Eagle over twin dune crests"></a><br><a href="docs/assets/current/twin-cove-crests.png">Still</a><br>Bald Eagle over twin dune crests</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/elbow-cove-dune.mp4"><img src="docs/assets/readme/elbow-cove-dune.gif" width="220" alt="Glider following an elbow-shaped dune"></a><br><a href="docs/assets/current/elbow-cove-dune.png">Still</a><br>Glider following an elbow-shaped dune</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/notched-cove-dune.mp4"><img src="docs/assets/readme/notched-cove-dune.gif" width="220" alt="Ultralight along a notched cove dune"></a><br><a href="docs/assets/current/notched-cove-dune.png">Still</a><br>Ultralight along a notched cove dune</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/twin-rock-buttresses.mp4"><img src="docs/assets/readme/twin-rock-buttresses.gif" width="220" alt="Bald Eagle above twin rock buttresses"></a><br><a href="docs/assets/current/twin-rock-buttresses.png">Still</a><br>Bald Eagle above twin rock buttresses</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/descending-columnar-wall.mp4"><img src="docs/assets/readme/descending-columnar-wall.gif" width="220" alt="Glider over a descending rock face"></a><br><a href="docs/assets/current/descending-columnar-wall.png">Still</a><br>Glider over a descending rock face</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/fallen-columnar-blocks.mp4"><img src="docs/assets/readme/fallen-columnar-blocks.gif" width="220" alt="Bush Plane past a broad columnar outcrop"></a><br><a href="docs/assets/current/fallen-columnar-blocks.png">Still</a><br>Bush Plane past a broad columnar outcrop</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/late-split-alluvial-fan.mp4"><img src="docs/assets/readme/late-split-alluvial-fan.gif" width="220" alt="Bald Eagle over a three-way desert wash"></a><br><a href="docs/assets/current/late-split-alluvial-fan.png">Still</a><br>Bald Eagle over a three-way desert wash</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/narrow-alluvial-fan.mp4"><img src="docs/assets/readme/narrow-alluvial-fan.gif" width="220" alt="Glider above a narrow alluvial fan"></a><br><a href="docs/assets/current/narrow-alluvial-fan.png">Still</a><br>Glider above a narrow alluvial fan</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/wide-gravel-fan.mp4"><img src="docs/assets/readme/wide-gravel-fan.gif" width="220" alt="Ultralight over a wide gravel apron"></a><br><a href="docs/assets/current/wide-gravel-fan.png">Still</a><br>Ultralight over a wide gravel apron</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/broad-glacial-debris-apron.mp4"><img src="docs/assets/readme/broad-glacial-debris-apron.gif" width="220" alt="Ultralight past a broad moraine apron"></a><br><a href="docs/assets/current/broad-glacial-debris-apron.png">Still</a><br>Ultralight past a broad moraine apron</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/sinuous-glacial-moraine.mp4"><img src="docs/assets/readme/sinuous-glacial-moraine.gif" width="220" alt="Bald Eagle over a sinuous moraine"></a><br><a href="docs/assets/current/sinuous-glacial-moraine.png">Still</a><br>Bald Eagle over a sinuous moraine</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/twin-recessional-ridges.mp4"><img src="docs/assets/readme/twin-recessional-ridges.gif" width="220" alt="Glider over twin moraine ridges"></a><br><a href="docs/assets/current/twin-recessional-ridges.png">Still</a><br>Glider over twin moraine ridges</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/gravel-shoulder-tarn.mp4"><img src="docs/assets/readme/gravel-shoulder-tarn.gif" width="220" alt="Ultralight over a tarn with gravel shoulders"></a><br><a href="docs/assets/current/gravel-shoulder-tarn.png">Still</a><br>Ultralight over a tarn with gravel shoulders</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/sand-fan-tarn.mp4"><img src="docs/assets/readme/sand-fan-tarn.gif" width="220" alt="Glider past a sand-fan tarn"></a><br><a href="docs/assets/current/sand-fan-tarn.png">Still</a><br>Glider past a sand-fan tarn</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/single-streak-moraine.mp4"><img src="docs/assets/readme/single-streak-moraine.gif" width="220" alt="Ultralight along a glacier moraine"></a><br><a href="docs/assets/current/single-streak-moraine.png">Still</a><br>Ultralight along a glacier moraine</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/bowed-blue-icefall.mp4"><img src="docs/assets/readme/bowed-blue-icefall.gif" width="220" alt="Glider over a bowed blue icefall"></a><br><a href="docs/assets/current/bowed-blue-icefall.png">Still</a><br>Glider over a bowed blue icefall</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/tapered-snow-apron.mp4"><img src="docs/assets/readme/tapered-snow-apron.gif" width="220" alt="Bald Eagle above a tapered snow apron"></a><br><a href="docs/assets/current/tapered-snow-apron.png">Still</a><br>Bald Eagle above a tapered snow apron</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/stone-crown-island.mp4"><img src="docs/assets/readme/stone-crown-island.gif" width="220" alt="Ultralight over a two-crown shingle island"></a><br><a href="docs/assets/current/stone-crown-island.png">Still</a><br>Ultralight over a two-crown shingle island</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/meadow-knoll-island.mp4"><img src="docs/assets/readme/meadow-knoll-island.gif" width="220" alt="Glider past twin meadow knolls"></a><br><a href="docs/assets/current/meadow-knoll-island.png">Still</a><br>Glider past twin meadow knolls</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/crescent-river-island.mp4"><img src="docs/assets/readme/crescent-river-island.gif" width="220" alt="Bald Eagle over a sandy crescent island"></a><br><a href="docs/assets/current/crescent-river-island.png">Still</a><br>Bald Eagle over a sandy crescent island</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/thin-stone-bridge.mp4"><img src="docs/assets/readme/thin-stone-bridge.gif" width="220" alt="Glider by a thin sea arch"></a><br><a href="docs/assets/current/thin-stone-bridge.png">Still</a><br>Glider by a thin sea arch</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/descending-skerries.mp4"><img src="docs/assets/readme/descending-skerries.gif" width="220" alt="Ultralight over descending skerries"></a><br><a href="docs/assets/current/descending-skerries.png">Still</a><br>Ultralight over descending skerries</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/needle-sea-stack.mp4"><img src="docs/assets/readme/needle-sea-stack.gif" width="220" alt="Bald Eagle past a needle sea stack"></a><br><a href="docs/assets/current/needle-sea-stack.png">Still</a><br>Bald Eagle past a needle sea stack</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/mesa-three-towers.mp4"><img src="docs/assets/readme/mesa-three-towers.gif" width="220" alt="Glider approaching three sandstone towers"></a><br><a href="docs/assets/current/mesa-three-towers.png">Still</a><br>Glider approaching three sandstone towers</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/mesa-ravine.mp4"><img src="docs/assets/readme/mesa-ravine.gif" width="220" alt="Bald Eagle along a sandstone ravine"></a><br><a href="docs/assets/current/mesa-ravine.png">Still</a><br>Bald Eagle along a sandstone ravine</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/mesa-double-ledge.mp4"><img src="docs/assets/readme/mesa-double-ledge.gif" width="220" alt="Ultralight over terraced sandstone mesas"></a><br><a href="docs/assets/current/mesa-double-ledge.png">Still</a><br>Ultralight over terraced sandstone mesas</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/flowering-reed-marsh.mp4"><img src="docs/assets/readme/flowering-reed-marsh.gif" width="220" alt="Ultralight along a reed marsh"></a><br><a href="docs/assets/current/flowering-reed-marsh.png">Still</a><br>Ultralight along a reed marsh</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/floodplain-bend.mp4"><img src="docs/assets/readme/floodplain-bend.gif" width="220" alt="Ultralight along a floodplain bend"></a><br><a href="docs/assets/current/floodplain-bend.png">Still</a><br>Ultralight along a floodplain bend</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/eagle-wet-meadow.mp4"><img src="docs/assets/readme/eagle-wet-meadow.gif" width="220" alt="Bald Eagle over a wet meadow"></a><br><a href="docs/assets/current/eagle-wet-meadow.png">Still</a><br>Bald Eagle over a wet meadow</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/glacier-ice-pass.mp4"><img src="docs/assets/readme/glacier-ice-pass.gif" width="220" alt="Glider above fractured ice"></a><br><a href="docs/assets/current/glacier-ice-pass.png">Still</a><br>Glider above fractured ice</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/alpine-ridgetop.mp4"><img src="docs/assets/readme/alpine-ridgetop.gif" width="220" alt="Bush Plane over a rocky alpine ridge"></a><br><a href="docs/assets/current/alpine-ridgetop.png">Still</a><br>Bush Plane over a rocky alpine ridge</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/alpine-rock-and-meadow.mp4"><img src="docs/assets/readme/alpine-rock-and-meadow.gif" width="220" alt="Ultralight over an alpine meadow"></a><br><a href="docs/assets/current/alpine-rock-and-meadow.png">Still</a><br>Ultralight over an alpine meadow</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/glider-upper-shore.mp4"><img src="docs/assets/readme/glider-upper-shore.gif" width="220" alt="Glider over coastal grasses"></a><br><a href="docs/assets/current/glider-upper-shore.png">Still</a><br>Glider over coastal grasses</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/eagle-headland.mp4"><img src="docs/assets/readme/eagle-headland.gif" width="220" alt="Bald Eagle along a sandy headland"></a><br><a href="docs/assets/current/eagle-headland.png">Still</a><br>Bald Eagle along a sandy headland</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/grassland-coastal-slopes.mp4"><img src="docs/assets/readme/grassland-coastal-slopes.gif" width="220" alt="Ultralight along coastal grassland"></a><br><a href="docs/assets/current/grassland-coastal-slopes.png">Still</a><br>Ultralight along coastal grassland</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/grassland-foothills.mp4"><img src="docs/assets/readme/grassland-foothills.gif" width="220" alt="Ultralight over grassy foothills"></a><br><a href="docs/assets/current/grassland-foothills.png">Still</a><br>Ultralight over grassy foothills</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/coastal-details-flight.mp4"><img src="docs/assets/readme/coastal-details-flight.gif" width="220" alt="Bald eagle along the sandy coast"></a><br><a href="docs/assets/current/coastal-details-flight.png">Still</a><br>Bald eagle along the sandy coast</td>
    <td align="center"><a href="docs/assets/current/riverside-town.png"><img src="docs/assets/current/riverside-town.png" width="220" alt="River through the foothills"></a><br>River through the foothills</td>
    <td align="center"><a href="docs/assets/current/cove-dune.png"><img src="docs/assets/current/cove-dune.png" width="220" alt="Cove dune"></a><br>Cove dune</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/current/alluvial-fan.png"><img src="docs/assets/current/alluvial-fan.png" width="220" alt="Desert alluvial fan"></a><br>Desert alluvial fan</td>
    <td align="center"><a href="docs/assets/current/badlands-ravine.png"><img src="docs/assets/current/badlands-ravine.png" width="220" alt="Badlands ravine"></a><br>Badlands ravine</td>
    <td align="center"><a href="docs/assets/current/inland-dunes.png"><img src="docs/assets/current/inland-dunes.png" width="220" alt="Inland dunes"></a><br>Inland dunes</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/current/salt-pan.png"><img src="docs/assets/current/salt-pan.png" width="220" alt="Salt pan"></a><br>Salt pan</td>
    <td align="center"><a href="docs/assets/current/columnar-rock.png"><img src="docs/assets/current/columnar-rock.png" width="220" alt="Columnar rock above an alpine scree slope"></a><br>Columnar rock above an alpine scree slope</td>
    <td align="center"><a href="docs/assets/current/alpine-tarn.png"><img src="docs/assets/current/alpine-tarn.png" width="220" alt="Alpine tarn"></a><br>Alpine tarn</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/current/oxbow-lake.png"><img src="docs/assets/current/oxbow-lake.png" width="220" alt="Oxbow lake"></a><br>Oxbow lake</td>
    <td align="center"><a href="docs/assets/current/coastal-stone-arch.png"><img src="docs/assets/current/coastal-stone-arch.png" width="220" alt="Sea arch"></a><br>Sea arch</td>
    <td align="center"><a href="docs/assets/current/coastal-skerries.png"><img src="docs/assets/current/coastal-skerries.png" width="220" alt="Skerries"></a><br>Skerries</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/current/coastal-sea-stack.png"><img src="docs/assets/current/coastal-sea-stack.png" width="220" alt="Sea stack"></a><br>Sea stack</td>
    <td align="center"><a href="docs/assets/current/ridge-view.png"><img src="docs/assets/current/ridge-view.png" width="220" alt="Ridge in daylight"></a><br>Ridge in daylight</td>
    <td align="center"><a href="docs/assets/current/bay-view.png"><img src="docs/assets/current/bay-view.png" width="220" alt="Bay and headlands"></a><br>Bay and headlands</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/current/cliffside.png"><img src="docs/assets/current/cliffside.png" width="220" alt="Cliffside flight"></a><br>Cliffside flight</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/bay-crossing.mp4"><img src="docs/assets/readme/bay-crossing.gif" width="220" alt="Flight over the bay"></a><br><a href="docs/assets/current/bay-crossing.png">Still</a><br>Flight over the bay</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/woodland-stands.mp4"><img src="docs/assets/readme/woodland-stands.gif" width="220" alt="Bush Plane along the wooded slopes"></a><br><a href="docs/assets/current/woodland-stands.png">Still</a><br>Bush Plane along the wooded slopes</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/glider-coast.mp4"><img src="docs/assets/readme/glider-coast.gif" width="220" alt="Glider on the coast"></a><br><a href="docs/assets/current/glider-coast.png">Still</a><br>Glider on the coast</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/river-valley.mp4"><img src="docs/assets/readme/river-valley.gif" width="220" alt="Ultralight over a river town"></a><br><a href="docs/assets/current/river-valley.png">Still</a><br>Ultralight over a river town</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/coastal-soaring.mp4"><img src="docs/assets/readme/coastal-soaring.gif" width="220" alt="Eagle over the foothills"></a><br><a href="docs/assets/current/coastal-soaring.png">Still</a><br>Eagle over the foothills</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/mountain-pass.mp4"><img src="docs/assets/readme/mountain-pass.gif" width="220" alt="Mountain pass"></a><br><a href="docs/assets/current/mountain-pass.png">Still</a><br>Mountain pass</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/snowfield-glider.mp4"><img src="docs/assets/readme/snowfield-glider.gif" width="220" alt="Glider over a snowfield"></a><br><a href="docs/assets/current/snowfield-glider.png">Still</a><br>Glider over a snowfield</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/ultralight-ridge.mp4"><img src="docs/assets/readme/ultralight-ridge.gif" width="220" alt="Ultralight over a rocky ridge"></a><br><a href="docs/assets/current/ultralight-ridge.png">Still</a><br>Ultralight over a rocky ridge</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/wooded-watershed.mp4"><img src="docs/assets/readme/wooded-watershed.gif" width="220" alt="Eagle over a wooded watershed"></a><br><a href="docs/assets/current/wooded-watershed.png">Still</a><br>Eagle over a wooded watershed</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/reedbed-meander.mp4"><img src="docs/assets/readme/reedbed-meander.gif" width="220" alt="Eagle over a reed-lined bend"></a><br><a href="docs/assets/current/reedbed-meander.png">Still</a><br>Eagle over a reed-lined bend</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/eagle-forest-edge.mp4"><img src="docs/assets/readme/eagle-forest-edge.gif" width="220" alt="Eagle along the forest edge"></a><br><a href="docs/assets/current/eagle-forest-edge.png">Still</a><br>Eagle along the forest edge</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/wooded-valley-pass.mp4"><img src="docs/assets/readme/wooded-valley-pass.gif" width="220" alt="Ultralight through a wooded valley"></a><br><a href="docs/assets/current/wooded-valley-pass.png">Still</a><br>Ultralight through a wooded valley</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/dryland-badlands-pass.mp4"><img src="docs/assets/readme/dryland-badlands-pass.gif" width="220" alt="Ultralight through the badlands"></a><br><a href="docs/assets/current/dryland-badlands-pass.png">Still</a><br>Ultralight through the badlands</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/dryland-basin-pass.mp4"><img src="docs/assets/readme/dryland-basin-pass.gif" width="220" alt="Ultralight over desert flats"></a><br><a href="docs/assets/current/dryland-basin-pass.png">Still</a><br>Ultralight over desert flats</td>
  </tr>
</table>

### Built World

<table>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/airfield-country-service-pocket.mp4"><img src="docs/assets/readme/airfield-country-service-pocket.gif" width="220" alt="Bush Plane circling a country strip"></a><br><a href="docs/assets/current/airfield-country-service-pocket.png">Still</a><br>Bush Plane circling a country strip</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/airfield-light-aircraft-bays.mp4"><img src="docs/assets/readme/airfield-light-aircraft-bays.gif" width="220" alt="Glider over light-aircraft bays"></a><br><a href="docs/assets/current/airfield-light-aircraft-bays.png">Still</a><br>Glider over light-aircraft bays</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/narrow-inlet-quarry.mp4"><img src="docs/assets/readme/narrow-inlet-quarry.gif" width="220" alt="Bush Plane above a narrow quarry entrance"></a><br><a href="docs/assets/current/narrow-inlet-quarry.png">Still</a><br>Bush Plane above a narrow quarry entrance</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/west-sweep-quarry.mp4"><img src="docs/assets/readme/west-sweep-quarry.gif" width="220" alt="Glider past a sweeping cut and haul road"></a><br><a href="docs/assets/current/west-sweep-quarry.png">Still</a><br>Glider past a sweeping cut and haul road</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/half-bowl-quarry.mp4"><img src="docs/assets/readme/half-bowl-quarry.gif" width="220" alt="Ultralight above a half-bowl quarry"></a><br><a href="docs/assets/current/half-bowl-quarry.png">Still</a><br>Ultralight above a half-bowl quarry</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/quay-boatyard-paint-hall.mp4"><img src="docs/assets/readme/quay-boatyard-paint-hall.gif" width="220" alt="Bush Plane along the boatyard promenade"></a><br><a href="docs/assets/current/quay-boatyard-paint-hall.png">Still</a><br>Bush Plane along the boatyard promenade</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/quay-twin-fish-sheds.mp4"><img src="docs/assets/readme/quay-twin-fish-sheds.gif" width="220" alt="Glider approaching twin fish sheds"></a><br><a href="docs/assets/current/quay-twin-fish-sheds.png">Still</a><br>Glider approaching twin fish sheds</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/market-gardens-below-town.mp4"><img src="docs/assets/readme/market-gardens-below-town.gif" width="220" alt="Ultralight over market gardens below town"></a><br><a href="docs/assets/current/market-gardens-below-town.png">Still</a><br>Ultralight over market gardens below town</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/coastal-stubble-fields.mp4"><img src="docs/assets/readme/coastal-stubble-fields.gif" width="220" alt="Bush Plane over coastal stubble fields"></a><br><a href="docs/assets/current/coastal-stubble-fields.png">Still</a><br>Bush Plane over coastal stubble fields</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/grassy-field-setbacks.mp4"><img src="docs/assets/readme/grassy-field-setbacks.gif" width="220" alt="Glider over fields with grassy setbacks"></a><br><a href="docs/assets/current/grassy-field-setbacks.png">Still</a><br>Glider over fields with grassy setbacks</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/chamber-on-columns-skyline.mp4"><img src="docs/assets/readme/chamber-on-columns-skyline.gif" width="220" alt="Bush Plane approaching Chamber on Columns"></a><br><a href="docs/assets/current/chamber-on-columns-skyline.png">Still</a><br>Bush Plane approaching Chamber on Columns</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/library-of-steps-skyline.mp4"><img src="docs/assets/readme/library-of-steps-skyline.gif" width="220" alt="Ultralight approaching the Library of Steps"></a><br><a href="docs/assets/current/library-of-steps-skyline.png">Still</a><br>Ultralight approaching the Library of Steps</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/glass-hourglass-skyline.mp4"><img src="docs/assets/readme/glass-hourglass-skyline.gif" width="220" alt="Glider approaching the Glass Hourglass"></a><br><a href="docs/assets/current/glass-hourglass-skyline.png">Still</a><br>Glider approaching the Glass Hourglass</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/solar-three-islands.mp4"><img src="docs/assets/readme/solar-three-islands.gif" width="220" alt="Bush Plane over three banks of solar panels"></a><br><a href="docs/assets/current/solar-three-islands.png">Still</a><br>Bush Plane over three banks of solar panels</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/solar-striped-field.mp4"><img src="docs/assets/readme/solar-striped-field.gif" width="220" alt="Bald Eagle over a striped solar field"></a><br><a href="docs/assets/current/solar-striped-field.png">Still</a><br>Bald Eagle over a striped solar field</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/solar-diamond-field.mp4"><img src="docs/assets/readme/solar-diamond-field.gif" width="220" alt="Glider over a diamond-shaped solar field"></a><br><a href="docs/assets/current/solar-diamond-field.png">Still</a><br>Glider over a diamond-shaped solar field</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/solar-service-court.mp4"><img src="docs/assets/readme/solar-service-court.gif" width="220" alt="Ultralight over a solar service court"></a><br><a href="docs/assets/current/solar-service-court.png">Still</a><br>Ultralight over a solar service court</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/splayed-portal-water-tower.mp4"><img src="docs/assets/readme/splayed-portal-water-tower.gif" width="220" alt="Ultralight past a splayed portal tank"></a><br><a href="docs/assets/current/splayed-portal-water-tower.png">Still</a><br>Ultralight past a splayed portal tank</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/copper-dome-water-tower.mp4"><img src="docs/assets/readme/copper-dome-water-tower.gif" width="220" alt="Glider beside a masonry water tower"></a><br><a href="docs/assets/current/copper-dome-water-tower.png">Still</a><br>Glider beside a masonry water tower</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/steel-frame-saucer-tower.mp4"><img src="docs/assets/readme/steel-frame-saucer-tower.gif" width="220" alt="Bald Eagle past a steel-frame water tower"></a><br><a href="docs/assets/current/steel-frame-saucer-tower.png">Still</a><br>Bald Eagle past a steel-frame water tower</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/green-pratt-crossing.mp4"><img src="docs/assets/readme/green-pratt-crossing.gif" width="220" alt="Ultralight alongside a green Pratt truss"></a><br><a href="docs/assets/current/green-pratt-crossing.png">Still</a><br>Ultralight alongside a green Pratt truss</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/cream-tied-arch.mp4"><img src="docs/assets/readme/cream-tied-arch.gif" width="220" alt="Glider past a tied-arch bridge"></a><br><a href="docs/assets/current/cream-tied-arch.png">Still</a><br>Glider past a tied-arch bridge</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/timber-crossing.mp4"><img src="docs/assets/readme/timber-crossing.gif" width="220" alt="Bald Eagle over a timber-braced crossing"></a><br><a href="docs/assets/current/timber-crossing.png">Still</a><br>Bald Eagle over a timber-braced crossing</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/paired-mountain-gondolas.mp4"><img src="docs/assets/readme/paired-mountain-gondolas.gif" width="220" alt="Glider alongside mountain gondolas"></a><br><a href="docs/assets/current/paired-mountain-gondolas.png">Still</a><br>Glider alongside mountain gondolas</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/stone-cableway-landing.mp4"><img src="docs/assets/readme/stone-cableway-landing.gif" width="220" alt="Ultralight at a stone cableway landing"></a><br><a href="docs/assets/current/stone-cableway-landing.png">Still</a><br>Ultralight at a stone cableway landing</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/timber-cableway-terminal.mp4"><img src="docs/assets/readme/timber-cableway-terminal.gif" width="220" alt="Bald Eagle past a timber cableway terminal"></a><br><a href="docs/assets/current/timber-cableway-terminal.png">Still</a><br>Bald Eagle past a timber cableway terminal</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/open-frame-lighthouse-night.mp4"><img src="docs/assets/readme/open-frame-lighthouse-night.gif" width="220" alt="Bush Plane past an open-frame lighthouse at night"></a><br><a href="docs/assets/current/open-frame-lighthouse-night.png">Still</a><br>Bush Plane past an open-frame lighthouse at night</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/green-cupola-lighthouse.mp4"><img src="docs/assets/readme/green-cupola-lighthouse.gif" width="220" alt="Ultralight by a green-cupola keeper tower"></a><br><a href="docs/assets/current/green-cupola-lighthouse.png">Still</a><br>Ultralight by a green-cupola keeper tower</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/red-banded-lighthouse.mp4"><img src="docs/assets/readme/red-banded-lighthouse.gif" width="220" alt="Bald Eagle past a red-banded lighthouse"></a><br><a href="docs/assets/current/red-banded-lighthouse.png">Still</a><br>Bald Eagle past a red-banded lighthouse</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/coach-rest-hall.mp4"><img src="docs/assets/readme/coach-rest-hall.gif" width="220" alt="Glider over a coach rest hall"></a><br><a href="docs/assets/current/coach-rest-hall.png">Still</a><br>Glider over a coach rest hall</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/stone-roadside-overlook.mp4"><img src="docs/assets/readme/stone-roadside-overlook.gif" width="220" alt="Ultralight past a stone overlook"></a><br><a href="docs/assets/current/stone-roadside-overlook.png">Still</a><br>Ultralight past a stone overlook</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/forest-camp-check-in.mp4"><img src="docs/assets/readme/forest-camp-check-in.gif" width="220" alt="Eagle over a forest check-in kiosk"></a><br><a href="docs/assets/current/forest-camp-check-in.png">Still</a><br>Eagle over a forest check-in kiosk</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/twin-stack-boiler-house.mp4"><img src="docs/assets/readme/twin-stack-boiler-house.gif" width="220" alt="Ultralight by a twin-stack boiler house"></a><br><a href="docs/assets/current/twin-stack-boiler-house.png">Still</a><br>Ultralight by a twin-stack boiler house</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/rail-workshops-flyover.mp4"><img src="docs/assets/readme/rail-workshops-flyover.gif" width="220" alt="Ultralight over railway workshops"></a><br><a href="docs/assets/current/rail-workshops-flyover.png">Still</a><br>Ultralight over railway workshops</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/tram-carhouse-flyover.mp4"><img src="docs/assets/readme/tram-carhouse-flyover.gif" width="220" alt="Ultralight above a tram carhouse"></a><br><a href="docs/assets/current/tram-carhouse-flyover.png">Still</a><br>Ultralight above a tram carhouse</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/covered-market-court.mp4"><img src="docs/assets/readme/covered-market-court.gif" width="220" alt="Bald Eagle over a covered market court"></a><br><a href="docs/assets/current/covered-market-court.png">Still</a><br>Bald Eagle over a covered market court</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/coastal-grain-store.mp4"><img src="docs/assets/readme/coastal-grain-store.gif" width="220" alt="Ultralight passes a cooperative grain store"></a><br><a href="docs/assets/current/coastal-grain-store.png">Still</a><br>Ultralight passes a cooperative grain store</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/town-clock-tower.mp4"><img src="docs/assets/readme/town-clock-tower.gif" width="220" alt="Ultralight over a town hall clock tower"></a><br><a href="docs/assets/current/town-clock-tower.png">Still</a><br>Ultralight over a town hall clock tower</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/twin-summit-domes.mp4"><img src="docs/assets/readme/twin-summit-domes.gif" width="220" alt="Glider approaches twin summit observatories"></a><br><a href="docs/assets/current/twin-summit-domes.png">Still</a><br>Glider approaches twin summit observatories</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/cantilever-ridge-lookout.mp4"><img src="docs/assets/readme/cantilever-ridge-lookout.gif" width="220" alt="Bush plane passes a cantilevered ridge lookout"></a><br><a href="docs/assets/current/cantilever-ridge-lookout.png">Still</a><br>Bush plane passes a cantilevered ridge lookout</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/split-roof-summit-refuge.mp4"><img src="docs/assets/readme/split-roof-summit-refuge.gif" width="220" alt="Bush plane over a split-roof alpine refuge"></a><br><a href="docs/assets/current/split-roof-summit-refuge.png">Still</a><br>Bush plane over a split-roof alpine refuge</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/island-landing-craft.mp4"><img src="docs/assets/readme/island-landing-craft.gif" width="220" alt="Bald Eagle passes an island landing craft"></a><br><a href="docs/assets/current/island-landing-craft.png">Still</a><br>Bald Eagle passes an island landing craft</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/fish-farm-barge.mp4"><img src="docs/assets/readme/fish-farm-barge.gif" width="220" alt="Bald Eagle over a fish farm feeder barge"></a><br><a href="docs/assets/current/fish-farm-barge.png">Still</a><br>Bald Eagle over a fish farm feeder barge</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/night-industrial-skyline.mp4"><img src="docs/assets/readme/night-industrial-skyline.gif" width="220" alt="Ultralight over the industrial skyline at night"></a><br><a href="docs/assets/current/night-industrial-skyline.png">Still</a><br>Ultralight over the industrial skyline at night</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/linked-downtown.mp4"><img src="docs/assets/readme/linked-downtown.gif" width="220" alt="Ultralight approaching a linked downtown"></a><br><a href="docs/assets/current/linked-downtown.png">Still</a><br>Ultralight approaching a linked downtown</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/road-rest-stop.mp4"><img src="docs/assets/readme/road-rest-stop.gif" width="220" alt="Roadside rest stop"></a><br><a href="docs/assets/current/road-rest-stop.png">Still</a><br>Roadside rest stop</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/woodland-trailhead.mp4"><img src="docs/assets/readme/woodland-trailhead.gif" width="220" alt="Woodland trailhead"></a><br><a href="docs/assets/current/woodland-trailhead.png">Still</a><br>Woodland trailhead</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/road-overlook.mp4"><img src="docs/assets/readme/road-overlook.gif" width="220" alt="Mountain road overlook"></a><br><a href="docs/assets/current/road-overlook.png">Still</a><br>Mountain road overlook</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/freight-apron-preview.mp4"><img src="docs/assets/readme/freight-apron-preview.gif" width="220" alt="Freight siding and loading apron"></a><br><a href="docs/assets/current/freight-apron-preview.png">Still</a><br>Freight siding and loading apron</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/station-platform-preview.mp4"><img src="docs/assets/readme/station-platform-preview.gif" width="220" alt="Passenger station platform"></a><br><a href="docs/assets/current/station-platform-preview.png">Still</a><br>Passenger station platform</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/farm-details-flyover.mp4"><img src="docs/assets/readme/farm-details-flyover.gif" width="220" alt="Harvest equipment beside the fields"></a><br><a href="docs/assets/current/farm-details-flyover.png">Still</a><br>Harvest equipment beside the fields</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/current/coastal-boardwalk.png"><img src="docs/assets/current/coastal-boardwalk.png" width="220" alt="Coastal boardwalk village"></a><br>Coastal boardwalk village</td>
    <td align="center"><a href="docs/assets/current/river-boardwalk.png"><img src="docs/assets/current/river-boardwalk.png" width="220" alt="River boardwalk and lookout"></a><br>River boardwalk and lookout</td>
    <td align="center"><a href="docs/assets/current/ferry-quayside.png"><img src="docs/assets/current/ferry-quayside.png" width="220" alt="Ferry quay and workshops"></a><br>Ferry quay and workshops</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/current/beach-umbrellas.png"><img src="docs/assets/current/beach-umbrellas.png" width="220" alt="Beach umbrellas and towels"></a><br>Beach umbrellas and towels</td>
    <td align="center"><a href="docs/assets/current/coastal-lighthouse.png"><img src="docs/assets/current/coastal-lighthouse.png" width="220" alt="Coastal lighthouse"></a><br>Coastal lighthouse</td>
    <td align="center"><a href="docs/assets/current/hillside-town.png"><img src="docs/assets/current/hillside-town.png" width="220" alt="Hillside town"></a><br>Hillside town</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/current/river-road-bridge.png"><img src="docs/assets/current/river-road-bridge.png" width="220" alt="River road bridge"></a><br>River road bridge</td>
    <td align="center"><a href="docs/assets/current/regional-airfield.png"><img src="docs/assets/current/regional-airfield.png" width="220" alt="Regional airfield"></a><br>Regional airfield</td>
    <td align="center"><a href="docs/assets/current/grass-airstrip.png"><img src="docs/assets/current/grass-airstrip.png" width="220" alt="Country airstrip"></a><br>Country airstrip</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/freight-harbor.mp4"><img src="docs/assets/readme/freight-harbor.gif" width="220" alt="Freight harbor"></a><br><a href="docs/assets/current/freight-harbor.png">Still</a><br>Freight harbor</td>
    <td align="center"><a href="docs/assets/current/rail-earthworks.png"><img src="docs/assets/current/rail-earthworks.png" width="220" alt="Rail cutting and gravel embankment"></a><br>Rail cutting and gravel embankment</td>
    <td align="center"><a href="docs/assets/current/aggregate-service.png"><img src="docs/assets/current/aggregate-service.png" width="220" alt="Loaded gravel train and empty return"></a><br>Loaded gravel train and empty return</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/current/gravel-freight.png"><img src="docs/assets/current/gravel-freight.png" width="220" alt="Gravel pit and aggregate freight"></a><br>Gravel pit and aggregate freight</td>
    <td align="center"><a href="docs/assets/current/contour-fields.png"><img src="docs/assets/current/contour-fields.png" width="220" alt="Contour strips and grass buffers"></a><br>Contour strips and grass buffers</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/grain-terminal.gif"><img src="docs/assets/readme/grain-terminal.gif" width="220" alt="Rural grain elevator and hopper train"></a><br><a href="docs/assets/current/grain-terminal.png">Still</a><br>Rural grain elevator and hopper train</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/current/freight-loading.png"><img src="docs/assets/current/freight-loading.png" width="220" alt="Gravel and container loading aprons"></a><br>Gravel and container loading aprons</td>
    <td align="center"><a href="docs/assets/current/freight-platforms.png"><img src="docs/assets/current/freight-platforms.png" width="220" alt="Rail loading apron and inland transfer shed"></a><br>Rail loading apron and inland transfer shed</td>
    <td align="center"><a href="docs/assets/current/freight-steel-coils.png"><img src="docs/assets/current/freight-steel-coils.png" width="220" alt="Steel coils and machinery freight"></a><br>Steel coils and machinery freight</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/current/freight-cold-storage.png"><img src="docs/assets/current/freight-cold-storage.png" width="220" alt="Frozen-food reefers"></a><br>Frozen-food reefers</td>
    <td align="center"><a href="docs/assets/current/freight-tankers.png"><img src="docs/assets/current/freight-tankers.png" width="220" alt="Food-oil tank cars"></a><br>Food-oil tank cars</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/freight-containers.gif"><img src="docs/assets/readme/freight-containers.gif" width="220" alt="Container train below the Ultralight"></a><br><a href="docs/assets/current/freight-containers.png">Still</a><br>Container train below the Ultralight</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/ferry-terminal.mp4"><img src="docs/assets/readme/ferry-terminal.gif" width="220" alt="Ferry terminal"></a><br><a href="docs/assets/current/ferry-terminal.png">Still</a><br>Ferry terminal</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/marina-piers.mp4"><img src="docs/assets/readme/marina-piers.gif" width="220" alt="Marina and sailboats"></a><br><a href="docs/assets/current/marina-piers.png">Still</a><br>Marina and sailboats</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/city-approach.mp4"><img src="docs/assets/readme/city-approach.gif" width="220" alt="Racer approaching the skyline"></a><br><a href="docs/assets/current/city-approach.png">Still</a><br>Racer approaching the skyline</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/night-neighborhoods.mp4"><img src="docs/assets/readme/night-neighborhoods.gif" width="220" alt="Ultralight over the city at night"></a><br><a href="docs/assets/current/night-neighborhoods.png">Still</a><br>Ultralight over the city at night</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/neighborhood-avenue.mp4"><img src="docs/assets/readme/neighborhood-avenue.gif" width="220" alt="Racer over the outer neighborhoods"></a><br><a href="docs/assets/current/neighborhood-avenue.png">Still</a><br>Racer over the outer neighborhoods</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/country-strip-pass.mp4"><img src="docs/assets/readme/country-strip-pass.gif" width="220" alt="Ultralight over a country airstrip"></a><br><a href="docs/assets/current/country-strip-pass.png">Still</a><br>Ultralight over a country airstrip</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/regional-airfield-apron.mp4"><img src="docs/assets/readme/regional-airfield-apron.gif" width="220" alt="Ultralight by the regional apron"></a><br><a href="docs/assets/current/regional-airfield-apron.png">Still</a><br>Ultralight by the regional apron</td>
  </tr>
</table>

### Encounters

<table>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/blackbird-refueling.mp4"><img src="docs/assets/readme/blackbird-refueling.gif" width="220" alt="Blackbird refueling"></a><br><a href="docs/assets/current/blackbird-refueling.png">Still</a><br>Blackbird refueling</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/formation-echelon.mp4"><img src="docs/assets/readme/formation-echelon.gif" width="220" alt="Fighter echelon and peel-away"></a><br><a href="docs/assets/current/formation-echelon.png">Still</a><br>Fighter echelon and peel-away</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/formation-four.mp4"><img src="docs/assets/readme/formation-four.gif" width="220" alt="Fighter wedge · 4 wingmates"></a><br><a href="docs/assets/current/formation-four.png">Still</a><br>Fighter wedge · 4 wingmates</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/formation-pair.mp4"><img src="docs/assets/readme/formation-pair.gif" width="220" alt="Fighter line abreast"></a><br><a href="docs/assets/current/formation-pair.png">Still</a><br>Fighter line abreast</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/blackbird-reheat.mp4"><img src="docs/assets/readme/blackbird-reheat.gif" width="220" alt="Blackbird purple reheat"></a><br><a href="docs/assets/current/blackbird-reheat.png">Still</a><br>Blackbird purple reheat</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/fighter-afterburners.mp4"><img src="docs/assets/readme/fighter-afterburners.gif" width="220" alt="Fighter orange afterburner"></a><br><a href="docs/assets/current/fighter-afterburners.png">Still</a><br>Fighter orange afterburner</td>
  </tr>
</table>

### Aircraft

<table>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/racer-bank.mp4"><img src="docs/assets/readme/racer-bank.gif" width="220" alt="Racer banks around a ridge"></a><br><a href="docs/assets/current/racer-bank.png">Still</a><br>Racer banks around a ridge</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/glider-reflection.gif"><img src="docs/assets/readme/glider-reflection.gif" width="220" alt="Sun glint in a thermal turn"></a><br><a href="docs/assets/current/glider-reflection.png">Still</a><br>Sun glint in a thermal turn</td>
  </tr>
</table>

### Sky & Weather

<table>
  <tr>
    <td align="center"><a href="docs/assets/current/shower-rainbow.png"><img src="docs/assets/current/shower-rainbow.png" width="220" alt="Rainbow in a coastal shower"></a><br>Rainbow in a coastal shower</td>
    <td align="center"><a href="docs/assets/current/dusk-sky.png"><img src="docs/assets/current/dusk-sky.png" width="220" alt="Moon over the city"></a><br>Moon over the city</td>
  </tr>
</table>

### Interface

<table>
  <tr>
    <td align="center"><a href="docs/assets/current/flight-info.png"><img src="docs/assets/current/flight-info.png" width="220" alt="Flight information overlay"></a><br>Flight information overlay</td>
    <td align="center"><a href="docs/assets/current/new-world-ui.png"><img src="docs/assets/current/new-world-ui.png" width="220" alt="New World hold progress"></a><br>New World hold progress</td>
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

#### Airfield aprons · 25 variants

<a href="docs/assets/artwork/airfield-aprons.png"><img src="docs/assets/artwork/airfield-aprons.png" width="780" alt="Airfield aprons"></a>

<a href="docs/assets/artwork/airfield-aprons.gif"><img src="docs/assets/artwork/airfield-aprons.gif" width="550" alt="Airfield aprons rotating"></a>

#### Hillside quarries · 25 variants

<a href="docs/assets/artwork/quarry-profiles.png"><img src="docs/assets/artwork/quarry-profiles.png" width="780" alt="Hillside quarries"></a>

<a href="docs/assets/artwork/quarry-profiles.gif"><img src="docs/assets/artwork/quarry-profiles.gif" width="550" alt="Hillside quarries rotating"></a>

#### Quayside workshops · 25 variants

<a href="docs/assets/artwork/quay-workshops.png"><img src="docs/assets/artwork/quay-workshops.png" width="780" alt="Quayside workshops"></a>

<a href="docs/assets/artwork/quay-workshops.gif"><img src="docs/assets/artwork/quay-workshops.gif" width="550" alt="Quayside workshops rotating"></a>

#### Sheltered cove dunes · 25 variants

<a href="docs/assets/artwork/cove-dunes.png"><img src="docs/assets/artwork/cove-dunes.png" width="780" alt="Sheltered cove dunes"></a>

<a href="docs/assets/artwork/cove-dunes.gif"><img src="docs/assets/artwork/cove-dunes.gif" width="550" alt="Sheltered cove dunes rotating"></a>

#### Cultivated fields · 25 variants

<a href="docs/assets/artwork/cultivated-fields.png"><img src="docs/assets/artwork/cultivated-fields.png" width="780" alt="Cultivated fields"></a>

<a href="docs/assets/artwork/cultivated-fields.gif"><img src="docs/assets/artwork/cultivated-fields.gif" width="550" alt="Cultivated fields rotating"></a>

#### Columnar rock faces · 25 variants

<a href="docs/assets/artwork/columnar-headwalls.png"><img src="docs/assets/artwork/columnar-headwalls.png" width="780" alt="Columnar rock faces"></a>

<a href="docs/assets/artwork/columnar-headwalls.gif"><img src="docs/assets/artwork/columnar-headwalls.gif" width="550" alt="Columnar rock faces rotating"></a>

#### Desert alluvial fans · 25 variants

<a href="docs/assets/artwork/alluvial-fans.png"><img src="docs/assets/artwork/alluvial-fans.png" width="780" alt="Desert alluvial fans"></a>

<a href="docs/assets/artwork/alluvial-fans.gif"><img src="docs/assets/artwork/alluvial-fans.gif" width="550" alt="Desert alluvial fans rotating"></a>

#### Solar array layouts · 25 variants

<a href="docs/assets/artwork/solar-layouts.png"><img src="docs/assets/artwork/solar-layouts.png" width="780" alt="Solar array layouts"></a>

<a href="docs/assets/artwork/solar-layouts.gif"><img src="docs/assets/artwork/solar-layouts.gif" width="550" alt="Solar array layouts rotating"></a>

#### Municipal water towers · 25 variants

<a href="docs/assets/artwork/water-towers.png"><img src="docs/assets/artwork/water-towers.png" width="780" alt="Municipal water towers"></a>

<a href="docs/assets/artwork/water-towers.gif"><img src="docs/assets/artwork/water-towers.gif" width="550" alt="Municipal water towers rotating"></a>

#### Glacial moraines · 25 variants

<a href="docs/assets/artwork/moraines.png"><img src="docs/assets/artwork/moraines.png" width="780" alt="Glacial moraines"></a>

<a href="docs/assets/artwork/moraines.gif"><img src="docs/assets/artwork/moraines.gif" width="550" alt="Glacial moraines rotating"></a>

#### Mountain lake shores · 25 variants

<a href="docs/assets/artwork/tarns.png"><img src="docs/assets/artwork/tarns.png" width="780" alt="Mountain lake shores"></a>

<a href="docs/assets/artwork/tarns.gif"><img src="docs/assets/artwork/tarns.gif" width="550" alt="Mountain lake shores rotating"></a>

#### Alpine glaciers · 25 variants

<a href="docs/assets/artwork/glaciers.png"><img src="docs/assets/artwork/glaciers.png" width="780" alt="Alpine glaciers"></a>

<a href="docs/assets/artwork/glaciers.gif"><img src="docs/assets/artwork/glaciers.gif" width="550" alt="Alpine glaciers rotating"></a>

#### River islands · 25 variants

<a href="docs/assets/artwork/river-islands.png"><img src="docs/assets/artwork/river-islands.png" width="780" alt="River islands"></a>

<a href="docs/assets/artwork/river-islands.gif"><img src="docs/assets/artwork/river-islands.gif" width="550" alt="River islands rotating"></a>

#### River and coastal bridges · 25 variants

<a href="docs/assets/artwork/bridges.png"><img src="docs/assets/artwork/bridges.png" width="780" alt="River and coastal bridges"></a>

<a href="docs/assets/artwork/bridges.gif"><img src="docs/assets/artwork/bridges.gif" width="550" alt="River and coastal bridges rotating"></a>

#### Mountain cableways · 25 variants

<a href="docs/assets/artwork/cableways.png"><img src="docs/assets/artwork/cableways.png" width="780" alt="Mountain cableways"></a>

<a href="docs/assets/artwork/cableways.gif"><img src="docs/assets/artwork/cableways.gif" width="550" alt="Mountain cableways rotating"></a>

#### Lighthouses · 25 variants

<a href="docs/assets/artwork/lighthouses.png"><img src="docs/assets/artwork/lighthouses.png" width="780" alt="Lighthouses"></a>

<a href="docs/assets/artwork/lighthouses.gif"><img src="docs/assets/artwork/lighthouses.gif" width="550" alt="Lighthouses rotating"></a>

#### Coastal rock formations · 25 variants

<a href="docs/assets/artwork/coastal-rocks.png"><img src="docs/assets/artwork/coastal-rocks.png" width="780" alt="Coastal rock formations"></a>

<a href="docs/assets/artwork/coastal-rocks.gif"><img src="docs/assets/artwork/coastal-rocks.gif" width="550" alt="Coastal rock formations rotating"></a>

#### Countryside buildings · 25 variants

<a href="docs/assets/artwork/countryside-buildings.png"><img src="docs/assets/artwork/countryside-buildings.png" width="780" alt="Countryside buildings"></a>

<a href="docs/assets/artwork/countryside-buildings.gif"><img src="docs/assets/artwork/countryside-buildings.gif" width="550" alt="Countryside buildings rotating"></a>

#### Sandstone mesas · 25 variants

<a href="docs/assets/artwork/mesa-profiles.png"><img src="docs/assets/artwork/mesa-profiles.png" width="780" alt="Sandstone mesas"></a>

<a href="docs/assets/artwork/mesa-profiles.gif"><img src="docs/assets/artwork/mesa-profiles.gif" width="550" alt="Sandstone mesas rotating"></a>

#### Floodplain details · 25 variants

<a href="docs/assets/artwork/floodplain-details.png"><img src="docs/assets/artwork/floodplain-details.png" width="780" alt="Floodplain details"></a>

<a href="docs/assets/artwork/floodplain-details.gif"><img src="docs/assets/artwork/floodplain-details.gif" width="550" alt="Floodplain details rotating"></a>

#### Industrial landmarks · 25 variants

<a href="docs/assets/artwork/industrial-landmarks.png"><img src="docs/assets/artwork/industrial-landmarks.png" width="780" alt="Industrial landmarks"></a>

<a href="docs/assets/artwork/industrial-landmarks.gif"><img src="docs/assets/artwork/industrial-landmarks.gif" width="550" alt="Industrial landmarks rotating"></a>

#### Alpine ground details · 25 variants

<a href="docs/assets/artwork/alpine-ground-details.png"><img src="docs/assets/artwork/alpine-ground-details.png" width="780" alt="Alpine ground details"></a>

<a href="docs/assets/artwork/alpine-ground-details.gif"><img src="docs/assets/artwork/alpine-ground-details.gif" width="550" alt="Alpine ground details rotating"></a>

#### Headland details · 25 variants

<a href="docs/assets/artwork/headland-details.png"><img src="docs/assets/artwork/headland-details.png" width="780" alt="Headland details"></a>

<a href="docs/assets/artwork/headland-details.gif"><img src="docs/assets/artwork/headland-details.gif" width="550" alt="Headland details rotating"></a>

#### Grassland details · 25 variants

<a href="docs/assets/artwork/grassland-details.png"><img src="docs/assets/artwork/grassland-details.png" width="780" alt="Grassland details"></a>

<a href="docs/assets/artwork/grassland-details.gif"><img src="docs/assets/artwork/grassland-details.gif" width="550" alt="Grassland details rotating"></a>

#### Dryland details · 25 variants

<a href="docs/assets/artwork/dryland-details.png"><img src="docs/assets/artwork/dryland-details.png" width="780" alt="Dryland details"></a>

<a href="docs/assets/artwork/dryland-details.gif"><img src="docs/assets/artwork/dryland-details.gif" width="550" alt="Dryland details rotating"></a>

#### Woodland details · 25 variants

<a href="docs/assets/artwork/woodland-details.png"><img src="docs/assets/artwork/woodland-details.png" width="780" alt="Woodland details"></a>

<a href="docs/assets/artwork/woodland-details.gif"><img src="docs/assets/artwork/woodland-details.gif" width="550" alt="Woodland details rotating"></a>

#### Airfield buildings · 25 variants

<a href="docs/assets/artwork/airfield-buildings.png"><img src="docs/assets/artwork/airfield-buildings.png" width="780" alt="Airfield buildings"></a>

<a href="docs/assets/artwork/airfield-buildings.gif"><img src="docs/assets/artwork/airfield-buildings.gif" width="550" alt="Airfield buildings rotating"></a>

#### Street buildings · 25 variants

<a href="docs/assets/artwork/street-buildings.png"><img src="docs/assets/artwork/street-buildings.png" width="780" alt="Street buildings"></a>

<a href="docs/assets/artwork/street-buildings.gif"><img src="docs/assets/artwork/street-buildings.gif" width="550" alt="Street buildings rotating"></a>

#### Freshwater details · 25 variants

<a href="docs/assets/artwork/freshwater-details.png"><img src="docs/assets/artwork/freshwater-details.png" width="780" alt="Freshwater details"></a>

<a href="docs/assets/artwork/freshwater-details.gif"><img src="docs/assets/artwork/freshwater-details.gif" width="550" alt="Freshwater details rotating"></a>

#### Mountain details · 25 variants

<a href="docs/assets/artwork/mountain-details.png"><img src="docs/assets/artwork/mountain-details.png" width="780" alt="Mountain details"></a>

<a href="docs/assets/artwork/mountain-details.gif"><img src="docs/assets/artwork/mountain-details.gif" width="550" alt="Mountain details rotating"></a>

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

#### Skyline landmarks · 75 variants

<a href="docs/assets/artwork/skyline-landmarks.png"><img src="docs/assets/artwork/skyline-landmarks.png" width="780" alt="Skyline landmarks"></a>

<a href="docs/assets/artwork/skyline-landmarks.gif"><img src="docs/assets/artwork/skyline-landmarks.gif" width="550" alt="Skyline landmarks rotating"></a>

#### Summit landmarks · 25 variants

<a href="docs/assets/artwork/summit-landmarks.png"><img src="docs/assets/artwork/summit-landmarks.png" width="780" alt="Summit landmarks"></a>

<a href="docs/assets/artwork/summit-landmarks.gif"><img src="docs/assets/artwork/summit-landmarks.gif" width="550" alt="Summit landmarks rotating"></a>

#### Town landmarks · 25 variants

<a href="docs/assets/artwork/town-landmarks.png"><img src="docs/assets/artwork/town-landmarks.png" width="780" alt="Town landmarks"></a>

<a href="docs/assets/artwork/town-landmarks.gif"><img src="docs/assets/artwork/town-landmarks.gif" width="550" alt="Town landmarks rotating"></a>

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

#### Service vessels · 25 variants

<a href="docs/assets/artwork/service-vessels.png"><img src="docs/assets/artwork/service-vessels.png" width="780" alt="Service vessels"></a>

<a href="docs/assets/artwork/service-vessels.gif"><img src="docs/assets/artwork/service-vessels.gif" width="550" alt="Service vessels rotating"></a>

#### Wetland plants · 256 variants

<a href="docs/assets/artwork/wetlands.png"><img src="docs/assets/artwork/wetlands.png" width="780" alt="Wetland plants"></a>

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
