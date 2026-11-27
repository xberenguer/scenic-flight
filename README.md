# Scenic Flight

A Rust flight simulator for the round Waveshare ESP32-S3-Touch-AMOLED-1.75C. Pick an aircraft and fly over Los Angeles, Las Vegas, or a procedural world.

[Play in browser](https://xberenguer.github.io/scenic-flight/play.html) · [Fly Los Angeles](https://xberenguer.github.io/scenic-flight/play.html?destination=los-angeles&session=215) · [Fly Las Vegas](https://xberenguer.github.io/scenic-flight/play.html?destination=las-vegas&session=215) · [Interactive gallery](https://xberenguer.github.io/scenic-flight/) · [Install on Waveshare](https://xn--sg8hhb.ws/flasher/?project=scenic-flight)

<!-- BEGIN GALLERY -->

## Gameplay showcase

Select a preview to open the full-size clip or screenshot.

<table>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/la-downtown.mp4"><img src="docs/assets/readme/la-downtown.gif" width="220" alt="Glider over Downtown Los Angeles"></a><br><a href="docs/assets/current/la-downtown.png">Still</a><br>Glider over Downtown Los Angeles</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/la-lax.mp4"><img src="docs/assets/readme/la-lax.gif" width="220" alt="Sport Jet on the ocean approach to LAX"></a><br><a href="docs/assets/current/la-lax.png">Still</a><br>Sport Jet on the ocean approach to LAX</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/la-santa-monica.mp4"><img src="docs/assets/readme/la-santa-monica.gif" width="220" alt="Bush Plane approaching Santa Monica Pier"></a><br><a href="docs/assets/current/la-santa-monica.png">Still</a><br>Bush Plane approaching Santa Monica Pier</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/la-griffith-observatory.mp4"><img src="docs/assets/readme/la-griffith-observatory.gif" width="220" alt="Bush Plane on the Griffith Observatory approach"></a><br><a href="docs/assets/current/la-griffith-observatory.png">Still</a><br>Bush Plane on the Griffith Observatory approach</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/vegas-sphere.mp4"><img src="docs/assets/readme/vegas-sphere.gif" width="220" alt="Racer passing the Sphere&#x27;s animated emoji"></a><br><a href="docs/assets/current/vegas-sphere.png">Still</a><br>Racer passing the Sphere&#x27;s animated emoji</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/readme/southwest-barstow.gif"><img src="docs/assets/readme/southwest-barstow.gif" width="220" alt="Bush Plane over Barstow"></a><br><a href="docs/assets/current/southwest-barstow.png">Still</a><br>Bush Plane over Barstow</td>
  </tr>
</table>

[Sky and weather](#sky--weather) · [Encounters](#encounters) · [Biomes](#biomes) · [Built world](#built-world) · [Aircraft](#aircraft) · [Interface](#interface) · [Aircraft and picker screens](#aircraft-and-picker-screens) · [Rare flight moments](#rare-flight-moments) · [Artwork gallery](#artwork-gallery) · [Las Vegas artwork](#las-vegas-artwork) · [World generation](#world-generation)

### Sky & Weather

<table>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/sky-weather-snow-0.mp4"><img src="docs/assets/readme/sky-weather-snow-0.gif" width="220" alt="Snow curtain over high country"></a><br><a href="docs/assets/current/sky-weather-snow-0.png">Still</a><br>Snow curtain over high country</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/sky-weather-snow-1.mp4"><img src="docs/assets/readme/sky-weather-snow-1.gif" width="220" alt="Narrow snow shaft"></a><br><a href="docs/assets/current/sky-weather-snow-1.png">Still</a><br>Narrow snow shaft</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/sky-weather-snow-18.mp4"><img src="docs/assets/readme/sky-weather-snow-18.gif" width="220" alt="Wind-swept snow sheet"></a><br><a href="docs/assets/current/sky-weather-snow-18.png">Still</a><br>Wind-swept snow sheet</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/sky-weather-snow-23.mp4"><img src="docs/assets/readme/sky-weather-snow-23.gif" width="220" alt="Broken snow curtains"></a><br><a href="docs/assets/current/sky-weather-snow-23.png">Still</a><br>Broken snow curtains</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/sky-weather-summer-sunset.mp4"><img src="docs/assets/readme/sky-weather-summer-sunset.gif" width="220" alt="Summer sunset"></a><br><a href="docs/assets/current/sky-weather-summer-sunset.png">Still</a><br>Summer sunset</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/sky-weather-winter-sunset.mp4"><img src="docs/assets/readme/sky-weather-winter-sunset.gif" width="220" alt="Winter sunset"></a><br><a href="docs/assets/current/sky-weather-winter-sunset.png">Still</a><br>Winter sunset</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/sky-weather-clouds.mp4"><img src="docs/assets/readme/sky-weather-clouds.gif" width="220" alt="Drifting clouds above the coast"></a><br><a href="docs/assets/current/sky-weather-clouds.png">Still</a><br>Drifting clouds above the coast</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/sky-weather-rain.mp4"><img src="docs/assets/readme/sky-weather-rain.gif" width="220" alt="Rain shaft over the coast"></a><br><a href="docs/assets/current/sky-weather-rain.png">Still</a><br>Rain shaft over the coast</td>
    <td align="center"><a href="docs/assets/current/shower-rainbow.png"><img src="docs/assets/current/shower-rainbow.png" width="220" alt="Rainbow in a coastal shower"></a><br>Rainbow in a coastal shower</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/sky-weather-mist.mp4"><img src="docs/assets/readme/sky-weather-mist.gif" width="220" alt="Valley mist at dawn"></a><br><a href="docs/assets/current/sky-weather-mist.png">Still</a><br>Valley mist at dawn</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/snowfield-glider.mp4"><img src="docs/assets/readme/snowfield-glider.gif" width="220" alt="Glider over an alpine snowfield"></a><br><a href="docs/assets/current/snowfield-glider.png">Still</a><br>Glider over an alpine snowfield</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/sky-weather-sunrise.mp4"><img src="docs/assets/readme/sky-weather-sunrise.gif" width="220" alt="Sunrise over the ocean"></a><br><a href="docs/assets/current/sky-weather-sunrise.png">Still</a><br>Sunrise over the ocean</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/sky-weather-sunset.mp4"><img src="docs/assets/readme/sky-weather-sunset.gif" width="220" alt="Sunset behind a mountain ridge"></a><br><a href="docs/assets/current/sky-weather-sunset.png">Still</a><br>Sunset behind a mountain ridge</td>
    <td align="center"><a href="docs/assets/current/sky-weather-stars.png"><img src="docs/assets/current/sky-weather-stars.png" width="220" alt="Stars over a dark coastline"></a><br>Stars over a dark coastline</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/sky-weather-meteor.mp4"><img src="docs/assets/readme/sky-weather-meteor.gif" width="220" alt="Shooting star on a clear night"></a><br><a href="docs/assets/current/sky-weather-meteor.png">Still</a><br>Shooting star on a clear night</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/current/sky-weather-lunar-detail.png"><img src="docs/assets/current/sky-weather-lunar-detail.png" width="220" alt="Moon phases up close"></a><br>Moon phases up close</td>
    <td align="center"><a href="docs/assets/current/sky-weather-crescent.png"><img src="docs/assets/current/sky-weather-crescent.png" width="220" alt="Crescent Moon"></a><br>Crescent Moon</td>
    <td align="center"><a href="docs/assets/current/sky-weather-quarter.png"><img src="docs/assets/current/sky-weather-quarter.png" width="220" alt="Quarter Moon"></a><br>Quarter Moon</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/current/sky-weather-gibbous.png"><img src="docs/assets/current/sky-weather-gibbous.png" width="220" alt="Gibbous Moon"></a><br>Gibbous Moon</td>
    <td align="center"><a href="docs/assets/current/sky-weather-full.png"><img src="docs/assets/current/sky-weather-full.png" width="220" alt="Full Moon"></a><br>Full Moon</td>
    <td align="center"><a href="docs/assets/current/dusk-sky.png"><img src="docs/assets/current/dusk-sky.png" width="220" alt="Moon over the city"></a><br>Moon over the city</td>
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

### Biomes

<table>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/dry-coast-flight.mp4"><img src="docs/assets/readme/dry-coast-flight.gif" width="220" alt="Ultralight over coastal scrub"></a><br><a href="docs/assets/current/dry-coast-flight.png">Still</a><br>Ultralight over coastal scrub</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/sandstone-ridge-flight.mp4"><img src="docs/assets/readme/sandstone-ridge-flight.gif" width="220" alt="Bald Eagle alongside sandstone ridges"></a><br><a href="docs/assets/current/sandstone-ridge-flight.png">Still</a><br>Bald Eagle alongside sandstone ridges</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/dry-foothills-flight.mp4"><img src="docs/assets/readme/dry-foothills-flight.gif" width="220" alt="Bald Eagle over desert foothills"></a><br><a href="docs/assets/current/dry-foothills-flight.png">Still</a><br>Bald Eagle over desert foothills</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/summit-trail-ridge-canopy.mp4"><img src="docs/assets/readme/summit-trail-ridge-canopy.gif" width="220" alt="Bush Plane over a leaning ridge canopy"></a><br><a href="docs/assets/current/summit-trail-ridge-canopy.png">Still</a><br>Bush Plane over a leaning ridge canopy</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/summit-trail-red-gable.mp4"><img src="docs/assets/readme/summit-trail-red-gable.gif" width="220" alt="Ultralight passing a red gable information hut"></a><br><a href="docs/assets/current/summit-trail-red-gable.png">Still</a><br>Ultralight passing a red gable information hut</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/summit-trail-viewing-decks.mp4"><img src="docs/assets/readme/summit-trail-viewing-decks.gif" width="220" alt="Glider over a trail with two-level viewing decks"></a><br><a href="docs/assets/current/summit-trail-viewing-decks.png">Still</a><br>Glider over a trail with two-level viewing decks</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/summit-trail-windward-shelter.mp4"><img src="docs/assets/readme/summit-trail-windward-shelter.gif" width="220" alt="Bald Eagle beside a windward stone shelter"></a><br><a href="docs/assets/current/summit-trail-windward-shelter.png">Still</a><br>Bald Eagle beside a windward stone shelter</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/badlands-terraced-cleft.mp4"><img src="docs/assets/readme/badlands-terraced-cleft.gif" width="220" alt="Bald Eagle over a terraced cleft"></a><br><a href="docs/assets/current/badlands-terraced-cleft.png">Still</a><br>Bald Eagle over a terraced cleft</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/badlands-hourglass.mp4"><img src="docs/assets/readme/badlands-hourglass.gif" width="220" alt="Glider over a narrow badlands canyon"></a><br><a href="docs/assets/current/badlands-hourglass.png">Still</a><br>Glider over a narrow badlands canyon</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/badlands-open-wash.mp4"><img src="docs/assets/readme/badlands-open-wash.gif" width="220" alt="Bush Plane above an open wash mouth"></a><br><a href="docs/assets/current/badlands-open-wash.png">Still</a><br>Bush Plane above an open wash mouth</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/badlands-boxhead.mp4"><img src="docs/assets/readme/badlands-boxhead.gif" width="220" alt="Ultralight over a box-headed ravine"></a><br><a href="docs/assets/current/badlands-boxhead.png">Still</a><br>Ultralight over a box-headed ravine</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/glacial-outwash-splay.mp4"><img src="docs/assets/readme/glacial-outwash-splay.gif" width="220" alt="Bush Plane over a diverging outwash fan"></a><br><a href="docs/assets/current/glacial-outwash-splay.png">Still</a><br>Bush Plane over a diverging outwash fan</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/glacial-outwash-chute.mp4"><img src="docs/assets/readme/glacial-outwash-chute.gif" width="220" alt="Ultralight over an inset meltwater chute"></a><br><a href="docs/assets/current/glacial-outwash-chute.png">Still</a><br>Ultralight over an inset meltwater chute</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/glacial-outwash-braidplain.mp4"><img src="docs/assets/readme/glacial-outwash-braidplain.gif" width="220" alt="Glider over a wide glacial braidplain"></a><br><a href="docs/assets/current/glacial-outwash-braidplain.png">Still</a><br>Glider over a wide glacial braidplain</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/coastal-bluff-rotated-slump.mp4"><img src="docs/assets/readme/coastal-bluff-rotated-slump.gif" width="220" alt="Low pass along a coastal slump"></a><br><a href="docs/assets/current/coastal-bluff-rotated-slump.png">Still</a><br>Low pass along a coastal slump</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/coastal-bluff-twin-noses.mp4"><img src="docs/assets/readme/coastal-bluff-twin-noses.gif" width="220" alt="Twin rock spurs"></a><br><a href="docs/assets/current/coastal-bluff-twin-noses.png">Still</a><br>Twin rock spurs</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/coastal-bluff-raised-shelves.mp4"><img src="docs/assets/readme/coastal-bluff-raised-shelves.gif" width="220" alt="Raised shelves above the shoreline"></a><br><a href="docs/assets/current/coastal-bluff-raised-shelves.png">Still</a><br>Raised shelves above the shoreline</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/alpine-deep-convex-apron.mp4"><img src="docs/assets/readme/alpine-deep-convex-apron.gif" width="220" alt="Blackbird above a mountain rockfall fan"></a><br><a href="docs/assets/current/alpine-deep-convex-apron.png">Still</a><br>Blackbird above a mountain rockfall fan</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/alpine-stone-ribs-gullies.mp4"><img src="docs/assets/readme/alpine-stone-ribs-gullies.gif" width="220" alt="Stone ribs and fine gullies"></a><br><a href="docs/assets/current/alpine-stone-ribs-gullies.png">Still</a><br>Stone ribs and fine gullies</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/alpine-forked-rockfall.mp4"><img src="docs/assets/readme/alpine-forked-rockfall.gif" width="220" alt="Forked rockfall apron below the snow line"></a><br><a href="docs/assets/current/alpine-forked-rockfall.png">Still</a><br>Forked rockfall apron below the snow line</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/river-bend-scroll-terrace.mp4"><img src="docs/assets/readme/river-bend-scroll-terrace.gif" width="220" alt="Interrupted scroll terrace"></a><br><a href="docs/assets/current/river-bend-scroll-terrace.png">Still</a><br>Interrupted scroll terrace</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/river-bend-cobble-ridges.mp4"><img src="docs/assets/readme/river-bend-cobble-ridges.gif" width="220" alt="Cobble ridges and silt pockets"></a><br><a href="docs/assets/current/river-bend-cobble-ridges.png">Still</a><br>Cobble ridges and silt pockets</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/river-bend-sand-crescent.mp4"><img src="docs/assets/readme/river-bend-sand-crescent.gif" width="220" alt="Fine sand crescent inside a bend"></a><br><a href="docs/assets/current/river-bend-sand-crescent.png">Still</a><br>Fine sand crescent inside a bend</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/tarn-cascade-south.mp4"><img src="docs/assets/readme/tarn-cascade-south.gif" width="220" alt="Deflected rock cascade"></a><br><a href="docs/assets/current/tarn-cascade-south.png">Still</a><br>Deflected rock cascade</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/tarn-cascade-west.mp4"><img src="docs/assets/readme/tarn-cascade-west.gif" width="220" alt="Beaded twin cascade"></a><br><a href="docs/assets/current/tarn-cascade-west.png">Still</a><br>Beaded twin cascade</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/twin-cove-crests.mp4"><img src="docs/assets/readme/twin-cove-crests.gif" width="220" alt="Bald Eagle over twin dune crests"></a><br><a href="docs/assets/current/twin-cove-crests.png">Still</a><br>Bald Eagle over twin dune crests</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/elbow-cove-dune.mp4"><img src="docs/assets/readme/elbow-cove-dune.gif" width="220" alt="Glider following an elbow-shaped dune"></a><br><a href="docs/assets/current/elbow-cove-dune.png">Still</a><br>Glider following an elbow-shaped dune</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/notched-cove-dune.mp4"><img src="docs/assets/readme/notched-cove-dune.gif" width="220" alt="Ultralight along a notched cove dune"></a><br><a href="docs/assets/current/notched-cove-dune.png">Still</a><br>Ultralight along a notched cove dune</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/twin-rock-buttresses.mp4"><img src="docs/assets/readme/twin-rock-buttresses.gif" width="220" alt="Bald Eagle above twin rock buttresses"></a><br><a href="docs/assets/current/twin-rock-buttresses.png">Still</a><br>Bald Eagle above twin rock buttresses</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/descending-columnar-wall.mp4"><img src="docs/assets/readme/descending-columnar-wall.gif" width="220" alt="Glider over a descending rock face"></a><br><a href="docs/assets/current/descending-columnar-wall.png">Still</a><br>Glider over a descending rock face</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/fallen-columnar-blocks.mp4"><img src="docs/assets/readme/fallen-columnar-blocks.gif" width="220" alt="Bush Plane past a broad columnar outcrop"></a><br><a href="docs/assets/current/fallen-columnar-blocks.png">Still</a><br>Bush Plane past a broad columnar outcrop</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/late-split-alluvial-fan.mp4"><img src="docs/assets/readme/late-split-alluvial-fan.gif" width="220" alt="Bald Eagle over a three-way desert wash"></a><br><a href="docs/assets/current/late-split-alluvial-fan.png">Still</a><br>Bald Eagle over a three-way desert wash</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/narrow-alluvial-fan.mp4"><img src="docs/assets/readme/narrow-alluvial-fan.gif" width="220" alt="Glider above a narrow alluvial fan"></a><br><a href="docs/assets/current/narrow-alluvial-fan.png">Still</a><br>Glider above a narrow alluvial fan</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/wide-gravel-fan.mp4"><img src="docs/assets/readme/wide-gravel-fan.gif" width="220" alt="Ultralight over a wide gravel apron"></a><br><a href="docs/assets/current/wide-gravel-fan.png">Still</a><br>Ultralight over a wide gravel apron</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/broad-glacial-debris-apron.mp4"><img src="docs/assets/readme/broad-glacial-debris-apron.gif" width="220" alt="Ultralight past a broad moraine apron"></a><br><a href="docs/assets/current/broad-glacial-debris-apron.png">Still</a><br>Ultralight past a broad moraine apron</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/sinuous-glacial-moraine.mp4"><img src="docs/assets/readme/sinuous-glacial-moraine.gif" width="220" alt="Bald Eagle over a sinuous moraine"></a><br><a href="docs/assets/current/sinuous-glacial-moraine.png">Still</a><br>Bald Eagle over a sinuous moraine</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/twin-recessional-ridges.mp4"><img src="docs/assets/readme/twin-recessional-ridges.gif" width="220" alt="Glider over twin moraine ridges"></a><br><a href="docs/assets/current/twin-recessional-ridges.png">Still</a><br>Glider over twin moraine ridges</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/gravel-shoulder-tarn.mp4"><img src="docs/assets/readme/gravel-shoulder-tarn.gif" width="220" alt="Ultralight over a tarn with gravel shoulders"></a><br><a href="docs/assets/current/gravel-shoulder-tarn.png">Still</a><br>Ultralight over a tarn with gravel shoulders</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/sand-fan-tarn.mp4"><img src="docs/assets/readme/sand-fan-tarn.gif" width="220" alt="Glider past a sand-fan tarn"></a><br><a href="docs/assets/current/sand-fan-tarn.png">Still</a><br>Glider past a sand-fan tarn</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/sedge-strand-tarn.mp4"><img src="docs/assets/readme/sedge-strand-tarn.gif" width="220" alt="Bald Eagle over a sedge-lined tarn"></a><br><a href="docs/assets/current/sedge-strand-tarn.png">Still</a><br>Bald Eagle over a sedge-lined tarn</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/single-streak-moraine.mp4"><img src="docs/assets/readme/single-streak-moraine.gif" width="220" alt="Ultralight along a glacier moraine"></a><br><a href="docs/assets/current/single-streak-moraine.png">Still</a><br>Ultralight along a glacier moraine</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/bowed-blue-icefall.mp4"><img src="docs/assets/readme/bowed-blue-icefall.gif" width="220" alt="Glider over a bowed blue icefall"></a><br><a href="docs/assets/current/bowed-blue-icefall.png">Still</a><br>Glider over a bowed blue icefall</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/tapered-snow-apron.mp4"><img src="docs/assets/readme/tapered-snow-apron.gif" width="220" alt="Bald Eagle above a tapered snow apron"></a><br><a href="docs/assets/current/tapered-snow-apron.png">Still</a><br>Bald Eagle above a tapered snow apron</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/stone-crown-island.mp4"><img src="docs/assets/readme/stone-crown-island.gif" width="220" alt="Ultralight over a two-crown shingle island"></a><br><a href="docs/assets/current/stone-crown-island.png">Still</a><br>Ultralight over a two-crown shingle island</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/meadow-knoll-island.mp4"><img src="docs/assets/readme/meadow-knoll-island.gif" width="220" alt="Glider past twin meadow knolls"></a><br><a href="docs/assets/current/meadow-knoll-island.png">Still</a><br>Glider past twin meadow knolls</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/crescent-river-island.mp4"><img src="docs/assets/readme/crescent-river-island.gif" width="220" alt="Bald Eagle over a sandy crescent island"></a><br><a href="docs/assets/current/crescent-river-island.png">Still</a><br>Bald Eagle over a sandy crescent island</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/thin-stone-bridge.mp4"><img src="docs/assets/readme/thin-stone-bridge.gif" width="220" alt="Glider by a thin sea arch"></a><br><a href="docs/assets/current/thin-stone-bridge.png">Still</a><br>Glider by a thin sea arch</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/descending-skerries.mp4"><img src="docs/assets/readme/descending-skerries.gif" width="220" alt="Ultralight over descending skerries"></a><br><a href="docs/assets/current/descending-skerries.png">Still</a><br>Ultralight over descending skerries</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/needle-sea-stack.mp4"><img src="docs/assets/readme/needle-sea-stack.gif" width="220" alt="Bald Eagle past a needle sea stack"></a><br><a href="docs/assets/current/needle-sea-stack.png">Still</a><br>Bald Eagle past a needle sea stack</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/mesa-three-towers.mp4"><img src="docs/assets/readme/mesa-three-towers.gif" width="220" alt="Glider approaching three sandstone towers"></a><br><a href="docs/assets/current/mesa-three-towers.png">Still</a><br>Glider approaching three sandstone towers</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/mesa-ravine.mp4"><img src="docs/assets/readme/mesa-ravine.gif" width="220" alt="Bald Eagle along a sandstone ravine"></a><br><a href="docs/assets/current/mesa-ravine.png">Still</a><br>Bald Eagle along a sandstone ravine</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/mesa-double-ledge.mp4"><img src="docs/assets/readme/mesa-double-ledge.gif" width="220" alt="Ultralight over terraced sandstone mesas"></a><br><a href="docs/assets/current/mesa-double-ledge.png">Still</a><br>Ultralight over terraced sandstone mesas</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/flowering-reed-marsh.mp4"><img src="docs/assets/readme/flowering-reed-marsh.gif" width="220" alt="Ultralight along a reed marsh"></a><br><a href="docs/assets/current/flowering-reed-marsh.png">Still</a><br>Ultralight along a reed marsh</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/floodplain-bend.mp4"><img src="docs/assets/readme/floodplain-bend.gif" width="220" alt="Ultralight along a floodplain bend"></a><br><a href="docs/assets/current/floodplain-bend.png">Still</a><br>Ultralight along a floodplain bend</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/eagle-wet-meadow.mp4"><img src="docs/assets/readme/eagle-wet-meadow.gif" width="220" alt="Bald Eagle over a wet meadow"></a><br><a href="docs/assets/current/eagle-wet-meadow.png">Still</a><br>Bald Eagle over a wet meadow</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/glacier-ice-pass.mp4"><img src="docs/assets/readme/glacier-ice-pass.gif" width="220" alt="Glider above fractured ice"></a><br><a href="docs/assets/current/glacier-ice-pass.png">Still</a><br>Glider above fractured ice</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/alpine-ridgetop.mp4"><img src="docs/assets/readme/alpine-ridgetop.gif" width="220" alt="Bush Plane over a rocky alpine ridge"></a><br><a href="docs/assets/current/alpine-ridgetop.png">Still</a><br>Bush Plane over a rocky alpine ridge</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/alpine-rock-and-meadow.mp4"><img src="docs/assets/readme/alpine-rock-and-meadow.gif" width="220" alt="Ultralight over an alpine meadow"></a><br><a href="docs/assets/current/alpine-rock-and-meadow.png">Still</a><br>Ultralight over an alpine meadow</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/glider-upper-shore.mp4"><img src="docs/assets/readme/glider-upper-shore.gif" width="220" alt="Glider over coastal grasses"></a><br><a href="docs/assets/current/glider-upper-shore.png">Still</a><br>Glider over coastal grasses</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/eagle-headland.mp4"><img src="docs/assets/readme/eagle-headland.gif" width="220" alt="Bald Eagle along a sandy headland"></a><br><a href="docs/assets/current/eagle-headland.png">Still</a><br>Bald Eagle along a sandy headland</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/grassland-coastal-slopes.mp4"><img src="docs/assets/readme/grassland-coastal-slopes.gif" width="220" alt="Ultralight along coastal grassland"></a><br><a href="docs/assets/current/grassland-coastal-slopes.png">Still</a><br>Ultralight along coastal grassland</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/grassland-foothills.mp4"><img src="docs/assets/readme/grassland-foothills.gif" width="220" alt="Ultralight over grassy foothills"></a><br><a href="docs/assets/current/grassland-foothills.png">Still</a><br>Ultralight over grassy foothills</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/coastal-details-flight.mp4"><img src="docs/assets/readme/coastal-details-flight.gif" width="220" alt="Bald eagle along the sandy coast"></a><br><a href="docs/assets/current/coastal-details-flight.png">Still</a><br>Bald eagle along the sandy coast</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/current/riverside-town.png"><img src="docs/assets/current/riverside-town.png" width="220" alt="River through the foothills"></a><br>River through the foothills</td>
    <td align="center"><a href="docs/assets/current/cove-dune.png"><img src="docs/assets/current/cove-dune.png" width="220" alt="Cove dune"></a><br>Cove dune</td>
    <td align="center"><a href="docs/assets/current/alluvial-fan.png"><img src="docs/assets/current/alluvial-fan.png" width="220" alt="Desert alluvial fan"></a><br>Desert alluvial fan</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/current/badlands-ravine.png"><img src="docs/assets/current/badlands-ravine.png" width="220" alt="Badlands ravine"></a><br>Badlands ravine</td>
    <td align="center"><a href="docs/assets/current/inland-dunes.png"><img src="docs/assets/current/inland-dunes.png" width="220" alt="Inland dunes"></a><br>Inland dunes</td>
    <td align="center"><a href="docs/assets/current/salt-pan.png"><img src="docs/assets/current/salt-pan.png" width="220" alt="Salt pan"></a><br>Salt pan</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/current/columnar-rock.png"><img src="docs/assets/current/columnar-rock.png" width="220" alt="Columnar rock above an alpine scree slope"></a><br>Columnar rock above an alpine scree slope</td>
    <td align="center"><a href="docs/assets/current/alpine-tarn.png"><img src="docs/assets/current/alpine-tarn.png" width="220" alt="Alpine tarn"></a><br>Alpine tarn</td>
    <td align="center"><a href="docs/assets/current/oxbow-lake.png"><img src="docs/assets/current/oxbow-lake.png" width="220" alt="Oxbow lake"></a><br>Oxbow lake</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/current/coastal-stone-arch.png"><img src="docs/assets/current/coastal-stone-arch.png" width="220" alt="Sea arch"></a><br>Sea arch</td>
    <td align="center"><a href="docs/assets/current/coastal-skerries.png"><img src="docs/assets/current/coastal-skerries.png" width="220" alt="Skerries"></a><br>Skerries</td>
    <td align="center"><a href="docs/assets/current/coastal-sea-stack.png"><img src="docs/assets/current/coastal-sea-stack.png" width="220" alt="Sea stack"></a><br>Sea stack</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/current/ridge-view.png"><img src="docs/assets/current/ridge-view.png" width="220" alt="Ridge in daylight"></a><br>Ridge in daylight</td>
    <td align="center"><a href="docs/assets/current/bay-view.png"><img src="docs/assets/current/bay-view.png" width="220" alt="Bay and headlands"></a><br>Bay and headlands</td>
    <td align="center"><a href="docs/assets/current/cliffside.png"><img src="docs/assets/current/cliffside.png" width="220" alt="Cliffside flight"></a><br>Cliffside flight</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/bay-crossing.mp4"><img src="docs/assets/readme/bay-crossing.gif" width="220" alt="Flight over the bay"></a><br><a href="docs/assets/current/bay-crossing.png">Still</a><br>Flight over the bay</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/woodland-stands.mp4"><img src="docs/assets/readme/woodland-stands.gif" width="220" alt="Bush Plane along the wooded slopes"></a><br><a href="docs/assets/current/woodland-stands.png">Still</a><br>Bush Plane along the wooded slopes</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/glider-coast.mp4"><img src="docs/assets/readme/glider-coast.gif" width="220" alt="Glider on the coast"></a><br><a href="docs/assets/current/glider-coast.png">Still</a><br>Glider on the coast</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/river-valley.mp4"><img src="docs/assets/readme/river-valley.gif" width="220" alt="Ultralight over a river town"></a><br><a href="docs/assets/current/river-valley.png">Still</a><br>Ultralight over a river town</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/coastal-soaring.mp4"><img src="docs/assets/readme/coastal-soaring.gif" width="220" alt="Eagle over the foothills"></a><br><a href="docs/assets/current/coastal-soaring.png">Still</a><br>Eagle over the foothills</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/mountain-pass.mp4"><img src="docs/assets/readme/mountain-pass.gif" width="220" alt="Mountain pass"></a><br><a href="docs/assets/current/mountain-pass.png">Still</a><br>Mountain pass</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/ultralight-ridge.mp4"><img src="docs/assets/readme/ultralight-ridge.gif" width="220" alt="Ultralight over a rocky ridge"></a><br><a href="docs/assets/current/ultralight-ridge.png">Still</a><br>Ultralight over a rocky ridge</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/wooded-watershed.mp4"><img src="docs/assets/readme/wooded-watershed.gif" width="220" alt="Eagle over a wooded watershed"></a><br><a href="docs/assets/current/wooded-watershed.png">Still</a><br>Eagle over a wooded watershed</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/reedbed-meander.mp4"><img src="docs/assets/readme/reedbed-meander.gif" width="220" alt="Eagle over a reed-lined bend"></a><br><a href="docs/assets/current/reedbed-meander.png">Still</a><br>Eagle over a reed-lined bend</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/eagle-forest-edge.mp4"><img src="docs/assets/readme/eagle-forest-edge.gif" width="220" alt="Eagle along the forest edge"></a><br><a href="docs/assets/current/eagle-forest-edge.png">Still</a><br>Eagle along the forest edge</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/wooded-valley-pass.mp4"><img src="docs/assets/readme/wooded-valley-pass.gif" width="220" alt="Ultralight through a wooded valley"></a><br><a href="docs/assets/current/wooded-valley-pass.png">Still</a><br>Ultralight through a wooded valley</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/dryland-badlands-pass.mp4"><img src="docs/assets/readme/dryland-badlands-pass.gif" width="220" alt="Ultralight through the badlands"></a><br><a href="docs/assets/current/dryland-badlands-pass.png">Still</a><br>Ultralight through the badlands</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/dryland-basin-pass.mp4"><img src="docs/assets/readme/dryland-basin-pass.gif" width="220" alt="Ultralight over desert flats"></a><br><a href="docs/assets/current/dryland-basin-pass.png">Still</a><br>Ultralight over desert flats</td>
  </tr>
</table>

### Built World

<table>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/outskirts-flyover.mp4"><img src="docs/assets/readme/outskirts-flyover.gif" width="220" alt="Glider over the city outskirts"></a><br><a href="docs/assets/current/outskirts-flyover.png">Still</a><br>Glider over the city outskirts</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/metro-train.mp4"><img src="docs/assets/readme/metro-train.gif" width="220" alt="Ultralight passing a metropolitan train"></a><br><a href="docs/assets/current/metro-train.png">Still</a><br>Ultralight passing a metropolitan train</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/metropolitan-airport.mp4"><img src="docs/assets/readme/metropolitan-airport.gif" width="220" alt="Glider over a metropolitan airport"></a><br><a href="docs/assets/current/metropolitan-airport.png">Still</a><br>Glider over a metropolitan airport</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/blackbird-city-cruise.mp4"><img src="docs/assets/readme/blackbird-city-cruise.gif" width="220" alt="SR-71 above the city and clouds"></a><br><a href="docs/assets/current/blackbird-city-cruise.png">Still</a><br>SR-71 above the city and clouds</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/metropolis-flight.mp4"><img src="docs/assets/readme/metropolis-flight.gif" width="220" alt="Bald Eagle over a metropolis"></a><br><a href="docs/assets/current/metropolis-flight.png">Still</a><br>Bald Eagle over a metropolis</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/road-between-towns.mp4"><img src="docs/assets/readme/road-between-towns.gif" width="220" alt="Racer over a highway joining two towns"></a><br><a href="docs/assets/current/road-between-towns.png">Still</a><br>Racer over a highway joining two towns</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/highway-neighborhood-33.mp4"><img src="docs/assets/readme/highway-neighborhood-33.gif" width="220" alt="Bush Plane approaching a coastal town"></a><br><a href="docs/assets/current/highway-neighborhood-33.png">Still</a><br>Bush Plane approaching a coastal town</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/highway-neighborhood-18.mp4"><img src="docs/assets/readme/highway-neighborhood-18.gif" width="220" alt="Bush Plane over highway shops and duplexes"></a><br><a href="docs/assets/current/highway-neighborhood-18.png">Still</a><br>Bush Plane over highway shops and duplexes</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/contour-barley-steps.mp4"><img src="docs/assets/readme/contour-barley-steps.gif" width="220" alt="Glider over three narrow barley steps"></a><br><a href="docs/assets/current/contour-barley-steps.png">Still</a><br>Glider over three narrow barley steps</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/contour-grass-spine.mp4"><img src="docs/assets/readme/contour-grass-spine.gif" width="220" alt="Bush Plane along a grass spine and hay belts"></a><br><a href="docs/assets/current/contour-grass-spine.png">Still</a><br>Bush Plane along a grass spine and hay belts</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/contour-oat-fan.mp4"><img src="docs/assets/readme/contour-oat-fan.gif" width="220" alt="Sport Jet above a fan-shaped oat field"></a><br><a href="docs/assets/current/contour-oat-fan.png">Still</a><br>Sport Jet above a fan-shaped oat field</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/contour-hay-fallow-shelves.mp4"><img src="docs/assets/readme/contour-hay-fallow-shelves.gif" width="220" alt="Ultralight over hay and fallow shelves"></a><br><a href="docs/assets/current/contour-hay-fallow-shelves.png">Still</a><br>Ultralight over hay and fallow shelves</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/metro-green-inlet.mp4"><img src="docs/assets/readme/metro-green-inlet.gif" width="220" alt="Bush Plane over Green Inlet Steps"></a><br><a href="docs/assets/current/metro-green-inlet.png">Still</a><br>Bush Plane over Green Inlet Steps</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/metro-pocket-valley.mp4"><img src="docs/assets/readme/metro-pocket-valley.gif" width="220" alt="Glider over Pocket-valley Edge"></a><br><a href="docs/assets/current/metro-pocket-valley.png">Still</a><br>Glider over Pocket-valley Edge</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/metro-asymmetric-fan.mp4"><img src="docs/assets/readme/metro-asymmetric-fan.gif" width="220" alt="Ultralight over the Asymmetric Fan"></a><br><a href="docs/assets/current/metro-asymmetric-fan.png">Still</a><br>Ultralight over the Asymmetric Fan</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/metro-greenbelt-fork.mp4"><img src="docs/assets/readme/metro-greenbelt-fork.gif" width="220" alt="Racer over the Greenbelt Fork"></a><br><a href="docs/assets/current/metro-greenbelt-fork.png">Still</a><br>Racer over the Greenbelt Fork</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/hotel-skybridge-skyline.mp4"><img src="docs/assets/readme/hotel-skybridge-skyline.gif" width="220" alt="Ultralight approaching a hotel with a skybridge"></a><br><a href="docs/assets/current/hotel-skybridge-skyline.png">Still</a><br>Ultralight approaching a hotel with a skybridge</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/glass-bell-tower-skyline.mp4"><img src="docs/assets/readme/glass-bell-tower-skyline.gif" width="220" alt="Glider past a slanted glass bell tower"></a><br><a href="docs/assets/current/glass-bell-tower-skyline.png">Still</a><br>Glider past a slanted glass bell tower</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/hillside-civic-center-skyline.mp4"><img src="docs/assets/readme/hillside-civic-center-skyline.gif" width="220" alt="Racer approaching a hillside civic center"></a><br><a href="docs/assets/current/hillside-civic-center-skyline.png">Still</a><br>Racer approaching a hillside civic center</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/tiered-elevator-tower-skyline.mp4"><img src="docs/assets/readme/tiered-elevator-tower-skyline.gif" width="220" alt="Bush Plane above a tiered elevator tower"></a><br><a href="docs/assets/current/tiered-elevator-tower-skyline.png">Still</a><br>Bush Plane above a tiered elevator tower</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/paved-runway-double-arrow.mp4"><img src="docs/assets/readme/paved-runway-double-arrow.gif" width="220" alt="Racer crossing the runway 23 lead-in arrows"></a><br><a href="docs/assets/current/paved-runway-double-arrow.png">Still</a><br>Racer crossing the runway 23 lead-in arrows</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/paved-runway-aiming-blocks.mp4"><img src="docs/assets/readme/paved-runway-aiming-blocks.gif" width="220" alt="Glider approaching runway 08"></a><br><a href="docs/assets/current/paved-runway-aiming-blocks.png">Still</a><br>Glider approaching runway 08</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/grass-runway-displaced-threshold.mp4"><img src="docs/assets/readme/grass-runway-displaced-threshold.gif" width="220" alt="Bush Plane over a grass-strip threshold"></a><br><a href="docs/assets/current/grass-runway-displaced-threshold.png">Still</a><br>Bush Plane over a grass-strip threshold</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/tunnel-brick-keystone.mp4"><img src="docs/assets/readme/tunnel-brick-keystone.gif" width="220" alt="Brick keystone portal"></a><br><a href="docs/assets/current/tunnel-brick-keystone.png">Still</a><br>Brick keystone portal</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/tunnel-sandstone-buttresses.mp4"><img src="docs/assets/readme/tunnel-sandstone-buttresses.gif" width="220" alt="Sandstone buttressed portal"></a><br><a href="docs/assets/current/tunnel-sandstone-buttresses.png">Still</a><br>Sandstone buttressed portal</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/tunnel-limestone-double-ring.mp4"><img src="docs/assets/readme/tunnel-limestone-double-ring.gif" width="220" alt="Limestone double-ring portal"></a><br><a href="docs/assets/current/tunnel-limestone-double-ring.png">Still</a><br>Limestone double-ring portal</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/riverfront-covered-hide.mp4"><img src="docs/assets/readme/riverfront-covered-hide.gif" width="220" alt="Covered bird hide by the coast"></a><br><a href="docs/assets/current/riverfront-covered-hide.png">Still</a><br>Covered bird hide by the coast</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/riverfront-glazed-arcade.mp4"><img src="docs/assets/readme/riverfront-glazed-arcade.gif" width="220" alt="Glazed market arcade"></a><br><a href="docs/assets/current/riverfront-glazed-arcade.png">Still</a><br>Glazed market arcade</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/riverfront-twin-canopies.mp4"><img src="docs/assets/readme/riverfront-twin-canopies.gif" width="220" alt="Picnic canopies above the river"></a><br><a href="docs/assets/current/riverfront-twin-canopies.png">Still</a><br>Picnic canopies above the river</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/outer-block-coastal-flight.mp4"><img src="docs/assets/readme/outer-block-coastal-flight.gif" width="220" alt="The edge of a coastal neighborhood"></a><br><a href="docs/assets/current/outer-block-coastal-flight.png">Still</a><br>The edge of a coastal neighborhood</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/outer-block-courtyard-flight.mp4"><img src="docs/assets/readme/outer-block-courtyard-flight.gif" width="220" alt="Courtyards beyond downtown"></a><br><a href="docs/assets/current/outer-block-courtyard-flight.png">Still</a><br>Courtyards beyond downtown</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/rail-gravel-shoulders.mp4"><img src="docs/assets/readme/rail-gravel-shoulders.gif" width="220" alt="Gravel embankment along the foothill railway"></a><br><a href="docs/assets/current/rail-gravel-shoulders.png">Still</a><br>Gravel embankment along the foothill railway</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/rail-earthworks.mp4"><img src="docs/assets/readme/rail-earthworks.gif" width="220" alt="Rock cutting through the foothills"></a><br><a href="docs/assets/current/rail-earthworks.png">Still</a><br>Rock cutting through the foothills</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/airfield-country-service-pocket.mp4"><img src="docs/assets/readme/airfield-country-service-pocket.gif" width="220" alt="Bush Plane circling a country strip"></a><br><a href="docs/assets/current/airfield-country-service-pocket.png">Still</a><br>Bush Plane circling a country strip</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/airfield-light-aircraft-bays.mp4"><img src="docs/assets/readme/airfield-light-aircraft-bays.gif" width="220" alt="Glider over light-aircraft bays"></a><br><a href="docs/assets/current/airfield-light-aircraft-bays.png">Still</a><br>Glider over light-aircraft bays</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/airfield-visitor-stands.mp4"><img src="docs/assets/readme/airfield-visitor-stands.gif" width="220" alt="Racer approaching a regional airfield"></a><br><a href="docs/assets/current/airfield-visitor-stands.png">Still</a><br>Racer approaching a regional airfield</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/narrow-inlet-quarry.mp4"><img src="docs/assets/readme/narrow-inlet-quarry.gif" width="220" alt="Bush Plane above a narrow quarry entrance"></a><br><a href="docs/assets/current/narrow-inlet-quarry.png">Still</a><br>Bush Plane above a narrow quarry entrance</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/west-sweep-quarry.mp4"><img src="docs/assets/readme/west-sweep-quarry.gif" width="220" alt="Glider past a sweeping cut and haul road"></a><br><a href="docs/assets/current/west-sweep-quarry.png">Still</a><br>Glider past a sweeping cut and haul road</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/half-bowl-quarry.mp4"><img src="docs/assets/readme/half-bowl-quarry.gif" width="220" alt="Ultralight above a half-bowl quarry"></a><br><a href="docs/assets/current/half-bowl-quarry.png">Still</a><br>Ultralight above a half-bowl quarry</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/quay-boatyard-paint-hall.mp4"><img src="docs/assets/readme/quay-boatyard-paint-hall.gif" width="220" alt="Bush Plane along the boatyard promenade"></a><br><a href="docs/assets/current/quay-boatyard-paint-hall.png">Still</a><br>Bush Plane along the boatyard promenade</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/quay-twin-fish-sheds.mp4"><img src="docs/assets/readme/quay-twin-fish-sheds.gif" width="220" alt="Glider approaching twin fish sheds"></a><br><a href="docs/assets/current/quay-twin-fish-sheds.png">Still</a><br>Glider approaching twin fish sheds</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/quay-customs-annex.mp4"><img src="docs/assets/readme/quay-customs-annex.gif" width="220" alt="Ultralight past the harbor customs annex"></a><br><a href="docs/assets/current/quay-customs-annex.png">Still</a><br>Ultralight past the harbor customs annex</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/market-gardens-below-town.mp4"><img src="docs/assets/readme/market-gardens-below-town.gif" width="220" alt="Ultralight over market gardens below town"></a><br><a href="docs/assets/current/market-gardens-below-town.png">Still</a><br>Ultralight over market gardens below town</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/coastal-stubble-fields.mp4"><img src="docs/assets/readme/coastal-stubble-fields.gif" width="220" alt="Bush Plane over coastal stubble fields"></a><br><a href="docs/assets/current/coastal-stubble-fields.png">Still</a><br>Bush Plane over coastal stubble fields</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/grassy-field-setbacks.mp4"><img src="docs/assets/readme/grassy-field-setbacks.gif" width="220" alt="Glider over fields with grassy setbacks"></a><br><a href="docs/assets/current/grassy-field-setbacks.png">Still</a><br>Glider over fields with grassy setbacks</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/raised-council-building-skyline.mp4"><img src="docs/assets/readme/raised-council-building-skyline.gif" width="220" alt="Bush Plane approaching a raised council building"></a><br><a href="docs/assets/current/raised-council-building-skyline.png">Still</a><br>Bush Plane approaching a raised council building</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/terraced-library-skyline.mp4"><img src="docs/assets/readme/terraced-library-skyline.gif" width="220" alt="Ultralight approaching a terraced library"></a><br><a href="docs/assets/current/terraced-library-skyline.png">Still</a><br>Ultralight approaching a terraced library</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/glass-tower-skyline.mp4"><img src="docs/assets/readme/glass-tower-skyline.gif" width="220" alt="Glider approaching a glass tower"></a><br><a href="docs/assets/current/glass-tower-skyline.png">Still</a><br>Glider approaching a glass tower</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/skybridge-towers-skyline.mp4"><img src="docs/assets/readme/skybridge-towers-skyline.gif" width="220" alt="Racer approaching twin towers with a skybridge"></a><br><a href="docs/assets/current/skybridge-towers-skyline.png">Still</a><br>Racer approaching twin towers with a skybridge</td>
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
    <td align="center"><a href="docs/assets/current/aggregate-service.png"><img src="docs/assets/current/aggregate-service.png" width="220" alt="Loaded gravel train and empty return"></a><br>Loaded gravel train and empty return</td>
    <td align="center"><a href="docs/assets/current/gravel-freight.png"><img src="docs/assets/current/gravel-freight.png" width="220" alt="Gravel pit and aggregate freight"></a><br>Gravel pit and aggregate freight</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/current/contour-fields.png"><img src="docs/assets/current/contour-fields.png" width="220" alt="Contour strips and grass buffers"></a><br>Contour strips and grass buffers</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/grain-terminal.gif"><img src="docs/assets/readme/grain-terminal.gif" width="220" alt="Rural grain elevator and hopper train"></a><br><a href="docs/assets/current/grain-terminal.png">Still</a><br>Rural grain elevator and hopper train</td>
    <td align="center"><a href="docs/assets/current/freight-loading.png"><img src="docs/assets/current/freight-loading.png" width="220" alt="Gravel and container loading aprons"></a><br>Gravel and container loading aprons</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/current/freight-platforms.png"><img src="docs/assets/current/freight-platforms.png" width="220" alt="Rail loading apron and inland transfer shed"></a><br>Rail loading apron and inland transfer shed</td>
    <td align="center"><a href="docs/assets/current/freight-steel-coils.png"><img src="docs/assets/current/freight-steel-coils.png" width="220" alt="Steel coils and machinery freight"></a><br>Steel coils and machinery freight</td>
    <td align="center"><a href="docs/assets/current/freight-cold-storage.png"><img src="docs/assets/current/freight-cold-storage.png" width="220" alt="Frozen-food reefers"></a><br>Frozen-food reefers</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/current/freight-tankers.png"><img src="docs/assets/current/freight-tankers.png" width="220" alt="Food-oil tank cars"></a><br>Food-oil tank cars</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/freight-containers.gif"><img src="docs/assets/readme/freight-containers.gif" width="220" alt="Container train below the Ultralight"></a><br><a href="docs/assets/current/freight-containers.png">Still</a><br>Container train below the Ultralight</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/ferry-terminal.mp4"><img src="docs/assets/readme/ferry-terminal.gif" width="220" alt="Ferry terminal"></a><br><a href="docs/assets/current/ferry-terminal.png">Still</a><br>Ferry terminal</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/marina-piers.mp4"><img src="docs/assets/readme/marina-piers.gif" width="220" alt="Marina and sailboats"></a><br><a href="docs/assets/current/marina-piers.png">Still</a><br>Marina and sailboats</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/city-approach.mp4"><img src="docs/assets/readme/city-approach.gif" width="220" alt="Racer approaching the skyline"></a><br><a href="docs/assets/current/city-approach.png">Still</a><br>Racer approaching the skyline</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/night-neighborhoods.mp4"><img src="docs/assets/readme/night-neighborhoods.gif" width="220" alt="Ultralight over the city at night"></a><br><a href="docs/assets/current/night-neighborhoods.png">Still</a><br>Ultralight over the city at night</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/neighborhood-avenue.mp4"><img src="docs/assets/readme/neighborhood-avenue.gif" width="220" alt="Racer over the outer neighborhoods"></a><br><a href="docs/assets/current/neighborhood-avenue.png">Still</a><br>Racer over the outer neighborhoods</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/country-strip-pass.mp4"><img src="docs/assets/readme/country-strip-pass.gif" width="220" alt="Ultralight over a country airstrip"></a><br><a href="docs/assets/current/country-strip-pass.png">Still</a><br>Ultralight over a country airstrip</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/regional-airfield-apron.mp4"><img src="docs/assets/readme/regional-airfield-apron.gif" width="220" alt="Ultralight by the regional apron"></a><br><a href="docs/assets/current/regional-airfield-apron.png">Still</a><br>Ultralight by the regional apron</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/vegas-fremont.mp4"><img src="docs/assets/readme/vegas-fremont.gif" width="220" alt="Viva Vision lights over Fremont Street"></a><br><a href="docs/assets/current/vegas-fremont.png">Still</a><br>Viva Vision lights over Fremont Street</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/vegas-stadium.mp4"><img src="docs/assets/readme/vegas-stadium.gif" width="220" alt="Sport Jet over Allegiant Stadium"></a><br><a href="docs/assets/current/vegas-stadium.png">Still</a><br>Sport Jet over Allegiant Stadium</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/vegas-venetian.mp4"><img src="docs/assets/readme/vegas-venetian.gif" width="220" alt="Sport Jet past the Venetian and Palazzo"></a><br><a href="docs/assets/current/vegas-venetian.png">Still</a><br>Sport Jet past the Venetian and Palazzo</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/vegas-high-roller.mp4"><img src="docs/assets/readme/vegas-high-roller.gif" width="220" alt="Sport Jet past the High Roller"></a><br><a href="docs/assets/current/vegas-high-roller.png">Still</a><br>Sport Jet past the High Roller</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/vegas-paris.mp4"><img src="docs/assets/readme/vegas-paris.gif" width="220" alt="Sport Jet past the Paris Las Vegas Eiffel Tower"></a><br><a href="docs/assets/current/vegas-paris.png">Still</a><br>Sport Jet past the Paris Las Vegas Eiffel Tower</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/vegas-downtown.mp4"><img src="docs/assets/readme/vegas-downtown.gif" width="220" alt="SR-71 above Downtown Las Vegas"></a><br><a href="docs/assets/current/vegas-downtown.png">Still</a><br>SR-71 above Downtown Las Vegas</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/vegas-airport.mp4"><img src="docs/assets/readme/vegas-airport.gif" width="220" alt="SR-71 above Harry Reid Airport"></a><br><a href="docs/assets/current/vegas-airport.png">Still</a><br>SR-71 above Harry Reid Airport</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/vegas-strip.mp4"><img src="docs/assets/readme/vegas-strip.gif" width="220" alt="Racer along the Las Vegas Strip"></a><br><a href="docs/assets/current/vegas-strip.png">Still</a><br>Racer along the Las Vegas Strip</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/la-union-station.mp4"><img src="docs/assets/readme/la-union-station.gif" width="220" alt="Sport Jet approaching Union Station"></a><br><a href="docs/assets/current/la-union-station.png">Still</a><br>Sport Jet approaching Union Station</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/la-exposition-park.mp4"><img src="docs/assets/readme/la-exposition-park.gif" width="220" alt="Glider near Exposition Park"></a><br><a href="docs/assets/current/la-exposition-park.png">Still</a><br>Glider near Exposition Park</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/la-dodger-stadium.mp4"><img src="docs/assets/readme/la-dodger-stadium.gif" width="220" alt="Bush Plane near Dodger Stadium"></a><br><a href="docs/assets/current/la-dodger-stadium.png">Still</a><br>Bush Plane near Dodger Stadium</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/la-inglewood.mp4"><img src="docs/assets/readme/la-inglewood.gif" width="220" alt="Sport Jet approaching SoFi Stadium"></a><br><a href="docs/assets/current/la-inglewood.png">Still</a><br>Sport Jet approaching SoFi Stadium</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/la-pasadena.mp4"><img src="docs/assets/readme/la-pasadena.gif" width="220" alt="Glider approaching Pasadena"></a><br><a href="docs/assets/current/la-pasadena.png">Still</a><br>Glider approaching Pasadena</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/la-hollywood.mp4"><img src="docs/assets/readme/la-hollywood.gif" width="220" alt="Racer over Hollywood"></a><br><a href="docs/assets/current/la-hollywood.png">Still</a><br>Racer over Hollywood</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/la-getty-center.mp4"><img src="docs/assets/readme/la-getty-center.gif" width="220" alt="Racer near Getty Center"></a><br><a href="docs/assets/current/la-getty-center.png">Still</a><br>Racer near Getty Center</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/la-venice-beach.mp4"><img src="docs/assets/readme/la-venice-beach.gif" width="220" alt="Glider on the Venice Beach coast"></a><br><a href="docs/assets/current/la-venice-beach.png">Still</a><br>Glider on the Venice Beach coast</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/readme/southwest-primm.gif"><img src="docs/assets/readme/southwest-primm.gif" width="220" alt="Bush Plane over Primm"></a><br><a href="docs/assets/current/southwest-primm.png">Still</a><br>Bush Plane over Primm</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/readme/southwest-baker.gif"><img src="docs/assets/readme/southwest-baker.gif" width="220" alt="Ultralight over Baker"></a><br><a href="docs/assets/current/southwest-baker.png">Still</a><br>Ultralight over Baker</td>
  </tr>
</table>

### Aircraft

<table>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/cub-valley.mp4"><img src="docs/assets/readme/cub-valley.gif" width="220" alt="Bush Plane following a mountain valley"></a><br><a href="docs/assets/current/cub-valley.png">Still</a><br>Bush Plane following a mountain valley</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/sport-jet-climb.mp4"><img src="docs/assets/readme/sport-jet-climb.gif" width="220" alt="Sport Jet climbing through a mountain valley"></a><br><a href="docs/assets/current/sport-jet-climb.png">Still</a><br>Sport Jet climbing through a mountain valley</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/racer-bank.mp4"><img src="docs/assets/readme/racer-bank.gif" width="220" alt="Racer banks around a ridge"></a><br><a href="docs/assets/current/racer-bank.png">Still</a><br>Racer banks around a ridge</td>
  </tr>
  <tr>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/glider-reflection.gif"><img src="docs/assets/readme/glider-reflection.gif" width="220" alt="Sun glint in a thermal turn"></a><br><a href="docs/assets/current/glider-reflection.png">Still</a><br>Sun glint in a thermal turn</td>
    <td align="center"><a href="https://xberenguer.github.io/scenic-flight/assets/current/high-cruise.mp4"><img src="docs/assets/readme/high-cruise.gif" width="220" alt="High-altitude cruise"></a><br><a href="docs/assets/current/high-cruise.png">Still</a><br>High-altitude cruise</td>
  </tr>
</table>

### Interface

<table>
  <tr>
    <td align="center"><a href="docs/assets/current/flight-info.png"><img src="docs/assets/current/flight-info.png" width="220" alt="Flight information overlay"></a><br>Flight information overlay</td>
    <td align="center"><a href="docs/assets/current/new-world-ui.png"><img src="docs/assets/current/new-world-ui.png" width="220" alt="New World hold progress"></a><br>New World hold progress</td>
  </tr>
</table>

## Aircraft and picker screens

<table>
  <tr>
    <td align="center"><a href="docs/assets/picker-7.png"><img src="docs/assets/picker-7.png" width="168" alt="Bald Eagle picker screen"></a><br><a href="docs/assets/turntable-7.gif"><img src="docs/assets/turntable-7.gif" width="168" alt="Bald Eagle turntable"></a><br>Bald Eagle</td>
    <td align="center"><a href="docs/assets/picker-3.png"><img src="docs/assets/picker-3.png" width="168" alt="Ultralight picker screen"></a><br><a href="docs/assets/turntable-3.gif"><img src="docs/assets/turntable-3.gif" width="168" alt="Ultralight turntable"></a><br>Ultralight</td>
    <td align="center"><a href="docs/assets/picker-4.png"><img src="docs/assets/picker-4.png" width="168" alt="Glider picker screen"></a><br><a href="docs/assets/turntable-4.gif"><img src="docs/assets/turntable-4.gif" width="168" alt="Glider turntable"></a><br>Glider</td>
    <td align="center"><a href="docs/assets/picker-0.png"><img src="docs/assets/picker-0.png" width="168" alt="Bush Plane picker screen"></a><br><a href="docs/assets/turntable-0.gif"><img src="docs/assets/turntable-0.gif" width="168" alt="Bush Plane turntable"></a><br>Bush Plane</td>
  </tr>
</table>

<table>
  <tr>
    <td align="center"><a href="docs/assets/picker-1.png"><img src="docs/assets/picker-1.png" width="168" alt="Racer picker screen"></a><br><a href="docs/assets/turntable-1.gif"><img src="docs/assets/turntable-1.gif" width="168" alt="Racer turntable"></a><br>Racer</td>
    <td align="center"><a href="docs/assets/picker-2.png"><img src="docs/assets/picker-2.png" width="168" alt="Sport Jet picker screen"></a><br><a href="docs/assets/turntable-2.gif"><img src="docs/assets/turntable-2.gif" width="168" alt="Sport Jet turntable"></a><br>Sport Jet</td>
    <td align="center"><a href="docs/assets/picker-5.png"><img src="docs/assets/picker-5.png" width="168" alt="F/A-18 Fighter picker screen"></a><br><a href="docs/assets/turntable-5.gif"><img src="docs/assets/turntable-5.gif" width="168" alt="F/A-18 Fighter turntable"></a><br>F/A-18 Fighter</td>
    <td align="center"><a href="docs/assets/picker-6.png"><img src="docs/assets/picker-6.png" width="168" alt="SR-71 Blackbird picker screen"></a><br><a href="docs/assets/turntable-6.gif"><img src="docs/assets/turntable-6.gif" width="168" alt="SR-71 Blackbird turntable"></a><br>SR-71 Blackbird</td>
  </tr>
</table>

## Rare flight moments

Each flight has a two-in-three chance at a rare moment every 4 min 20 sec. Events last 9.5–14.5 simulated seconds.

### Bald Eagle

Bald Eagle crossing · Talon stretch · Tail fan · Ridge wing tuck

<a href="docs/assets/artwork/flight-moments-7.gif"><img src="docs/assets/artwork/flight-moments-7.gif" width="720" alt="Bald Eagle rare flight moments. Bald Eagle crossing · Talon stretch · Tail fan · Ridge wing tuck"></a>

### Ultralight

Hot-air balloon pass · Ultralight pair · Sailplane passing overhead · Valley lookout turn

<a href="docs/assets/artwork/flight-moments-3.gif"><img src="docs/assets/artwork/flight-moments-3.gif" width="720" alt="Ultralight rare flight moments. Hot-air balloon pass · Ultralight pair · Sailplane passing overhead · Valley lookout turn"></a>

### Glider

Towplane and sailplane crossing · Sailplane joins alongside · Airbrake descent · Slow sideslip

<a href="docs/assets/artwork/flight-moments-4.gif"><img src="docs/assets/artwork/flight-moments-4.gif" width="720" alt="Glider rare flight moments. Towplane and sailplane crossing · Sailplane joins alongside · Airbrake descent · Slow sideslip"></a>

### Bush Plane

Biplane crossing · Bush-plane escort · Survey S-turn · Landing light pass

<a href="docs/assets/artwork/flight-moments-0.gif"><img src="docs/assets/artwork/flight-moments-0.gif" width="720" alt="Bush Plane rare flight moments. Biplane crossing · Bush-plane escort · Survey S-turn · Landing light pass"></a>

### Racer

Aerobatic pair · Wing wag · Straight smoke pass · Rising smoke arc

<a href="docs/assets/artwork/flight-moments-1.gif"><img src="docs/assets/artwork/flight-moments-1.gif" width="720" alt="Racer rare flight moments. Aerobatic pair · Wing wag · Straight smoke pass · Rising smoke arc"></a>

### Sport Jet

Business jet crossing · Wingtip vapor · Arrival lights · Long viewing turn

<a href="docs/assets/artwork/flight-moments-2.gif"><img src="docs/assets/artwork/flight-moments-2.gif" width="720" alt="Sport Jet rare flight moments. Business jet crossing · Wingtip vapor · Arrival lights · Long viewing turn"></a>

### F/A-18 Fighter

Fighter high pass · Section break · Condensation streamers · Shallow bank reversal

<a href="docs/assets/artwork/flight-moments-5.gif"><img src="docs/assets/artwork/flight-moments-5.gif" width="720" alt="F/A-18 Fighter rare flight moments. Fighter high pass · Section break · Condensation streamers · Shallow bank reversal"></a>

### SR-71 Blackbird

Twin high-altitude contrails · Distant Blackbird crossing · Airliner below · Cruise pitch adjustment

<a href="docs/assets/artwork/flight-moments-6.gif"><img src="docs/assets/artwork/flight-moments-6.gif" width="720" alt="SR-71 Blackbird rare flight moments. Twin high-altitude contrails · Distant Blackbird crossing · Airliner below · Cruise pitch adjustment"></a>

## Artwork gallery

[Weather artwork](#weather-artwork) · [Mountains and ice](#mountains-and-ice) · [Rivers and lakes](#rivers-and-lakes) · [Coasts](#coasts) · [Deserts and badlands](#deserts-and-badlands) · [Vegetation and ground cover](#vegetation-and-ground-cover) · [Farms and industry](#farms-and-industry) · [Cities and towns](#cities-and-towns) · [Roads and mountain routes](#roads-and-mountain-routes) · [Railways](#railways) · [Airfields](#airfields) · [Ports and vessels](#ports-and-vessels)

### Weather artwork

#### Animated snowfall · 25 variants

<a href="docs/assets/artwork/snowfall-profiles.gif"><img src="docs/assets/artwork/snowfall-profiles.gif" width="780" alt="Animated snowfall"></a>

### Mountains and ice

#### Alpine glaciers · 25 variants

<a href="docs/assets/artwork/glaciers.png"><img src="docs/assets/artwork/glaciers.png" width="780" alt="Alpine glaciers"></a>

<a href="docs/assets/artwork/glaciers.gif"><img src="docs/assets/artwork/glaciers.gif" width="550" alt="Alpine glaciers animated atlas"></a>

#### Glacial moraines · 25 variants

<a href="docs/assets/artwork/moraines.png"><img src="docs/assets/artwork/moraines.png" width="780" alt="Glacial moraines"></a>

<a href="docs/assets/artwork/moraines.gif"><img src="docs/assets/artwork/moraines.gif" width="550" alt="Glacial moraines animated atlas"></a>

#### Glacial outwash fans · 25 variants

<a href="docs/assets/artwork/glacial-outwash.png"><img src="docs/assets/artwork/glacial-outwash.png" width="780" alt="Glacial outwash fans"></a>

<a href="docs/assets/artwork/glacial-outwash.gif"><img src="docs/assets/artwork/glacial-outwash.gif" width="550" alt="Glacial outwash fans animated atlas"></a>

#### Alpine rockfall fans · 25 variants

<a href="docs/assets/artwork/talus-fans.png"><img src="docs/assets/artwork/talus-fans.png" width="780" alt="Alpine rockfall fans"></a>

<a href="docs/assets/artwork/talus-fans.gif"><img src="docs/assets/artwork/talus-fans.gif" width="550" alt="Alpine rockfall fans animated atlas"></a>

#### Columnar rock faces · 25 variants

<a href="docs/assets/artwork/columnar-headwalls.png"><img src="docs/assets/artwork/columnar-headwalls.png" width="780" alt="Columnar rock faces"></a>

<a href="docs/assets/artwork/columnar-headwalls.gif"><img src="docs/assets/artwork/columnar-headwalls.gif" width="550" alt="Columnar rock faces animated atlas"></a>

#### Mountain details · 25 variants

<a href="docs/assets/artwork/mountain-details.png"><img src="docs/assets/artwork/mountain-details.png" width="780" alt="Mountain details"></a>

<a href="docs/assets/artwork/mountain-details.gif"><img src="docs/assets/artwork/mountain-details.gif" width="550" alt="Mountain details animated atlas"></a>

#### Alpine ground details · 25 variants

<a href="docs/assets/artwork/alpine-ground-details.png"><img src="docs/assets/artwork/alpine-ground-details.png" width="780" alt="Alpine ground details"></a>

<a href="docs/assets/artwork/alpine-ground-details.gif"><img src="docs/assets/artwork/alpine-ground-details.gif" width="550" alt="Alpine ground details animated atlas"></a>

### Rivers and lakes

#### Mountain lake shores · 25 variants

<a href="docs/assets/artwork/tarns.png"><img src="docs/assets/artwork/tarns.png" width="780" alt="Mountain lake shores"></a>

<a href="docs/assets/artwork/tarns.gif"><img src="docs/assets/artwork/tarns.gif" width="550" alt="Mountain lake shores animated atlas"></a>

#### Alpine cascades · 25 variants

<a href="docs/assets/artwork/alpine-cascades.png"><img src="docs/assets/artwork/alpine-cascades.png" width="780" alt="Alpine cascades"></a>

<a href="docs/assets/artwork/alpine-cascades.gif"><img src="docs/assets/artwork/alpine-cascades.gif" width="550" alt="Alpine cascades animated atlas"></a>

#### River islands · 25 variants

<a href="docs/assets/artwork/river-islands.png"><img src="docs/assets/artwork/river-islands.png" width="780" alt="River islands"></a>

<a href="docs/assets/artwork/river-islands.gif"><img src="docs/assets/artwork/river-islands.gif" width="550" alt="River islands animated atlas"></a>

#### River bend point bars · 25 variants

<a href="docs/assets/artwork/point-bars.png"><img src="docs/assets/artwork/point-bars.png" width="780" alt="River bend point bars"></a>

<a href="docs/assets/artwork/point-bars.gif"><img src="docs/assets/artwork/point-bars.gif" width="550" alt="River bend point bars animated atlas"></a>

#### Floodplain details · 25 variants

<a href="docs/assets/artwork/floodplain-details.png"><img src="docs/assets/artwork/floodplain-details.png" width="780" alt="Floodplain details"></a>

<a href="docs/assets/artwork/floodplain-details.gif"><img src="docs/assets/artwork/floodplain-details.gif" width="550" alt="Floodplain details animated atlas"></a>

#### Freshwater details · 25 variants

<a href="docs/assets/artwork/freshwater-details.png"><img src="docs/assets/artwork/freshwater-details.png" width="780" alt="Freshwater details"></a>

<a href="docs/assets/artwork/freshwater-details.gif"><img src="docs/assets/artwork/freshwater-details.gif" width="550" alt="Freshwater details animated atlas"></a>

### Coasts

#### Coastal bluffs · 25 variants

<a href="docs/assets/artwork/coastal-bluffs.png"><img src="docs/assets/artwork/coastal-bluffs.png" width="780" alt="Coastal bluffs"></a>

<a href="docs/assets/artwork/coastal-bluffs.gif"><img src="docs/assets/artwork/coastal-bluffs.gif" width="550" alt="Coastal bluffs animated atlas"></a>

#### Coastal rock formations · 25 variants

<a href="docs/assets/artwork/coastal-rocks.png"><img src="docs/assets/artwork/coastal-rocks.png" width="780" alt="Coastal rock formations"></a>

<a href="docs/assets/artwork/coastal-rocks.gif"><img src="docs/assets/artwork/coastal-rocks.gif" width="550" alt="Coastal rock formations animated atlas"></a>

#### Sheltered cove dunes · 25 variants

<a href="docs/assets/artwork/cove-dunes.png"><img src="docs/assets/artwork/cove-dunes.png" width="780" alt="Sheltered cove dunes"></a>

<a href="docs/assets/artwork/cove-dunes.gif"><img src="docs/assets/artwork/cove-dunes.gif" width="550" alt="Sheltered cove dunes animated atlas"></a>

#### Headland details · 25 variants

<a href="docs/assets/artwork/headland-details.png"><img src="docs/assets/artwork/headland-details.png" width="780" alt="Headland details"></a>

<a href="docs/assets/artwork/headland-details.gif"><img src="docs/assets/artwork/headland-details.gif" width="550" alt="Headland details animated atlas"></a>

#### Coastal details · 25 variants

<a href="docs/assets/artwork/coastal-details.png"><img src="docs/assets/artwork/coastal-details.png" width="780" alt="Coastal details"></a>

<a href="docs/assets/artwork/coastal-details.gif"><img src="docs/assets/artwork/coastal-details.gif" width="550" alt="Coastal details animated atlas"></a>

### Deserts and badlands

#### Scrub, gravel bars, dunes, sandstone, and salt crust · 25 variants

<a href="docs/assets/artwork/dry-surfaces.png"><img src="docs/assets/artwork/dry-surfaces.png" width="780" alt="Scrub, gravel bars, dunes, sandstone, and salt crust"></a>

#### Foothill rock and plant groups · 25 variants

<a href="docs/assets/artwork/dry-foothill-models.png"><img src="docs/assets/artwork/dry-foothill-models.png" width="780" alt="Foothill rock and plant groups"></a>

<a href="docs/assets/artwork/dry-foothill-models.gif"><img src="docs/assets/artwork/dry-foothill-models.gif" width="550" alt="Foothill rock and plant groups animated atlas"></a>

#### Sandstone ridges · 25 variants

<a href="docs/assets/artwork/sandstone-ridges.png"><img src="docs/assets/artwork/sandstone-ridges.png" width="780" alt="Sandstone ridges"></a>

<a href="docs/assets/artwork/sandstone-ridges.gif"><img src="docs/assets/artwork/sandstone-ridges.gif" width="550" alt="Sandstone ridges animated atlas"></a>

#### Badlands ravines · 25 variants

<a href="docs/assets/artwork/badlands-ravines.png"><img src="docs/assets/artwork/badlands-ravines.png" width="780" alt="Badlands ravines"></a>

<a href="docs/assets/artwork/badlands-ravines.gif"><img src="docs/assets/artwork/badlands-ravines.gif" width="550" alt="Badlands ravines animated atlas"></a>

#### Sandstone mesas · 25 variants

<a href="docs/assets/artwork/mesa-profiles.png"><img src="docs/assets/artwork/mesa-profiles.png" width="780" alt="Sandstone mesas"></a>

<a href="docs/assets/artwork/mesa-profiles.gif"><img src="docs/assets/artwork/mesa-profiles.gif" width="550" alt="Sandstone mesas animated atlas"></a>

#### Desert alluvial fans · 25 variants

<a href="docs/assets/artwork/alluvial-fans.png"><img src="docs/assets/artwork/alluvial-fans.png" width="780" alt="Desert alluvial fans"></a>

<a href="docs/assets/artwork/alluvial-fans.gif"><img src="docs/assets/artwork/alluvial-fans.gif" width="550" alt="Desert alluvial fans animated atlas"></a>

#### Dryland details · 25 variants

<a href="docs/assets/artwork/dryland-details.png"><img src="docs/assets/artwork/dryland-details.png" width="780" alt="Dryland details"></a>

<a href="docs/assets/artwork/dryland-details.gif"><img src="docs/assets/artwork/dryland-details.gif" width="550" alt="Dryland details animated atlas"></a>

### Vegetation and ground cover

#### Trees · 256 variants

<a href="docs/assets/artwork/trees.png"><img src="docs/assets/artwork/trees.png" width="780" alt="Trees"></a>

#### Bushes · 256 variants

<a href="docs/assets/artwork/bushes.png"><img src="docs/assets/artwork/bushes.png" width="780" alt="Bushes"></a>

#### Cacti · 256 variants

<a href="docs/assets/artwork/cacti.png"><img src="docs/assets/artwork/cacti.png" width="780" alt="Cacti"></a>

#### Wetland plants · 256 variants

<a href="docs/assets/artwork/wetlands.png"><img src="docs/assets/artwork/wetlands.png" width="780" alt="Wetland plants"></a>

#### Woodland details · 25 variants

<a href="docs/assets/artwork/woodland-details.png"><img src="docs/assets/artwork/woodland-details.png" width="780" alt="Woodland details"></a>

<a href="docs/assets/artwork/woodland-details.gif"><img src="docs/assets/artwork/woodland-details.gif" width="550" alt="Woodland details animated atlas"></a>

#### Grassland details · 25 variants

<a href="docs/assets/artwork/grassland-details.png"><img src="docs/assets/artwork/grassland-details.png" width="780" alt="Grassland details"></a>

<a href="docs/assets/artwork/grassland-details.gif"><img src="docs/assets/artwork/grassland-details.gif" width="550" alt="Grassland details animated atlas"></a>

#### Rocks · 256 variants

<a href="docs/assets/artwork/rocks.png"><img src="docs/assets/artwork/rocks.png" width="780" alt="Rocks"></a>

#### Habitat details · 30 variants

<a href="docs/assets/artwork/habitat-details.png"><img src="docs/assets/artwork/habitat-details.png" width="780" alt="Habitat details"></a>

### Farms and industry

#### Metropolitan outskirts · 25 variants

<a href="docs/assets/artwork/metropolitan-outskirts.png"><img src="docs/assets/artwork/metropolitan-outskirts.png" width="780" alt="Metropolitan outskirts"></a>

#### Cultivated fields · 25 variants

<a href="docs/assets/artwork/cultivated-fields.png"><img src="docs/assets/artwork/cultivated-fields.png" width="780" alt="Cultivated fields"></a>

<a href="docs/assets/artwork/cultivated-fields.gif"><img src="docs/assets/artwork/cultivated-fields.gif" width="550" alt="Cultivated fields animated atlas"></a>

#### Contour fields · 25 variants

<a href="docs/assets/artwork/contour-fields.png"><img src="docs/assets/artwork/contour-fields.png" width="780" alt="Contour fields"></a>

<a href="docs/assets/artwork/contour-fields.gif"><img src="docs/assets/artwork/contour-fields.gif" width="550" alt="Contour fields animated atlas"></a>

#### Farm details · 25 variants

<a href="docs/assets/artwork/farm-details.png"><img src="docs/assets/artwork/farm-details.png" width="780" alt="Farm details"></a>

<a href="docs/assets/artwork/farm-details.gif"><img src="docs/assets/artwork/farm-details.gif" width="550" alt="Farm details animated atlas"></a>

#### Countryside buildings · 25 variants

<a href="docs/assets/artwork/countryside-buildings.png"><img src="docs/assets/artwork/countryside-buildings.png" width="780" alt="Countryside buildings"></a>

<a href="docs/assets/artwork/countryside-buildings.gif"><img src="docs/assets/artwork/countryside-buildings.gif" width="550" alt="Countryside buildings animated atlas"></a>

#### Hillside quarries · 25 variants

<a href="docs/assets/artwork/quarry-profiles.png"><img src="docs/assets/artwork/quarry-profiles.png" width="780" alt="Hillside quarries"></a>

<a href="docs/assets/artwork/quarry-profiles.gif"><img src="docs/assets/artwork/quarry-profiles.gif" width="550" alt="Hillside quarries animated atlas"></a>

#### Solar array layouts · 25 variants

<a href="docs/assets/artwork/solar-layouts.png"><img src="docs/assets/artwork/solar-layouts.png" width="780" alt="Solar array layouts"></a>

<a href="docs/assets/artwork/solar-layouts.gif"><img src="docs/assets/artwork/solar-layouts.gif" width="550" alt="Solar array layouts animated atlas"></a>

#### Industrial landmarks · 25 variants

<a href="docs/assets/artwork/industrial-landmarks.png"><img src="docs/assets/artwork/industrial-landmarks.png" width="780" alt="Industrial landmarks"></a>

<a href="docs/assets/artwork/industrial-landmarks.gif"><img src="docs/assets/artwork/industrial-landmarks.gif" width="550" alt="Industrial landmarks animated atlas"></a>

### Cities and towns

#### Metropolitan building blocks · 25 variants

<a href="docs/assets/artwork/metropolis-blocks.png"><img src="docs/assets/artwork/metropolis-blocks.png" width="780" alt="Metropolitan building blocks"></a>

#### Highway neighborhoods · 25 variants

<a href="docs/assets/artwork/frontage-layouts.png"><img src="docs/assets/artwork/frontage-layouts.png" width="780" alt="Highway neighborhoods"></a>

#### Metropolitan neighborhood plans · 25 variants

<a href="docs/assets/artwork/metro-layouts.png"><img src="docs/assets/artwork/metro-layouts.png" width="780" alt="Metropolitan neighborhood plans"></a>

#### Outer neighborhood blocks · 25 variants

<a href="docs/assets/artwork/outer-blocks.png"><img src="docs/assets/artwork/outer-blocks.png" width="780" alt="Outer neighborhood blocks"></a>

<a href="docs/assets/artwork/outer-blocks.gif"><img src="docs/assets/artwork/outer-blocks.gif" width="550" alt="Outer neighborhood blocks animated atlas"></a>

#### Neighborhood buildings · 27 variants

<a href="docs/assets/artwork/neighborhood-buildings.png"><img src="docs/assets/artwork/neighborhood-buildings.png" width="780" alt="Neighborhood buildings"></a>

#### Street buildings · 25 variants

<a href="docs/assets/artwork/street-buildings.png"><img src="docs/assets/artwork/street-buildings.png" width="780" alt="Street buildings"></a>

<a href="docs/assets/artwork/street-buildings.gif"><img src="docs/assets/artwork/street-buildings.gif" width="550" alt="Street buildings animated atlas"></a>

#### Town landmarks · 25 variants

<a href="docs/assets/artwork/town-landmarks.png"><img src="docs/assets/artwork/town-landmarks.png" width="780" alt="Town landmarks"></a>

<a href="docs/assets/artwork/town-landmarks.gif"><img src="docs/assets/artwork/town-landmarks.gif" width="550" alt="Town landmarks animated atlas"></a>

#### Riverfront buildings · 25 variants

<a href="docs/assets/artwork/riverfront-buildings.png"><img src="docs/assets/artwork/riverfront-buildings.png" width="780" alt="Riverfront buildings"></a>

<a href="docs/assets/artwork/riverfront-buildings.gif"><img src="docs/assets/artwork/riverfront-buildings.gif" width="550" alt="Riverfront buildings animated atlas"></a>

#### Municipal water towers · 25 variants

<a href="docs/assets/artwork/water-towers.png"><img src="docs/assets/artwork/water-towers.png" width="780" alt="Municipal water towers"></a>

<a href="docs/assets/artwork/water-towers.gif"><img src="docs/assets/artwork/water-towers.gif" width="550" alt="Municipal water towers animated atlas"></a>

#### City towers · 250 variants

<a href="docs/assets/artwork/towers.png"><img src="docs/assets/artwork/towers.png" width="780" alt="City towers"></a>

#### Skyline landmarks · 100 variants

<a href="docs/assets/artwork/skyline-landmarks.png"><img src="docs/assets/artwork/skyline-landmarks.png" width="780" alt="Skyline landmarks"></a>

<a href="docs/assets/artwork/skyline-landmarks.gif"><img src="docs/assets/artwork/skyline-landmarks.gif" width="550" alt="Skyline landmarks animated atlas"></a>

### Roads and mountain routes

#### Road layouts between towns · 26 variants

<a href="docs/assets/artwork/town-road-alignments.png"><img src="docs/assets/artwork/town-road-alignments.png" width="780" alt="Road layouts between towns"></a>

#### River and coastal bridges · 25 variants

<a href="docs/assets/artwork/bridges.png"><img src="docs/assets/artwork/bridges.png" width="780" alt="River and coastal bridges"></a>

<a href="docs/assets/artwork/bridges.gif"><img src="docs/assets/artwork/bridges.gif" width="550" alt="River and coastal bridges animated atlas"></a>

#### Road tunnel portals · 25 variants

<a href="docs/assets/artwork/tunnel-facades.png"><img src="docs/assets/artwork/tunnel-facades.png" width="780" alt="Road tunnel portals"></a>

<a href="docs/assets/artwork/tunnel-facades.gif"><img src="docs/assets/artwork/tunnel-facades.gif" width="550" alt="Road tunnel portals animated atlas"></a>

#### Roadside models · 30 variants

<a href="docs/assets/artwork/roadside-models.png"><img src="docs/assets/artwork/roadside-models.png" width="780" alt="Roadside models"></a>

<a href="docs/assets/artwork/roadside-models.gif"><img src="docs/assets/artwork/roadside-models.gif" width="550" alt="Roadside models animated atlas"></a>

#### Mountain cableways · 25 variants

<a href="docs/assets/artwork/cableways.png"><img src="docs/assets/artwork/cableways.png" width="780" alt="Mountain cableways"></a>

<a href="docs/assets/artwork/cableways.gif"><img src="docs/assets/artwork/cableways.gif" width="550" alt="Mountain cableways animated atlas"></a>

#### Summit landmarks · 25 variants

<a href="docs/assets/artwork/summit-landmarks.png"><img src="docs/assets/artwork/summit-landmarks.png" width="780" alt="Summit landmarks"></a>

<a href="docs/assets/artwork/summit-landmarks.gif"><img src="docs/assets/artwork/summit-landmarks.gif" width="550" alt="Summit landmarks animated atlas"></a>

#### Summit trail fixtures · 25 variants

<a href="docs/assets/artwork/summit-trail-rests.png"><img src="docs/assets/artwork/summit-trail-rests.png" width="780" alt="Summit trail fixtures"></a>

<a href="docs/assets/artwork/summit-trail-rests.gif"><img src="docs/assets/artwork/summit-trail-rests.gif" width="550" alt="Summit trail fixtures animated atlas"></a>

### Railways

#### Rail cuts and embankments · 25 variants

<a href="docs/assets/artwork/rail-earthworks.png"><img src="docs/assets/artwork/rail-earthworks.png" width="780" alt="Rail cuts and embankments"></a>

<a href="docs/assets/artwork/rail-earthworks.gif"><img src="docs/assets/artwork/rail-earthworks.gif" width="550" alt="Rail cuts and embankments animated atlas"></a>

#### Railway fixtures · 25 variants

<a href="docs/assets/artwork/rail-stations.png"><img src="docs/assets/artwork/rail-stations.png" width="780" alt="Railway fixtures"></a>

<a href="docs/assets/artwork/rail-stations.gif"><img src="docs/assets/artwork/rail-stations.gif" width="550" alt="Railway fixtures animated atlas"></a>

#### Locomotives · 6 variants

<a href="docs/assets/artwork/locomotives.png"><img src="docs/assets/artwork/locomotives.png" width="780" alt="Locomotives"></a>

#### Freight wagons · 250 variants

<a href="docs/assets/artwork/wagons.png"><img src="docs/assets/artwork/wagons.png" width="780" alt="Freight wagons"></a>

### Airfields

#### Metropolitan airport terminals · 5 variants

<a href="docs/assets/artwork/airport-terminals.png"><img src="docs/assets/artwork/airport-terminals.png" width="780" alt="Metropolitan airport terminals"></a>

#### Runway markings · 25 variants

<a href="docs/assets/artwork/runway-markings.png"><img src="docs/assets/artwork/runway-markings.png" width="780" alt="Runway markings"></a>

<a href="docs/assets/artwork/runway-markings.gif"><img src="docs/assets/artwork/runway-markings.gif" width="550" alt="Runway markings animated atlas"></a>

#### Airfield aprons · 25 variants

<a href="docs/assets/artwork/airfield-aprons.png"><img src="docs/assets/artwork/airfield-aprons.png" width="780" alt="Airfield aprons"></a>

<a href="docs/assets/artwork/airfield-aprons.gif"><img src="docs/assets/artwork/airfield-aprons.gif" width="550" alt="Airfield aprons animated atlas"></a>

#### Airfield buildings · 25 variants

<a href="docs/assets/artwork/airfield-buildings.png"><img src="docs/assets/artwork/airfield-buildings.png" width="780" alt="Airfield buildings"></a>

<a href="docs/assets/artwork/airfield-buildings.gif"><img src="docs/assets/artwork/airfield-buildings.gif" width="550" alt="Airfield buildings animated atlas"></a>

### Ports and vessels

#### Lighthouses · 25 variants

<a href="docs/assets/artwork/lighthouses.png"><img src="docs/assets/artwork/lighthouses.png" width="780" alt="Lighthouses"></a>

<a href="docs/assets/artwork/lighthouses.gif"><img src="docs/assets/artwork/lighthouses.gif" width="550" alt="Lighthouses animated atlas"></a>

#### Waterfront landmarks · 4 variants

<a href="docs/assets/artwork/waterfront-landmarks.png"><img src="docs/assets/artwork/waterfront-landmarks.png" width="780" alt="Waterfront landmarks"></a>

#### Quayside workshops · 25 variants

<a href="docs/assets/artwork/quay-workshops.png"><img src="docs/assets/artwork/quay-workshops.png" width="780" alt="Quayside workshops"></a>

<a href="docs/assets/artwork/quay-workshops.gif"><img src="docs/assets/artwork/quay-workshops.gif" width="550" alt="Quayside workshops animated atlas"></a>

#### Harbor models · 30 variants

<a href="docs/assets/artwork/harbor-models.png"><img src="docs/assets/artwork/harbor-models.png" width="780" alt="Harbor models"></a>

<a href="docs/assets/artwork/harbor-models.gif"><img src="docs/assets/artwork/harbor-models.gif" width="550" alt="Harbor models animated atlas"></a>

#### Vessels · 250 variants

<a href="docs/assets/artwork/vessels.png"><img src="docs/assets/artwork/vessels.png" width="780" alt="Vessels"></a>

#### Service vessels · 25 variants

<a href="docs/assets/artwork/service-vessels.png"><img src="docs/assets/artwork/service-vessels.png" width="780" alt="Service vessels"></a>

<a href="docs/assets/artwork/service-vessels.gif"><img src="docs/assets/artwork/service-vessels.gif" width="550" alt="Service vessels animated atlas"></a>

## Los Angeles

[Fly Los Angeles to Las Vegas](https://xberenguer.github.io/scenic-flight/play.html?destination=southwest&departure=to-las-vegas). Both cities sit at their real locations in the same metre-scale world. The route passes Barstow, Baker, and Primm over USGS terrain, with mapped streets and buildings, rail around Barstow, and a runway at Baker. Scenery outside the mapped areas is generated from the flight seed. [Southwest map details](docs/LOS-ANGELES.md)

Fly twelve mapped approaches, from Downtown and Griffith Observatory to LAX, Santa Monica Pier, Venice Beach, and Getty Center. The 48 × 48 km detail area is at metre scale. Downtown landmarks include U.S. Bank Tower, Wilshire Grand Center, Los Angeles City Hall, and Walt Disney Concert Hall.

<table>
  <tr>
    <td align="center"><a href="docs/assets/los-angeles/los-angeles-city-hall.png"><img src="docs/assets/los-angeles/los-angeles-city-hall.png" width="220" alt="City Hall game model"></a><br>Los Angeles City Hall</td>
    <td align="center"><a href="docs/assets/los-angeles/walt-disney-concert-hall.png"><img src="docs/assets/los-angeles/walt-disney-concert-hall.png" width="220" alt="Concert Hall game model"></a><br>Walt Disney Concert Hall</td>
  </tr>
</table>

[Los Angeles coverage, checkpoint audit, and source data](docs/LOS-ANGELES.md)

## Las Vegas

Select **Las Vegas** under **Destination**, then choose the **Strip**, **Harry Reid Airport**, **Downtown**, **Sphere**, **Paris Las Vegas**, **High Roller**, **Venetian and Palazzo**, **Allegiant Stadium**, or **Fremont Street**. One world unit is one metre. The first area uses mapped road and building footprints with USGS terrain and FAA runway geometry.

<table>
  <tr>
    <td align="center"><a href="docs/assets/planning/las-vegas-strip.png"><img src="docs/assets/planning/las-vegas-strip.png" width="250" alt="Mapped Las Vegas Strip, including the Sphere"></a><br>The Strip</td>
    <td align="center"><a href="docs/assets/planning/las-vegas-airport.png"><img src="docs/assets/planning/las-vegas-airport.png" width="250" alt="Runways and terminals at Harry Reid Airport"></a><br>Harry Reid Airport</td>
    <td align="center"><a href="docs/assets/planning/las-vegas-downtown.png"><img src="docs/assets/planning/las-vegas-downtown.png" width="250" alt="Mapped streets and buildings in downtown Las Vegas"></a><br>Downtown</td>
  </tr>
</table>

<a href="docs/assets/planning/las-vegas-wide.png"><img src="docs/assets/planning/las-vegas-wide.png" width="820" alt="Las Vegas Valley elevation, the mapped urban area, and Lake Mead"></a>

[Las Vegas coverage and source data](docs/LAS-VEGAS.md)

## Las Vegas artwork

### Sphere LED programs

<table>
  <tr>
    <td align="center"><a href="docs/assets/vegas/landmarks/sphere-face.gif"><img src="docs/assets/vegas/landmarks/sphere-face.gif" width="158" alt="Face"></a><br><sub>Runtime model</sub><br>Face</td>
    <td align="center"><a href="docs/assets/vegas/landmarks/sphere-waves.gif"><img src="docs/assets/vegas/landmarks/sphere-waves.gif" width="158" alt="Waves"></a><br><sub>Runtime model</sub><br>Waves</td>
    <td align="center"><a href="docs/assets/vegas/landmarks/sphere-moon.gif"><img src="docs/assets/vegas/landmarks/sphere-moon.gif" width="158" alt="Moon"></a><br><sub>Runtime model</sub><br>Moon</td>
    <td align="center"><a href="docs/assets/vegas/landmarks/sphere-ribbons.gif"><img src="docs/assets/vegas/landmarks/sphere-ribbons.gif" width="158" alt="Ribbons"></a><br><sub>Runtime model</sub><br>Ribbons</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/vegas/landmarks/sphere-eclipse.gif"><img src="docs/assets/vegas/landmarks/sphere-eclipse.gif" width="158" alt="Eclipse"></a><br><sub>Runtime model</sub><br>Eclipse</td>
    <td align="center"><a href="docs/assets/vegas/landmarks/sphere-earth.gif"><img src="docs/assets/vegas/landmarks/sphere-earth.gif" width="158" alt="Earth"></a><br><sub>Runtime model</sub><br>Earth</td>
    <td align="center"><a href="docs/assets/vegas/landmarks/sphere-meteors.gif"><img src="docs/assets/vegas/landmarks/sphere-meteors.gif" width="158" alt="Meteor Trails"></a><br><sub>Runtime model</sub><br>Meteor Trails</td>
    <td align="center"><a href="docs/assets/vegas/landmarks/sphere-aurora.gif"><img src="docs/assets/vegas/landmarks/sphere-aurora.gif" width="158" alt="Aurora"></a><br><sub>Runtime model</sub><br>Aurora</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/vegas/landmarks/sphere-rings.gif"><img src="docs/assets/vegas/landmarks/sphere-rings.gif" width="158" alt="Neon Rings"></a><br><sub>Runtime model</sub><br>Neon Rings</td>
    <td align="center"><a href="docs/assets/vegas/landmarks/sphere-eye.gif"><img src="docs/assets/vegas/landmarks/sphere-eye.gif" width="158" alt="Blinking Eye"></a><br><sub>Runtime model</sub><br>Blinking Eye</td>
  </tr>
</table>

### Animated landmarks

<table>
  <tr>
    <td align="center"><a href="docs/assets/vegas/landmarks/high-roller.gif"><img src="docs/assets/vegas/landmarks/high-roller.gif" width="158" alt="High Roller"></a><br><sub>Runtime model</sub><br>High Roller</td>
    <td align="center"><a href="docs/assets/vegas/landmarks/paris-balloon.gif"><img src="docs/assets/vegas/landmarks/paris-balloon.gif" width="158" alt="Paris Balloon"></a><br><sub>Runtime model</sub><br>Paris Balloon</td>
    <td align="center"><a href="docs/assets/vegas/landmarks/viva-vision.gif"><img src="docs/assets/vegas/landmarks/viva-vision.gif" width="158" alt="Viva Vision"></a><br><sub>Runtime model</sub><br>Viva Vision</td>
  </tr>
</table>

### Landmark models

<table>
  <tr>
    <td align="center"><a href="docs/assets/vegas/landmarks/paris-eiffel-tower.png"><img src="docs/assets/vegas/landmarks/paris-eiffel-tower.png" width="158" alt="Eiffel Tower"></a><br><sub>Runtime model</sub><br>Eiffel Tower</td>
    <td align="center"><a href="docs/assets/vegas/landmarks/paris-arc-de-triomphe.png"><img src="docs/assets/vegas/landmarks/paris-arc-de-triomphe.png" width="158" alt="Arc de Triomphe"></a><br><sub>Runtime model</sub><br>Arc de Triomphe</td>
    <td align="center"><a href="docs/assets/vegas/landmarks/strat.png"><img src="docs/assets/vegas/landmarks/strat.png" width="158" alt="The STRAT"></a><br><sub>Runtime model</sub><br>The STRAT</td>
    <td align="center"><a href="docs/assets/vegas/landmarks/allegiant-stadium.png"><img src="docs/assets/vegas/landmarks/allegiant-stadium.png" width="158" alt="Allegiant Stadium"></a><br><sub>Runtime model</sub><br>Allegiant Stadium</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/vegas/landmarks/allegiant-field-tray.png"><img src="docs/assets/vegas/landmarks/allegiant-field-tray.png" width="158" alt="Allegiant Stadium field tray"></a><br><sub>Runtime model</sub><br>Allegiant Stadium field tray</td>
    <td align="center"><a href="docs/assets/vegas/landmarks/luxor-pyramid.png"><img src="docs/assets/vegas/landmarks/luxor-pyramid.png" width="158" alt="Luxor Pyramid"></a><br><sub>Packed terrain</sub><br>Luxor Pyramid</td>
  </tr>
</table>

### Landmarks and airfields

<table>
  <tr>
    <td align="center"><a href="docs/assets/vegas/landmarks/allegiant-stadium.png"><img src="docs/assets/vegas/landmarks/allegiant-stadium.png" width="105" alt="Allegiant Stadium"></a><br><sub>Runtime model</sub><br>Allegiant Stadium</td>
    <td align="center"><a href="docs/assets/vegas/landmarks/viva-vision.gif"><img src="docs/assets/vegas/landmarks/viva-vision.png" width="105" alt="Fremont Street Experience"></a><br><sub>Runtime model</sub><br>Fremont Street Experience</td>
    <td align="center"><a href="docs/assets/vegas/venues/harry-reid-international-airport.png"><img src="docs/assets/vegas/venues/harry-reid-international-airport.png" width="105" alt="Harry Reid International Airport"></a><br><sub>Packed runways and terminals</sub><br>Harry Reid International Airport</td>
    <td align="center"><a href="docs/assets/vegas/venues/north-las-vegas-airport.png"><img src="docs/assets/vegas/venues/north-las-vegas-airport.png" width="105" alt="North Las Vegas Airport"></a><br><sub>Packed runways and terminals</sub><br>North Las Vegas Airport</td>
    <td align="center"><a href="docs/assets/vegas/landmarks/sphere-face.gif"><img src="docs/assets/vegas/landmarks/sphere-face.png" width="105" alt="Sphere"></a><br><sub>Runtime model</sub><br>Sphere</td>
    <td align="center"><a href="docs/assets/vegas/venues/t-mobile-arena.png"><img src="docs/assets/vegas/venues/t-mobile-arena.png" width="105" alt="T-Mobile Arena"></a><br><sub>Generic packed footprint</sub><br>T-Mobile Arena</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/vegas/landmarks/allegiant-field-tray.png"><img src="docs/assets/vegas/landmarks/allegiant-field-tray.png" width="105" alt="Allegiant Stadium field tray"></a><br><sub>Runtime model</sub><br>Allegiant Stadium field tray</td>
    <td align="center"><a href="docs/assets/vegas/landmarks/paris-arc-de-triomphe.png"><img src="docs/assets/vegas/landmarks/paris-arc-de-triomphe.png" width="105" alt="Arc de Triomphe"></a><br><sub>Runtime model</sub><br>Arc de Triomphe</td>
    <td align="center"><a href="docs/assets/vegas/landmarks/paris-eiffel-tower.png"><img src="docs/assets/vegas/landmarks/paris-eiffel-tower.png" width="105" alt="Eiffel Tower"></a><br><sub>Runtime model</sub><br>Eiffel Tower</td>
    <td align="center"><a href="docs/assets/vegas/landmarks/high-roller.gif"><img src="docs/assets/vegas/landmarks/high-roller.png" width="105" alt="High Roller"></a><br><sub>Runtime model</sub><br>High Roller</td>
    <td align="center"><a href="docs/assets/vegas/landmarks/sphere-moon.gif"><img src="docs/assets/vegas/landmarks/sphere-moon.png" width="105" alt="Moon"></a><br><sub>Runtime model</sub><br>Moon</td>
    <td align="center"><a href="docs/assets/vegas/landmarks/paris-balloon.gif"><img src="docs/assets/vegas/landmarks/paris-balloon.png" width="105" alt="Paris Balloon"></a><br><sub>Runtime model</sub><br>Paris Balloon</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/vegas/landmarks/sphere-ribbons.gif"><img src="docs/assets/vegas/landmarks/sphere-ribbons.png" width="105" alt="Ribbons"></a><br><sub>Runtime model</sub><br>Ribbons</td>
    <td align="center"><a href="docs/assets/vegas/landmarks/sphere-waves.gif"><img src="docs/assets/vegas/landmarks/sphere-waves.png" width="105" alt="Waves"></a><br><sub>Runtime model</sub><br>Waves</td>
  </tr>
</table>

### Casinos

<table>
  <tr>
    <td align="center"><a href="docs/assets/vegas/venues/aria-resort-casino.png"><img src="docs/assets/vegas/venues/aria-resort-casino.png" width="105" alt="Aria Resort &amp; Casino"></a><br><sub>Generic packed footprint</sub><br>Aria Resort &amp; Casino</td>
    <td align="center"><a href="docs/assets/vegas/venues/arizona-charlies-decatur.png"><img src="docs/assets/vegas/venues/arizona-charlies-decatur.png" width="105" alt="Arizona Charlies Decatur"></a><br><sub>Generic packed footprint</sub><br>Arizona Charlies Decatur</td>
    <td align="center"><a href="docs/assets/vegas/venues/bellagio.png"><img src="docs/assets/vegas/venues/bellagio.png" width="105" alt="Bellagio"></a><br><sub>Generic packed footprint</sub><br>Bellagio</td>
    <td align="center"><a href="docs/assets/vegas/venues/best-western-plus-casino-royale.png"><img src="docs/assets/vegas/venues/best-western-plus-casino-royale.png" width="105" alt="Best Western Plus Casino Royale"></a><br><sub>Generic packed footprint</sub><br>Best Western Plus Casino Royale</td>
    <td align="center"><a href="docs/assets/vegas/venues/best-western-plus-casino-royale.png"><img src="docs/assets/vegas/venues/best-western-plus-casino-royale.png" width="105" alt="Best Western Plus Casino Royale-Center Strip"></a><br><sub>Generic packed footprint</sub><br>Best Western Plus Casino Royale-Center Strip</td>
    <td align="center"><a href="docs/assets/vegas/venues/binion-s-gambling-hall-and-hotel.png"><img src="docs/assets/vegas/venues/binion-s-gambling-hall-and-hotel.png" width="105" alt="Binion&#x27;s Gambling Hall and Hotel"></a><br><sub>Generic packed footprint</sub><br>Binion&#x27;s Gambling Hall and Hotel</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/vegas/venues/bonanza-lounge-casino.png"><img src="docs/assets/vegas/venues/bonanza-lounge-casino.png" width="105" alt="Bonanza Lounge &amp; Casino"></a><br><sub>Generic packed footprint</sub><br>Bonanza Lounge &amp; Casino</td>
    <td align="center"><a href="docs/assets/vegas/venues/caesars-palace-complex.png"><img src="docs/assets/vegas/venues/caesars-palace-complex.png" width="105" alt="Caesars Palace Complex"></a><br><sub>Generic packed footprint</sub><br>Caesars Palace Complex</td>
    <td align="center"><a href="docs/assets/vegas/venues/caesars-palace-complex.png"><img src="docs/assets/vegas/venues/caesars-palace-complex.png" width="105" alt="Caesars Palace Hotel and Casino"></a><br><sub>Generic packed footprint</sub><br>Caesars Palace Hotel and Casino</td>
    <td align="center"><a href="docs/assets/vegas/venues/caesars-palace-poker-room.png"><img src="docs/assets/vegas/venues/caesars-palace-poker-room.png" width="105" alt="Caesars Palace Poker Room"></a><br><sub>Generic packed footprint</sub><br>Caesars Palace Poker Room</td>
    <td align="center"><a href="docs/assets/vegas/venues/california-hotel-and-casino.png"><img src="docs/assets/vegas/venues/california-hotel-and-casino.png" width="105" alt="California Hotel and Casino"></a><br><sub>Generic packed footprint</sub><br>California Hotel and Casino</td>
    <td align="center"><a href="docs/assets/vegas/venues/circa-resort-casino.png"><img src="docs/assets/vegas/venues/circa-resort-casino.png" width="105" alt="Circa Resort &amp; Casino"></a><br><sub>Generic packed footprint</sub><br>Circa Resort &amp; Casino</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/vegas/venues/circus-circus-las-vegas.png"><img src="docs/assets/vegas/venues/circus-circus-las-vegas.png" width="105" alt="Circus Circus Las Vegas"></a><br><sub>Generic packed footprint</sub><br>Circus Circus Las Vegas</td>
    <td align="center"><a href="docs/assets/vegas/venues/dotty-s.png"><img src="docs/assets/vegas/venues/dotty-s.png" width="105" alt="Dotty&#x27;s"></a><br><sub>Generic packed footprint</sub><br>Dotty&#x27;s</td>
    <td align="center"><a href="docs/assets/vegas/venues/dotty-s-casino.png"><img src="docs/assets/vegas/venues/dotty-s-casino.png" width="105" alt="Dotty&#x27;s Casino"></a><br><sub>Generic packed footprint</sub><br>Dotty&#x27;s Casino</td>
    <td align="center"><a href="docs/assets/vegas/venues/el-cortez-hotel-casino.png"><img src="docs/assets/vegas/venues/el-cortez-hotel-casino.png" width="105" alt="El Cortez Hotel &amp; Casino"></a><br><sub>Generic packed footprint</sub><br>El Cortez Hotel &amp; Casino</td>
    <td align="center"><a href="docs/assets/vegas/venues/el-cortez-hotel-and-casino.png"><img src="docs/assets/vegas/venues/el-cortez-hotel-and-casino.png" width="105" alt="El Cortez Hotel and Casino"></a><br><sub>Generic packed footprint</sub><br>El Cortez Hotel and Casino</td>
    <td align="center"><a href="docs/assets/vegas/venues/ellis-island-casino.png"><img src="docs/assets/vegas/venues/ellis-island-casino.png" width="105" alt="Ellis Island Casino"></a><br><sub>Generic packed footprint</sub><br>Ellis Island Casino</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/vegas/venues/encore.png"><img src="docs/assets/vegas/venues/encore.png" width="105" alt="Encore"></a><br><sub>Generic packed footprint</sub><br>Encore</td>
    <td align="center"><a href="docs/assets/vegas/venues/encore-casino-at-wynn.png"><img src="docs/assets/vegas/venues/encore-casino-at-wynn.png" width="105" alt="Encore Casino at Wynn"></a><br><sub>Generic packed footprint</sub><br>Encore Casino at Wynn</td>
    <td align="center"><a href="docs/assets/vegas/venues/excalibur-hotel-casino.png"><img src="docs/assets/vegas/venues/excalibur-hotel-casino.png" width="105" alt="Excalibur Hotel &amp; Casino"></a><br><sub>Generic packed footprint</sub><br>Excalibur Hotel &amp; Casino</td>
    <td align="center"><a href="docs/assets/vegas/venues/flamingo-las-vegas.png"><img src="docs/assets/vegas/venues/flamingo-las-vegas.png" width="105" alt="Flamingo Las Vegas"></a><br><sub>Generic packed footprint</sub><br>Flamingo Las Vegas</td>
    <td align="center"><a href="docs/assets/vegas/venues/fontainebleau-resort-casino.png"><img src="docs/assets/vegas/venues/fontainebleau-resort-casino.png" width="105" alt="Fontainebleau Resort &amp; Casino"></a><br><sub>Generic packed footprint</sub><br>Fontainebleau Resort &amp; Casino</td>
    <td align="center"><a href="docs/assets/vegas/venues/fremont-hotel-casino-las-vegas.png"><img src="docs/assets/vegas/venues/fremont-hotel-casino-las-vegas.png" width="105" alt="Fremont Hotel &amp; Casino Las Vegas"></a><br><sub>Generic packed footprint</sub><br>Fremont Hotel &amp; Casino Las Vegas</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/vegas/venues/gold-coast-hotel-casino.png"><img src="docs/assets/vegas/venues/gold-coast-hotel-casino.png" width="105" alt="Gold Coast Hotel &amp; Casino"></a><br><sub>Generic packed footprint</sub><br>Gold Coast Hotel &amp; Casino</td>
    <td align="center"><a href="docs/assets/vegas/venues/gold-spike.png"><img src="docs/assets/vegas/venues/gold-spike.png" width="105" alt="Gold Spike"></a><br><sub>Generic packed footprint</sub><br>Gold Spike</td>
    <td align="center"><a href="docs/assets/vegas/venues/golden-gate-hotel-and-casino.png"><img src="docs/assets/vegas/venues/golden-gate-hotel-and-casino.png" width="105" alt="Golden Gate Hotel and Casino"></a><br><sub>Generic packed footprint</sub><br>Golden Gate Hotel and Casino</td>
    <td align="center"><a href="docs/assets/vegas/venues/hard-rock-las-vegas.png"><img src="docs/assets/vegas/venues/hard-rock-las-vegas.png" width="105" alt="Hard Rock Las Vegas"></a><br><sub>Nearby packed footprint</sub><br>Hard Rock Las Vegas</td>
    <td align="center"><a href="docs/assets/vegas/venues/harrah-s-las-vegas.png"><img src="docs/assets/vegas/venues/harrah-s-las-vegas.png" width="105" alt="Harrah&#x27;s Las Vegas"></a><br><sub>Generic packed footprint</sub><br>Harrah&#x27;s Las Vegas</td>
    <td align="center"><a href="docs/assets/vegas/venues/harrah-s-las-vegas-hotel-casino.png"><img src="docs/assets/vegas/venues/harrah-s-las-vegas-hotel-casino.png" width="105" alt="Harrah&#x27;s Las Vegas Hotel &amp; Casino"></a><br><sub>Generic packed footprint</sub><br>Harrah&#x27;s Las Vegas Hotel &amp; Casino</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/vegas/venues/horseshoe-las-vegas-hotel-casino-a-caesars-rewards-destination.png"><img src="docs/assets/vegas/venues/horseshoe-las-vegas-hotel-casino-a-caesars-rewards-destination.png" width="105" alt="Horseshoe Las Vegas Hotel &amp; Casino - A Caesars Rewards Destination"></a><br><sub>Generic packed footprint</sub><br>Horseshoe Las Vegas Hotel &amp; Casino - A Caesars Rewards Destination</td>
    <td align="center"><a href="docs/assets/vegas/venues/hotel-mgm-grand-las-vegas.png"><img src="docs/assets/vegas/venues/hotel-mgm-grand-las-vegas.png" width="105" alt="Hotel MGM Grand Las Vegas"></a><br><sub>Generic packed footprint</sub><br>Hotel MGM Grand Las Vegas</td>
    <td align="center"><a href="docs/assets/vegas/venues/jackpot-joanie-s.png"><img src="docs/assets/vegas/venues/jackpot-joanie-s.png" width="105" alt="Jackpot Joanie&#x27;s"></a><br><sub>Generic packed footprint</sub><br>Jackpot Joanie&#x27;s</td>
    <td align="center"><a href="docs/assets/vegas/venues/jackpot-joanie-s-casino.png"><img src="docs/assets/vegas/venues/jackpot-joanie-s-casino.png" width="105" alt="Jackpot Joanie&#x27;s Casino"></a><br><sub>Generic packed footprint</sub><br>Jackpot Joanie&#x27;s Casino</td>
    <td align="center"><a href="docs/assets/vegas/venues/jerry-s-nugget-casino.png"><img src="docs/assets/vegas/venues/jerry-s-nugget-casino.png" width="105" alt="Jerry&#x27;s Nugget Casino"></a><br><sub>Nearby packed footprint</sub><br>Jerry&#x27;s Nugget Casino</td>
    <td align="center"><a href="docs/assets/vegas/venues/just-play.png"><img src="docs/assets/vegas/venues/just-play.png" width="105" alt="Just Play"></a><br><sub>Generic packed footprint</sub><br>Just Play</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/vegas/venues/longhorn-casino.png"><img src="docs/assets/vegas/venues/longhorn-casino.png" width="105" alt="Longhorn Casino"></a><br><sub>Generic packed footprint</sub><br>Longhorn Casino</td>
    <td align="center"><a href="docs/assets/vegas/venues/lucky-club-hotel-casino.png"><img src="docs/assets/vegas/venues/lucky-club-hotel-casino.png" width="105" alt="Lucky Club Hotel &amp; Casino"></a><br><sub>Generic packed footprint</sub><br>Lucky Club Hotel &amp; Casino</td>
    <td align="center"><a href="docs/assets/vegas/venues/lucky-s-gaming-spirits-casino.png"><img src="docs/assets/vegas/venues/lucky-s-gaming-spirits-casino.png" width="105" alt="Lucky&#x27;s Gaming &amp; Spirits Casino"></a><br><sub>Generic packed footprint</sub><br>Lucky&#x27;s Gaming &amp; Spirits Casino</td>
    <td align="center"><a href="docs/assets/vegas/landmarks/luxor-pyramid.png"><img src="docs/assets/vegas/landmarks/luxor-pyramid.png" width="105" alt="Luxor Las Vegas"></a><br><sub>Packed terrain</sub><br>Luxor Las Vegas</td>
    <td align="center"><a href="docs/assets/vegas/venues/mgm-grand.png"><img src="docs/assets/vegas/venues/mgm-grand.png" width="105" alt="MGM Grand"></a><br><sub>Nearby packed footprint</sub><br>MGM Grand</td>
    <td align="center"><a href="docs/assets/vegas/venues/mandalay-bay.png"><img src="docs/assets/vegas/venues/mandalay-bay.png" width="105" alt="Mandalay Bay"></a><br><sub>Generic packed footprint</sub><br>Mandalay Bay</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/vegas/venues/mandalay-bay-resort-casino.png"><img src="docs/assets/vegas/venues/mandalay-bay-resort-casino.png" width="105" alt="Mandalay Bay Resort &amp; Casino"></a><br><sub>Generic packed footprint</sub><br>Mandalay Bay Resort &amp; Casino</td>
    <td align="center"><a href="docs/assets/vegas/venues/mardi-gras-hotel-casino.png"><img src="docs/assets/vegas/venues/mardi-gras-hotel-casino.png" width="105" alt="Mardi Gras Hotel &amp; Casino"></a><br><sub>Generic packed footprint</sub><br>Mardi Gras Hotel &amp; Casino</td>
    <td align="center"><a href="docs/assets/vegas/venues/miz-lola-s-spirits-gaming.png"><img src="docs/assets/vegas/venues/miz-lola-s-spirits-gaming.png" width="105" alt="Miz Lola&#x27;s Spirits &amp; Gaming"></a><br><sub>Generic packed footprint</sub><br>Miz Lola&#x27;s Spirits &amp; Gaming</td>
    <td align="center"><a href="docs/assets/vegas/venues/new-york-new-york-hotel-and-casino.png"><img src="docs/assets/vegas/venues/new-york-new-york-hotel-and-casino.png" width="105" alt="New York New York Hotel and Casino"></a><br><sub>Generic packed footprint</sub><br>New York New York Hotel and Casino</td>
    <td align="center"><a href="docs/assets/vegas/venues/nomad-high-limit-casino.png"><img src="docs/assets/vegas/venues/nomad-high-limit-casino.png" width="105" alt="NoMad High Limit Casino"></a><br><sub>Generic packed footprint</sub><br>NoMad High Limit Casino</td>
    <td align="center"><a href="docs/assets/vegas/venues/oyo-casino-hotel.png"><img src="docs/assets/vegas/venues/oyo-casino-hotel.png" width="105" alt="Oyo Casino Hotel"></a><br><sub>Generic packed footprint</sub><br>Oyo Casino Hotel</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/vegas/venues/palace-station.png"><img src="docs/assets/vegas/venues/palace-station.png" width="105" alt="Palace Station"></a><br><sub>Generic packed footprint</sub><br>Palace Station</td>
    <td align="center"><a href="docs/assets/vegas/venues/palms.png"><img src="docs/assets/vegas/venues/palms.png" width="105" alt="Palms"></a><br><sub>Generic packed footprint</sub><br>Palms</td>
    <td align="center"><a href="docs/assets/vegas/venues/paris-las-vegas.png"><img src="docs/assets/vegas/venues/paris-las-vegas.png" width="105" alt="Paris Las Vegas"></a><br><sub>Generic packed footprint</sub><br>Paris Las Vegas</td>
    <td align="center"><a href="docs/assets/vegas/venues/park-mgm.png"><img src="docs/assets/vegas/venues/park-mgm.png" width="105" alt="Park MGM"></a><br><sub>Generic packed footprint</sub><br>Park MGM</td>
    <td align="center"><a href="docs/assets/vegas/venues/pat-s-pub.png"><img src="docs/assets/vegas/venues/pat-s-pub.png" width="105" alt="Pat&#x27;s Pub"></a><br><sub>Generic packed footprint</sub><br>Pat&#x27;s Pub</td>
    <td align="center"><a href="docs/assets/vegas/venues/planet-hollywood.png"><img src="docs/assets/vegas/venues/planet-hollywood.png" width="105" alt="Planet Hollywood"></a><br><sub>Generic packed footprint</sub><br>Planet Hollywood</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/vegas/venues/planet-hollywood-las-vegas-resort-casino.png"><img src="docs/assets/vegas/venues/planet-hollywood-las-vegas-resort-casino.png" width="105" alt="Planet Hollywood Las Vegas Resort &amp; Casino"></a><br><sub>Generic packed footprint</sub><br>Planet Hollywood Las Vegas Resort &amp; Casino</td>
    <td align="center"><a href="docs/assets/vegas/venues/plaza-hotel-casino.png"><img src="docs/assets/vegas/venues/plaza-hotel-casino.png" width="105" alt="Plaza Hotel &amp; Casino"></a><br><sub>Generic packed footprint</sub><br>Plaza Hotel &amp; Casino</td>
    <td align="center"><a href="docs/assets/vegas/venues/red-dragon.png"><img src="docs/assets/vegas/venues/red-dragon.png" width="105" alt="Red Dragon"></a><br><sub>Generic packed footprint</sub><br>Red Dragon</td>
    <td align="center"><a href="docs/assets/vegas/venues/red-dragon-gaming-spirits.png"><img src="docs/assets/vegas/venues/red-dragon-gaming-spirits.png" width="105" alt="Red Dragon Gaming &amp; Spirits"></a><br><sub>Generic packed footprint</sub><br>Red Dragon Gaming &amp; Spirits</td>
    <td align="center"><a href="docs/assets/vegas/venues/resorts-world-las-vegas.png"><img src="docs/assets/vegas/venues/resorts-world-las-vegas.png" width="105" alt="Resorts World Las Vegas"></a><br><sub>Generic packed footprint</sub><br>Resorts World Las Vegas</td>
    <td align="center"><a href="docs/assets/vegas/venues/rio-all-suite-hotel-and-casino.png"><img src="docs/assets/vegas/venues/rio-all-suite-hotel-and-casino.png" width="105" alt="Rio All Suite Hotel and Casino"></a><br><sub>Generic packed footprint</sub><br>Rio All Suite Hotel and Casino</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/vegas/venues/sahara-las-vegas-hotel-casino.png"><img src="docs/assets/vegas/venues/sahara-las-vegas-hotel-casino.png" width="105" alt="Sahara Las Vegas Hotel &amp; Casino"></a><br><sub>Generic packed footprint</sub><br>Sahara Las Vegas Hotel &amp; Casino</td>
    <td align="center"><a href="docs/assets/vegas/venues/silver-sevens-hotel-casino.png"><img src="docs/assets/vegas/venues/silver-sevens-hotel-casino.png" width="105" alt="Silver Sevens Hotel &amp; Casino"></a><br><sub>Generic packed footprint</sub><br>Silver Sevens Hotel &amp; Casino</td>
    <td align="center"><a href="docs/assets/vegas/venues/silverton-casino-hotel.png"><img src="docs/assets/vegas/venues/silverton-casino-hotel.png" width="105" alt="Silverton Casino Hotel"></a><br><sub>Generic packed footprint</sub><br>Silverton Casino Hotel</td>
    <td align="center"><a href="docs/assets/vegas/venues/south-point.png"><img src="docs/assets/vegas/venues/south-point.png" width="105" alt="South Point"></a><br><sub>Generic packed footprint</sub><br>South Point</td>
    <td align="center"><a href="docs/assets/vegas/venues/south-point-hotel-casino-spa.png"><img src="docs/assets/vegas/venues/south-point-hotel-casino-spa.png" width="105" alt="South Point Hotel, Casino &amp; Spa"></a><br><sub>Generic packed footprint</sub><br>South Point Hotel, Casino &amp; Spa</td>
    <td align="center"><a href="docs/assets/vegas/venues/stage-door-casino.png"><img src="docs/assets/vegas/venues/stage-door-casino.png" width="105" alt="Stage Door Casino"></a><br><sub>Generic packed footprint</sub><br>Stage Door Casino</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/vegas/venues/sunset-station.png"><img src="docs/assets/vegas/venues/sunset-station.png" width="105" alt="Sunset Station"></a><br><sub>Generic packed footprint</sub><br>Sunset Station</td>
    <td align="center"><a href="docs/assets/vegas/venues/the-linq-hotel-experience.png"><img src="docs/assets/vegas/venues/the-linq-hotel-experience.png" width="105" alt="The LINQ Hotel + Experience"></a><br><sub>Generic packed footprint</sub><br>The LINQ Hotel + Experience</td>
    <td align="center"><a href="docs/assets/vegas/venues/the-orleans-hotel-casino.png"><img src="docs/assets/vegas/venues/the-orleans-hotel-casino.png" width="105" alt="The Orleans Hotel &amp; Casino"></a><br><sub>Generic packed footprint</sub><br>The Orleans Hotel &amp; Casino</td>
    <td align="center"><a href="docs/assets/vegas/venues/the-palazzo.png"><img src="docs/assets/vegas/venues/the-palazzo.png" width="105" alt="The Palazzo"></a><br><sub>Generic packed footprint</sub><br>The Palazzo</td>
    <td align="center"><a href="docs/assets/vegas/landmarks/strat.png"><img src="docs/assets/vegas/landmarks/strat.png" width="105" alt="The Strat"></a><br><sub>Runtime model</sub><br>The Strat</td>
    <td align="center"><a href="docs/assets/vegas/venues/the-venetian-las-vegas.png"><img src="docs/assets/vegas/venues/the-venetian-las-vegas.png" width="105" alt="The Venetian Las Vegas"></a><br><sub>Generic packed footprint</sub><br>The Venetian Las Vegas</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/vegas/venues/tix4tonight-slots-a-fun.png"><img src="docs/assets/vegas/venues/tix4tonight-slots-a-fun.png" width="105" alt="Tix4Tonight - Slots A Fun"></a><br><sub>Generic packed footprint</sub><br>Tix4Tonight - Slots A Fun</td>
    <td align="center"><a href="docs/assets/vegas/venues/treasure-island.png"><img src="docs/assets/vegas/venues/treasure-island.png" width="105" alt="Treasure Island"></a><br><sub>Generic packed footprint</sub><br>Treasure Island</td>
    <td align="center"><a href="docs/assets/vegas/venues/treasure-island.png"><img src="docs/assets/vegas/venues/treasure-island.png" width="105" alt="Treasure Island Hotel and Casino"></a><br><sub>Generic packed footprint</sub><br>Treasure Island Hotel and Casino</td>
    <td align="center"><a href="docs/assets/vegas/venues/tuscany-suites-casino.png"><img src="docs/assets/vegas/venues/tuscany-suites-casino.png" width="105" alt="Tuscany Suites &amp; Casino"></a><br><sub>Generic packed footprint</sub><br>Tuscany Suites &amp; Casino</td>
    <td align="center"><a href="docs/assets/vegas/venues/westgate-las-vegas-resort-casino.png"><img src="docs/assets/vegas/venues/westgate-las-vegas-resort-casino.png" width="105" alt="Westgate Las Vegas Resort &amp; Casino"></a><br><sub>Generic packed footprint</sub><br>Westgate Las Vegas Resort &amp; Casino</td>
    <td align="center"><a href="docs/assets/vegas/venues/wildfire.png"><img src="docs/assets/vegas/venues/wildfire.png" width="105" alt="Wildfire"></a><br><sub>Generic packed footprint</sub><br>Wildfire</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/vegas/venues/wildfire-casino.png"><img src="docs/assets/vegas/venues/wildfire-casino.png" width="105" alt="Wildfire Casino"></a><br><sub>Generic packed footprint</sub><br>Wildfire Casino</td>
    <td align="center"><a href="docs/assets/vegas/venues/wildfire-fremont.png"><img src="docs/assets/vegas/venues/wildfire-fremont.png" width="105" alt="Wildfire Fremont"></a><br><sub>Generic packed footprint</sub><br>Wildfire Fremont</td>
    <td align="center"><a href="docs/assets/vegas/venues/wynn-las-vegas-casino.png"><img src="docs/assets/vegas/venues/wynn-las-vegas-casino.png" width="105" alt="Wynn Las Vegas Casino"></a><br><sub>Generic packed footprint</sub><br>Wynn Las Vegas Casino</td>
  </tr>
</table>

### Hotels and other venues

<table>
  <tr>
    <td align="center"><a href="docs/assets/vegas/venues/ac-hotel-by-marriott-las-vegas-symphony-park.png"><img src="docs/assets/vegas/venues/ac-hotel-by-marriott-las-vegas-symphony-park.png" width="105" alt="AC Hotel by Marriott Las Vegas Symphony Park"></a><br><sub>Nearby packed footprint</sub><br>AC Hotel by Marriott Las Vegas Symphony Park</td>
    <td align="center"><a href="docs/assets/vegas/venues/ahern.png"><img src="docs/assets/vegas/venues/ahern.png" width="105" alt="Ahern"></a><br><sub>Generic packed footprint</sub><br>Ahern</td>
    <td align="center"><a href="docs/assets/vegas/venues/aloft-henderson.png"><img src="docs/assets/vegas/venues/aloft-henderson.png" width="105" alt="Aloft Henderson"></a><br><sub>Generic packed footprint</sub><br>Aloft Henderson</td>
    <td align="center"><a href="docs/assets/vegas/venues/americana-apartments.png"><img src="docs/assets/vegas/venues/americana-apartments.png" width="105" alt="Americana Apartments"></a><br><sub>Generic packed footprint</sub><br>Americana Apartments</td>
    <td align="center"><a href="docs/assets/vegas/venues/arizona-charlie-s-boulder.png"><img src="docs/assets/vegas/venues/arizona-charlie-s-boulder.png" width="105" alt="Arizona Charlie&#x27;s Boulder"></a><br><sub>Generic packed footprint</sub><br>Arizona Charlie&#x27;s Boulder</td>
    <td align="center"><a href="docs/assets/vegas/venues/aztec-inn.png"><img src="docs/assets/vegas/venues/aztec-inn.png" width="105" alt="Aztec Inn"></a><br><sub>Generic packed footprint</sub><br>Aztec Inn</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/vegas/venues/baymont-inn-suites-las-vegas-south-strip.png"><img src="docs/assets/vegas/venues/baymont-inn-suites-las-vegas-south-strip.png" width="105" alt="Baymont Inn &amp; Suites Las Vegas South Strip"></a><br><sub>Generic packed footprint</sub><br>Baymont Inn &amp; Suites Las Vegas South Strip</td>
    <td align="center"><a href="docs/assets/vegas/venues/bellagio-las-vegas.png"><img src="docs/assets/vegas/venues/bellagio-las-vegas.png" width="105" alt="Bellagio Las Vegas"></a><br><sub>Generic packed footprint</sub><br>Bellagio Las Vegas</td>
    <td align="center"><a href="docs/assets/vegas/venues/best-western-mccarran-inn.png"><img src="docs/assets/vegas/venues/best-western-mccarran-inn.png" width="105" alt="Best Western McCarran Inn"></a><br><sub>Generic packed footprint</sub><br>Best Western McCarran Inn</td>
    <td align="center"><a href="docs/assets/vegas/venues/best-western-plus-las-vegas-west.png"><img src="docs/assets/vegas/venues/best-western-plus-las-vegas-west.png" width="105" alt="Best Western Plus Las Vegas West"></a><br><sub>Generic packed footprint</sub><br>Best Western Plus Las Vegas West</td>
    <td align="center"><a href="docs/assets/vegas/venues/bluegreen-vacations-club-36-an-ascend-collection-resort.png"><img src="docs/assets/vegas/venues/bluegreen-vacations-club-36-an-ascend-collection-resort.png" width="105" alt="Bluegreen Vacations Club 36, an Ascend Collection Resort"></a><br><sub>Generic packed footprint</sub><br>Bluegreen Vacations Club 36, an Ascend Collection Resort</td>
    <td align="center"><a href="docs/assets/vegas/venues/boulder-station.png"><img src="docs/assets/vegas/venues/boulder-station.png" width="105" alt="Boulder Station"></a><br><sub>Generic packed footprint</sub><br>Boulder Station</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/vegas/venues/branding-iron-hotel.png"><img src="docs/assets/vegas/venues/branding-iron-hotel.png" width="105" alt="Branding Iron Hotel"></a><br><sub>Generic packed footprint</sub><br>Branding Iron Hotel</td>
    <td align="center"><a href="docs/assets/vegas/venues/bridger-inn-hotel-las-vegas.png"><img src="docs/assets/vegas/venues/bridger-inn-hotel-las-vegas.png" width="105" alt="Bridger Inn Hotel Las Vegas"></a><br><sub>Generic packed footprint</sub><br>Bridger Inn Hotel Las Vegas</td>
    <td align="center"><a href="docs/assets/vegas/venues/budget-suites-of-america-las-vegas.png"><img src="docs/assets/vegas/venues/budget-suites-of-america-las-vegas.png" width="105" alt="Budget Suites of America - Las Vegas"></a><br><sub>Generic packed footprint</sub><br>Budget Suites of America - Las Vegas</td>
    <td align="center"><a href="docs/assets/vegas/venues/cancun-resort-las-vegas.png"><img src="docs/assets/vegas/venues/cancun-resort-las-vegas.png" width="105" alt="Cancun Resort Las Vegas"></a><br><sub>Generic packed footprint</sub><br>Cancun Resort Las Vegas</td>
    <td align="center"><a href="docs/assets/vegas/venues/candlewood-suites-las-vegas.png"><img src="docs/assets/vegas/venues/candlewood-suites-las-vegas.png" width="105" alt="Candlewood Suites Las Vegas"></a><br><sub>Generic packed footprint</sub><br>Candlewood Suites Las Vegas</td>
    <td align="center"><a href="docs/assets/vegas/venues/candlewood-suites-las-vegas-e-tropicana.png"><img src="docs/assets/vegas/venues/candlewood-suites-las-vegas-e-tropicana.png" width="105" alt="Candlewood Suites Las Vegas - E Tropicana"></a><br><sub>Generic packed footprint</sub><br>Candlewood Suites Las Vegas - E Tropicana</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/vegas/venues/club-de-soleil.png"><img src="docs/assets/vegas/venues/club-de-soleil.png" width="105" alt="Club De Soleil"></a><br><sub>Generic packed footprint</sub><br>Club De Soleil</td>
    <td align="center"><a href="docs/assets/vegas/venues/comfort-inn.png"><img src="docs/assets/vegas/venues/comfort-inn.png" width="105" alt="Comfort Inn"></a><br><sub>Generic packed footprint</sub><br>Comfort Inn</td>
    <td align="center"><a href="docs/assets/vegas/venues/conrad-las-vegas-at-resorts-world.png"><img src="docs/assets/vegas/venues/conrad-las-vegas-at-resorts-world.png" width="105" alt="Conrad Las Vegas at Resorts World"></a><br><sub>Generic packed footprint</sub><br>Conrad Las Vegas at Resorts World</td>
    <td align="center"><a href="docs/assets/vegas/venues/courtyard.png"><img src="docs/assets/vegas/venues/courtyard.png" width="105" alt="Courtyard"></a><br><sub>Generic packed footprint</sub><br>Courtyard</td>
    <td align="center"><a href="docs/assets/vegas/venues/courtyard-las-vegas-convention-center.png"><img src="docs/assets/vegas/venues/courtyard-las-vegas-convention-center.png" width="105" alt="Courtyard Las Vegas Convention Center"></a><br><sub>Generic packed footprint</sub><br>Courtyard Las Vegas Convention Center</td>
    <td align="center"><a href="docs/assets/vegas/venues/courtyard-by-marriott-las-vegas-stadium-area.png"><img src="docs/assets/vegas/venues/courtyard-by-marriott-las-vegas-stadium-area.png" width="105" alt="Courtyard by Marriott Las Vegas Stadium Area"></a><br><sub>Generic packed footprint</sub><br>Courtyard by Marriott Las Vegas Stadium Area</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/vegas/venues/crockfords-las-vegas.png"><img src="docs/assets/vegas/venues/crockfords-las-vegas.png" width="105" alt="Crockfords Las Vegas"></a><br><sub>Generic packed footprint</sub><br>Crockfords Las Vegas</td>
    <td align="center"><a href="docs/assets/vegas/venues/days-inn.png"><img src="docs/assets/vegas/venues/days-inn.png" width="105" alt="Days Inn"></a><br><sub>Generic packed footprint</sub><br>Days Inn</td>
    <td align="center"><a href="docs/assets/vegas/venues/desert-rose-resort.png"><img src="docs/assets/vegas/venues/desert-rose-resort.png" width="105" alt="Desert Rose Resort"></a><br><sub>Generic packed footprint</sub><br>Desert Rose Resort</td>
    <td align="center"><a href="docs/assets/vegas/venues/doubletree-by-hilton-hotel-las-vegas-airport.png"><img src="docs/assets/vegas/venues/doubletree-by-hilton-hotel-las-vegas-airport.png" width="105" alt="DoubleTree by Hilton Hotel Las Vegas Airport"></a><br><sub>Generic packed footprint</sub><br>DoubleTree by Hilton Hotel Las Vegas Airport</td>
    <td align="center"><a href="docs/assets/vegas/venues/doubletree-by-hilton-las-vegas-east-flamingo.png"><img src="docs/assets/vegas/venues/doubletree-by-hilton-las-vegas-east-flamingo.png" width="105" alt="DoubleTree by Hilton Las Vegas East Flamingo"></a><br><sub>Generic packed footprint</sub><br>DoubleTree by Hilton Las Vegas East Flamingo</td>
    <td align="center"><a href="docs/assets/vegas/venues/downtown-grand.png"><img src="docs/assets/vegas/venues/downtown-grand.png" width="105" alt="Downtown Grand"></a><br><sub>Generic packed footprint</sub><br>Downtown Grand</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/vegas/venues/durango-resort.png"><img src="docs/assets/vegas/venues/durango-resort.png" width="105" alt="Durango Resort"></a><br><sub>Generic packed footprint</sub><br>Durango Resort</td>
    <td align="center"><a href="docs/assets/vegas/venues/el-cortez-cabana-suites.png"><img src="docs/assets/vegas/venues/el-cortez-cabana-suites.png" width="105" alt="El Cortez Cabana Suites"></a><br><sub>Generic packed footprint</sub><br>El Cortez Cabana Suites</td>
    <td align="center"><a href="docs/assets/vegas/venues/elara-a-hilton-grand-vacations-club.png"><img src="docs/assets/vegas/venues/elara-a-hilton-grand-vacations-club.png" width="105" alt="Elara, a Hilton Grand Vacations Club"></a><br><sub>Generic packed footprint</sub><br>Elara, a Hilton Grand Vacations Club</td>
    <td align="center"><a href="docs/assets/vegas/venues/ac-hotel-by-marriott-las-vegas-symphony-park.png"><img src="docs/assets/vegas/venues/ac-hotel-by-marriott-las-vegas-symphony-park.png" width="105" alt="Element by Marriott Las Vegas Symphony Park"></a><br><sub>Generic packed footprint</sub><br>Element by Marriott Las Vegas Symphony Park</td>
    <td align="center"><a href="docs/assets/vegas/venues/embassy-suites-by-hilton-convention-center-las-vegas.png"><img src="docs/assets/vegas/venues/embassy-suites-by-hilton-convention-center-las-vegas.png" width="105" alt="Embassy Suites by Hilton Convention Center Las Vegas"></a><br><sub>Generic packed footprint</sub><br>Embassy Suites by Hilton Convention Center Las Vegas</td>
    <td align="center"><a href="docs/assets/vegas/venues/embassy-suites-by-hilton-las-vegas.png"><img src="docs/assets/vegas/venues/embassy-suites-by-hilton-las-vegas.png" width="105" alt="Embassy Suites by Hilton Las Vegas"></a><br><sub>Generic packed footprint</sub><br>Embassy Suites by Hilton Las Vegas</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/vegas/venues/emerald-suites.png"><img src="docs/assets/vegas/venues/emerald-suites.png" width="105" alt="Emerald Suites"></a><br><sub>Generic packed footprint</sub><br>Emerald Suites</td>
    <td align="center"><a href="docs/assets/vegas/venues/encore-las-vegas.png"><img src="docs/assets/vegas/venues/encore-las-vegas.png" width="105" alt="Encore Las Vegas"></a><br><sub>Generic packed footprint</sub><br>Encore Las Vegas</td>
    <td align="center"><a href="docs/assets/vegas/venues/extended-stay-america.png"><img src="docs/assets/vegas/venues/extended-stay-america.png" width="105" alt="Extended Stay America"></a><br><sub>Generic packed footprint</sub><br>Extended Stay America</td>
    <td align="center"><a href="docs/assets/vegas/venues/fairfield-inn.png"><img src="docs/assets/vegas/venues/fairfield-inn.png" width="105" alt="Fairfield Inn"></a><br><sub>Generic packed footprint</sub><br>Fairfield Inn</td>
    <td align="center"><a href="docs/assets/vegas/venues/fairfield-inn-suites-las-vegas-airport-south.png"><img src="docs/assets/vegas/venues/fairfield-inn-suites-las-vegas-airport-south.png" width="105" alt="Fairfield Inn &amp; Suites Las Vegas Airport South"></a><br><sub>Generic packed footprint</sub><br>Fairfield Inn &amp; Suites Las Vegas Airport South</td>
    <td align="center"><a href="docs/assets/vegas/venues/fairfield-inn-suites-las-vegas-stadium-area.png"><img src="docs/assets/vegas/venues/fairfield-inn-suites-las-vegas-stadium-area.png" width="105" alt="Fairfield Inn &amp; Suites Las Vegas Stadium Area"></a><br><sub>Generic packed footprint</sub><br>Fairfield Inn &amp; Suites Las Vegas Stadium Area</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/vegas/venues/fortune-hotel-suites.png"><img src="docs/assets/vegas/venues/fortune-hotel-suites.png" width="105" alt="Fortune Hotel &amp; Suites"></a><br><sub>Generic packed footprint</sub><br>Fortune Hotel &amp; Suites</td>
    <td align="center"><a href="docs/assets/vegas/venues/four-queens.png"><img src="docs/assets/vegas/venues/four-queens.png" width="105" alt="Four Queens"></a><br><sub>Generic packed footprint</sub><br>Four Queens</td>
    <td align="center"><a href="docs/assets/vegas/venues/four-seasons-hotel-las-vegas.png"><img src="docs/assets/vegas/venues/four-seasons-hotel-las-vegas.png" width="105" alt="Four Seasons Hotel Las Vegas"></a><br><sub>Generic packed footprint</sub><br>Four Seasons Hotel Las Vegas</td>
    <td align="center"><a href="docs/assets/vegas/venues/golden-nugget-las-vegas.png"><img src="docs/assets/vegas/venues/golden-nugget-las-vegas.png" width="105" alt="Golden Nugget Las Vegas"></a><br><sub>Generic packed footprint</sub><br>Golden Nugget Las Vegas</td>
    <td align="center"><a href="docs/assets/vegas/venues/grandview-at-las-vegas.png"><img src="docs/assets/vegas/venues/grandview-at-las-vegas.png" width="105" alt="Grandview at Las Vegas"></a><br><sub>Nearby packed footprint</sub><br>Grandview at Las Vegas</td>
    <td align="center"><a href="docs/assets/vegas/venues/green-valley-ranch.png"><img src="docs/assets/vegas/venues/green-valley-ranch.png" width="105" alt="Green Valley Ranch"></a><br><sub>Generic packed footprint</sub><br>Green Valley Ranch</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/vegas/venues/hampton.png"><img src="docs/assets/vegas/venues/hampton.png" width="105" alt="Hampton"></a><br><sub>Generic packed footprint</sub><br>Hampton</td>
    <td align="center"><a href="docs/assets/vegas/venues/hampton-inn-suites-las-vegas-airport.png"><img src="docs/assets/vegas/venues/hampton-inn-suites-las-vegas-airport.png" width="105" alt="Hampton Inn &amp; Suites Las Vegas Airport"></a><br><sub>Generic packed footprint</sub><br>Hampton Inn &amp; Suites Las Vegas Airport</td>
    <td align="center"><a href="docs/assets/vegas/venues/hampton-inn-suites-las-vegas-convention-center.png"><img src="docs/assets/vegas/venues/hampton-inn-suites-las-vegas-convention-center.png" width="105" alt="Hampton Inn &amp; Suites Las Vegas Convention Center"></a><br><sub>Generic packed footprint</sub><br>Hampton Inn &amp; Suites Las Vegas Convention Center</td>
    <td align="center"><a href="docs/assets/vegas/venues/hampton-inn-suites-las-vegas-henderson.png"><img src="docs/assets/vegas/venues/hampton-inn-suites-las-vegas-henderson.png" width="105" alt="Hampton Inn &amp; Suites Las Vegas-Henderson"></a><br><sub>Generic packed footprint</sub><br>Hampton Inn &amp; Suites Las Vegas-Henderson</td>
    <td align="center"><a href="docs/assets/vegas/venues/hampton-inn-las-vegas-strip-south.png"><img src="docs/assets/vegas/venues/hampton-inn-las-vegas-strip-south.png" width="105" alt="Hampton Inn Las Vegas Strip South"></a><br><sub>Generic packed footprint</sub><br>Hampton Inn Las Vegas Strip South</td>
    <td align="center"><a href="docs/assets/vegas/venues/hampton-inn-tropicana.png"><img src="docs/assets/vegas/venues/hampton-inn-tropicana.png" width="105" alt="Hampton Inn Tropicana"></a><br><sub>Generic packed footprint</sub><br>Hampton Inn Tropicana</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/vegas/venues/highland-inn.png"><img src="docs/assets/vegas/venues/highland-inn.png" width="105" alt="Highland Inn"></a><br><sub>Generic packed footprint</sub><br>Highland Inn</td>
    <td align="center"><a href="docs/assets/vegas/venues/hilton-garden-inn.png"><img src="docs/assets/vegas/venues/hilton-garden-inn.png" width="105" alt="Hilton Garden Inn"></a><br><sub>Generic packed footprint</sub><br>Hilton Garden Inn</td>
    <td align="center"><a href="docs/assets/vegas/venues/hilton-garden-inn-las-vegas-strip-south.png"><img src="docs/assets/vegas/venues/hilton-garden-inn-las-vegas-strip-south.png" width="105" alt="Hilton Garden Inn Las Vegas Strip South"></a><br><sub>Generic packed footprint</sub><br>Hilton Garden Inn Las Vegas Strip South</td>
    <td align="center"><a href="docs/assets/vegas/venues/hilton-garden-inn-las-vegas-henderson.png"><img src="docs/assets/vegas/venues/hilton-garden-inn-las-vegas-henderson.png" width="105" alt="Hilton Garden Inn Las Vegas/Henderson"></a><br><sub>Generic packed footprint</sub><br>Hilton Garden Inn Las Vegas/Henderson</td>
    <td align="center"><a href="docs/assets/vegas/venues/hilton-grand-vacations-suites-las-vegas-convention-center.png"><img src="docs/assets/vegas/venues/hilton-grand-vacations-suites-las-vegas-convention-center.png" width="105" alt="Hilton Grand Vacations Suites - Las Vegas (Convention Center)"></a><br><sub>Generic packed footprint</sub><br>Hilton Grand Vacations Suites - Las Vegas (Convention Center)</td>
    <td align="center"><a href="docs/assets/vegas/venues/hilton-grand-vacations-at-the-flamingo.png"><img src="docs/assets/vegas/venues/hilton-grand-vacations-at-the-flamingo.png" width="105" alt="Hilton Grand Vacations at the Flamingo"></a><br><sub>Generic packed footprint</sub><br>Hilton Grand Vacations at the Flamingo</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/vegas/venues/hilton-grand-vacations-on-the-boulevard.png"><img src="docs/assets/vegas/venues/hilton-grand-vacations-on-the-boulevard.png" width="105" alt="Hilton Grand Vacations on the Boulevard"></a><br><sub>Generic packed footprint</sub><br>Hilton Grand Vacations on the Boulevard</td>
    <td align="center"><a href="docs/assets/vegas/venues/hilton-vacation-club-polo-towers-las-vegas.png"><img src="docs/assets/vegas/venues/hilton-vacation-club-polo-towers-las-vegas.png" width="105" alt="Hilton Vacation Club Polo Towers Las Vegas"></a><br><sub>Generic packed footprint</sub><br>Hilton Vacation Club Polo Towers Las Vegas</td>
    <td align="center"><a href="docs/assets/vegas/venues/holiday-inn-express-suites-henderson.png"><img src="docs/assets/vegas/venues/holiday-inn-express-suites-henderson.png" width="105" alt="Holiday Inn Express &amp; Suites Henderson"></a><br><sub>Generic packed footprint</sub><br>Holiday Inn Express &amp; Suites Henderson</td>
    <td align="center"><a href="docs/assets/vegas/venues/holiday-inn-express-suites-las-vegas-e-tropicana.png"><img src="docs/assets/vegas/venues/holiday-inn-express-suites-las-vegas-e-tropicana.png" width="105" alt="Holiday Inn Express &amp; Suites Las Vegas - E Tropicana"></a><br><sub>Generic packed footprint</sub><br>Holiday Inn Express &amp; Suites Las Vegas - E Tropicana</td>
    <td align="center"><a href="docs/assets/vegas/venues/holiday-inn-express-suites-las-vegas-sw-spring-valley.png"><img src="docs/assets/vegas/venues/holiday-inn-express-suites-las-vegas-sw-spring-valley.png" width="105" alt="Holiday Inn Express &amp; Suites Las Vegas SW - Spring Valley"></a><br><sub>Generic packed footprint</sub><br>Holiday Inn Express &amp; Suites Las Vegas SW - Spring Valley</td>
    <td align="center"><a href="docs/assets/vegas/venues/holiday-inn-express-las-vegas-south.png"><img src="docs/assets/vegas/venues/holiday-inn-express-las-vegas-south.png" width="105" alt="Holiday Inn Express Las Vegas - South"></a><br><sub>Generic packed footprint</sub><br>Holiday Inn Express Las Vegas - South</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/vegas/venues/home2-suites-las-vegas-i-215-curve.png"><img src="docs/assets/vegas/venues/home2-suites-las-vegas-i-215-curve.png" width="105" alt="Home2 Suites Las Vegas I-215 Curve"></a><br><sub>Generic packed footprint</sub><br>Home2 Suites Las Vegas I-215 Curve</td>
    <td align="center"><a href="docs/assets/vegas/venues/home2-suites-by-hilton-las-vegas-city-center.png"><img src="docs/assets/vegas/venues/home2-suites-by-hilton-las-vegas-city-center.png" width="105" alt="Home2 Suites by Hilton Las Vegas City Center"></a><br><sub>Generic packed footprint</sub><br>Home2 Suites by Hilton Las Vegas City Center</td>
    <td align="center"><a href="docs/assets/vegas/venues/hampton-inn-suites-las-vegas-convention-center.png"><img src="docs/assets/vegas/venues/hampton-inn-suites-las-vegas-convention-center.png" width="105" alt="Home2 Suites by Hilton Las Vegas Convention Center"></a><br><sub>Generic packed footprint</sub><br>Home2 Suites by Hilton Las Vegas Convention Center</td>
    <td align="center"><a href="docs/assets/vegas/venues/home2-suites-by-hilton-las-vegas-strip-south.png"><img src="docs/assets/vegas/venues/home2-suites-by-hilton-las-vegas-strip-south.png" width="105" alt="Home2 Suites by Hilton Las Vegas Strip South"></a><br><sub>Generic packed footprint</sub><br>Home2 Suites by Hilton Las Vegas Strip South</td>
    <td align="center"><a href="docs/assets/vegas/venues/homewood-suites-by-hilton-las-vegas-airport.png"><img src="docs/assets/vegas/venues/homewood-suites-by-hilton-las-vegas-airport.png" width="105" alt="Homewood Suites by Hilton Las Vegas Airport"></a><br><sub>Generic packed footprint</sub><br>Homewood Suites by Hilton Las Vegas Airport</td>
    <td align="center"><a href="docs/assets/vegas/venues/homewood-suites-by-hilton-las-vegas-city-center.png"><img src="docs/assets/vegas/venues/homewood-suites-by-hilton-las-vegas-city-center.png" width="105" alt="Homewood Suites by Hilton Las Vegas City Center"></a><br><sub>Generic packed footprint</sub><br>Homewood Suites by Hilton Las Vegas City Center</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/vegas/venues/hotel-apache-adults-only.png"><img src="docs/assets/vegas/venues/hotel-apache-adults-only.png" width="105" alt="Hotel Apache - Adults Only"></a><br><sub>Generic packed footprint</sub><br>Hotel Apache - Adults Only</td>
    <td align="center"><a href="docs/assets/vegas/venues/howard-johnson-tropicana.png"><img src="docs/assets/vegas/venues/howard-johnson-tropicana.png" width="105" alt="Howard Johnson Tropicana"></a><br><sub>Generic packed footprint</sub><br>Howard Johnson Tropicana</td>
    <td align="center"><a href="docs/assets/vegas/venues/hyatt-place.png"><img src="docs/assets/vegas/venues/hyatt-place.png" width="105" alt="Hyatt Place"></a><br><sub>Generic packed footprint</sub><br>Hyatt Place</td>
    <td align="center"><a href="docs/assets/vegas/venues/hyatt-place-las-vegas-at-silverton-village.png"><img src="docs/assets/vegas/venues/hyatt-place-las-vegas-at-silverton-village.png" width="105" alt="Hyatt Place Las Vegas At Silverton Village"></a><br><sub>Generic packed footprint</sub><br>Hyatt Place Las Vegas At Silverton Village</td>
    <td align="center"><a href="docs/assets/vegas/venues/jockey-club.png"><img src="docs/assets/vegas/venues/jockey-club.png" width="105" alt="Jockey Club"></a><br><sub>Generic packed footprint</sub><br>Jockey Club</td>
    <td align="center"><a href="docs/assets/vegas/venues/john-e-carson-hotel.png"><img src="docs/assets/vegas/venues/john-e-carson-hotel.png" width="105" alt="John E. Carson Hotel"></a><br><sub>Generic packed footprint</sub><br>John E. Carson Hotel</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/vegas/venues/knotty-pine-motel.png"><img src="docs/assets/vegas/venues/knotty-pine-motel.png" width="105" alt="Knotty Pine Motel"></a><br><sub>Generic packed footprint</sub><br>Knotty Pine Motel</td>
    <td align="center"><a href="docs/assets/vegas/venues/la-quinta-inn-suites-las-vegas-airport-n-conv.png"><img src="docs/assets/vegas/venues/la-quinta-inn-suites-las-vegas-airport-n-conv.png" width="105" alt="La Quinta Inn &amp; Suites Las Vegas Airport N Conv."></a><br><sub>Generic packed footprint</sub><br>La Quinta Inn &amp; Suites Las Vegas Airport N Conv.</td>
    <td align="center"><a href="docs/assets/vegas/venues/la-quinta-inn-suites-las-vegas-airport-south.png"><img src="docs/assets/vegas/venues/la-quinta-inn-suites-las-vegas-airport-south.png" width="105" alt="La Quinta Inn &amp; Suites Las Vegas Airport South"></a><br><sub>Generic packed footprint</sub><br>La Quinta Inn &amp; Suites Las Vegas Airport South</td>
    <td align="center"><a href="docs/assets/vegas/venues/la-quinta-inn-suites-las-vegas-summerlin-tech.png"><img src="docs/assets/vegas/venues/la-quinta-inn-suites-las-vegas-summerlin-tech.png" width="105" alt="La Quinta Inn &amp; Suites Las Vegas Summerlin Tech"></a><br><sub>Generic packed footprint</sub><br>La Quinta Inn &amp; Suites Las Vegas Summerlin Tech</td>
    <td align="center"><a href="docs/assets/vegas/venues/la-quinta-inn-suites-by-wyndham-lv-tropicana-stadium.png"><img src="docs/assets/vegas/venues/la-quinta-inn-suites-by-wyndham-lv-tropicana-stadium.png" width="105" alt="La Quinta Inn &amp; Suites by Wyndham LV Tropicana Stadium"></a><br><sub>Generic packed footprint</sub><br>La Quinta Inn &amp; Suites by Wyndham LV Tropicana Stadium</td>
    <td align="center"><a href="docs/assets/vegas/venues/las-vegas-desert-club-resort.png"><img src="docs/assets/vegas/venues/las-vegas-desert-club-resort.png" width="105" alt="Las Vegas Desert Club Resort"></a><br><sub>Generic packed footprint</sub><br>Las Vegas Desert Club Resort</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/vegas/venues/las-vegas-hilton-at-resorts-world.png"><img src="docs/assets/vegas/venues/las-vegas-hilton-at-resorts-world.png" width="105" alt="Las Vegas Hilton at Resorts World"></a><br><sub>Generic packed footprint</sub><br>Las Vegas Hilton at Resorts World</td>
    <td align="center"><a href="docs/assets/vegas/venues/las-vegas-marriott.png"><img src="docs/assets/vegas/venues/las-vegas-marriott.png" width="105" alt="Las Vegas Marriott"></a><br><sub>Generic packed footprint</sub><br>Las Vegas Marriott</td>
    <td align="center"><a href="docs/assets/vegas/venues/main-street-station.png"><img src="docs/assets/vegas/venues/main-street-station.png" width="105" alt="Main Street Station"></a><br><sub>Generic packed footprint</sub><br>Main Street Station</td>
    <td align="center"><a href="docs/assets/vegas/venues/mainstay-suites.png"><img src="docs/assets/vegas/venues/mainstay-suites.png" width="105" alt="MainStay Suites"></a><br><sub>Generic packed footprint</sub><br>MainStay Suites</td>
    <td align="center"><a href="docs/assets/vegas/venues/manor-suites.png"><img src="docs/assets/vegas/venues/manor-suites.png" width="105" alt="Manor Suites"></a><br><sub>Generic packed footprint</sub><br>Manor Suites</td>
    <td align="center"><a href="docs/assets/vegas/venues/marriott-s-grand-chateau.png"><img src="docs/assets/vegas/venues/marriott-s-grand-chateau.png" width="105" alt="Marriott&#x27;s Grand Chateau"></a><br><sub>Generic packed footprint</sub><br>Marriott&#x27;s Grand Chateau</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/vegas/venues/monterey-grand-manor.png"><img src="docs/assets/vegas/venues/monterey-grand-manor.png" width="105" alt="Monterey Grand Manor"></a><br><sub>Generic packed footprint</sub><br>Monterey Grand Manor</td>
    <td align="center"><a href="docs/assets/vegas/venues/nobu-hotel-las-vegas.png"><img src="docs/assets/vegas/venues/nobu-hotel-las-vegas.png" width="105" alt="NOBU Hotel Las Vegas"></a><br><sub>Generic packed footprint</sub><br>NOBU Hotel Las Vegas</td>
    <td align="center"><a href="docs/assets/vegas/venues/nirvana-hotel.png"><img src="docs/assets/vegas/venues/nirvana-hotel.png" width="105" alt="Nirvana Hotel"></a><br><sub>Generic packed footprint</sub><br>Nirvana Hotel</td>
    <td align="center"><a href="docs/assets/vegas/venues/oasis-at-gold-spike.png"><img src="docs/assets/vegas/venues/oasis-at-gold-spike.png" width="105" alt="Oasis at Gold Spike"></a><br><sub>Generic packed footprint</sub><br>Oasis at Gold Spike</td>
    <td align="center"><a href="docs/assets/vegas/venues/otonomus-hotel.png"><img src="docs/assets/vegas/venues/otonomus-hotel.png" width="105" alt="Otonomus Hotel"></a><br><sub>Generic packed footprint</sub><br>Otonomus Hotel</td>
    <td align="center"><a href="docs/assets/vegas/venues/park-inn-by-radisson.png"><img src="docs/assets/vegas/venues/park-inn-by-radisson.png" width="105" alt="Park Inn by Radisson"></a><br><sub>Generic packed footprint</sub><br>Park Inn by Radisson</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/vegas/venues/peppermill.png"><img src="docs/assets/vegas/venues/peppermill.png" width="105" alt="Peppermill"></a><br><sub>Generic packed footprint</sub><br>Peppermill</td>
    <td align="center"><a href="docs/assets/vegas/venues/platinum-hotel-and-spa.png"><img src="docs/assets/vegas/venues/platinum-hotel-and-spa.png" width="105" alt="Platinum Hotel and Spa"></a><br><sub>Generic packed footprint</sub><br>Platinum Hotel and Spa</td>
    <td align="center"><a href="docs/assets/vegas/venues/red-roof-inn-las-vegas.png"><img src="docs/assets/vegas/venues/red-roof-inn-las-vegas.png" width="105" alt="Red Roof Inn Las Vegas"></a><br><sub>Generic packed footprint</sub><br>Red Roof Inn Las Vegas</td>
    <td align="center"><a href="docs/assets/vegas/venues/renaissance-las-vegas-hotel.png"><img src="docs/assets/vegas/venues/renaissance-las-vegas-hotel.png" width="105" alt="Renaissance Las Vegas Hotel"></a><br><sub>Generic packed footprint</sub><br>Renaissance Las Vegas Hotel</td>
    <td align="center"><a href="docs/assets/vegas/venues/residence-inn.png"><img src="docs/assets/vegas/venues/residence-inn.png" width="105" alt="Residence Inn"></a><br><sub>Generic packed footprint</sub><br>Residence Inn</td>
    <td align="center"><a href="docs/assets/vegas/venues/residence-inn-las-vegas-airport.png"><img src="docs/assets/vegas/venues/residence-inn-las-vegas-airport.png" width="105" alt="Residence Inn Las Vegas Airport"></a><br><sub>Generic packed footprint</sub><br>Residence Inn Las Vegas Airport</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/vegas/venues/residence-inn-las-vegas-convention-center.png"><img src="docs/assets/vegas/venues/residence-inn-las-vegas-convention-center.png" width="105" alt="Residence Inn Las Vegas Convention Center"></a><br><sub>Generic packed footprint</sub><br>Residence Inn Las Vegas Convention Center</td>
    <td align="center"><a href="docs/assets/vegas/venues/residence-inn-las-vegas-hughes-center.png"><img src="docs/assets/vegas/venues/residence-inn-las-vegas-hughes-center.png" width="105" alt="Residence Inn Las Vegas Hughes Center"></a><br><sub>Generic packed footprint</sub><br>Residence Inn Las Vegas Hughes Center</td>
    <td align="center"><a href="docs/assets/vegas/venues/residence-inn-las-vegas-south.png"><img src="docs/assets/vegas/venues/residence-inn-las-vegas-south.png" width="105" alt="Residence Inn Las Vegas South"></a><br><sub>Generic packed footprint</sub><br>Residence Inn Las Vegas South</td>
    <td align="center"><a href="docs/assets/vegas/venues/riu.png"><img src="docs/assets/vegas/venues/riu.png" width="105" alt="Riu"></a><br><sub>Packed map location</sub><br>Riu</td>
    <td align="center"><a href="docs/assets/vegas/venues/sam-s-town-hotel-gambling-hall.png"><img src="docs/assets/vegas/venues/sam-s-town-hotel-gambling-hall.png" width="105" alt="Sam&#x27;s Town Hotel &amp; Gambling Hall"></a><br><sub>Generic packed footprint</sub><br>Sam&#x27;s Town Hotel &amp; Gambling Hall</td>
    <td align="center"><a href="docs/assets/vegas/venues/serene.png"><img src="docs/assets/vegas/venues/serene.png" width="105" alt="Serene"></a><br><sub>Nearby packed footprint</sub><br>Serene</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/vegas/venues/shalimar-hotel-of-las-vegas.png"><img src="docs/assets/vegas/venues/shalimar-hotel-of-las-vegas.png" width="105" alt="Shalimar Hotel of Las Vegas"></a><br><sub>Generic packed footprint</sub><br>Shalimar Hotel of Las Vegas</td>
    <td align="center"><a href="docs/assets/vegas/venues/siegel-select-flamingo.png"><img src="docs/assets/vegas/venues/siegel-select-flamingo.png" width="105" alt="Siegel Select  Flamingo"></a><br><sub>Generic packed footprint</sub><br>Siegel Select  Flamingo</td>
    <td align="center"><a href="docs/assets/vegas/venues/siegel-suites-select-convention-center.png"><img src="docs/assets/vegas/venues/siegel-suites-select-convention-center.png" width="105" alt="Siegel Suites Select Convention Center"></a><br><sub>Generic packed footprint</sub><br>Siegel Suites Select Convention Center</td>
    <td align="center"><a href="docs/assets/vegas/venues/siegel-suites-select-las-vegas-boulevard.png"><img src="docs/assets/vegas/venues/siegel-suites-select-las-vegas-boulevard.png" width="105" alt="Siegel Suites Select – Las Vegas Boulevard"></a><br><sub>Generic packed footprint</sub><br>Siegel Suites Select – Las Vegas Boulevard</td>
    <td align="center"><a href="docs/assets/vegas/venues/skylofts-at-mgm-grand.png"><img src="docs/assets/vegas/venues/skylofts-at-mgm-grand.png" width="105" alt="Skylofts at MGM Grand"></a><br><sub>Generic packed footprint</sub><br>Skylofts at MGM Grand</td>
    <td align="center"><a href="docs/assets/vegas/venues/comfort-inn.png"><img src="docs/assets/vegas/venues/comfort-inn.png" width="105" alt="Sleep Inn &amp; Suites"></a><br><sub>Nearby packed footprint</sub><br>Sleep Inn &amp; Suites</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/vegas/venues/sonesta-select-las-vegas-summerlin.png"><img src="docs/assets/vegas/venues/sonesta-select-las-vegas-summerlin.png" width="105" alt="Sonesta Select Las Vegas Summerlin"></a><br><sub>Generic packed footprint</sub><br>Sonesta Select Las Vegas Summerlin</td>
    <td align="center"><a href="docs/assets/vegas/venues/spark-by-hilton.png"><img src="docs/assets/vegas/venues/spark-by-hilton.png" width="105" alt="Spark by Hilton"></a><br><sub>Generic packed footprint</sub><br>Spark by Hilton</td>
    <td align="center"><a href="docs/assets/vegas/venues/springhill-suites.png"><img src="docs/assets/vegas/venues/springhill-suites.png" width="105" alt="SpringHill Suites"></a><br><sub>Generic packed footprint</sub><br>SpringHill Suites</td>
    <td align="center"><a href="docs/assets/vegas/venues/springhill-suites-las-vegas-airport.png"><img src="docs/assets/vegas/venues/springhill-suites-las-vegas-airport.png" width="105" alt="SpringHill Suites Las Vegas Airport"></a><br><sub>Generic packed footprint</sub><br>SpringHill Suites Las Vegas Airport</td>
    <td align="center"><a href="docs/assets/vegas/venues/springhill-suites-by-marriott-las-vegas-convention-center.png"><img src="docs/assets/vegas/venues/springhill-suites-by-marriott-las-vegas-convention-center.png" width="105" alt="Springhill Suites by Marriott Las Vegas Convention Center"></a><br><sub>Generic packed footprint</sub><br>Springhill Suites by Marriott Las Vegas Convention Center</td>
    <td align="center"><a href="docs/assets/vegas/venues/staybridge-suites-las-vegas.png"><img src="docs/assets/vegas/venues/staybridge-suites-las-vegas.png" width="105" alt="Staybridge Suites Las Vegas"></a><br><sub>Generic packed footprint</sub><br>Staybridge Suites Las Vegas</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/vegas/venues/sterling-gardens-hotel.png"><img src="docs/assets/vegas/venues/sterling-gardens-hotel.png" width="105" alt="Sterling Gardens Hotel"></a><br><sub>Generic packed footprint</sub><br>Sterling Gardens Hotel</td>
    <td align="center"><a href="docs/assets/vegas/venues/sunrise-inn.png"><img src="docs/assets/vegas/venues/sunrise-inn.png" width="105" alt="Sunrise Inn"></a><br><sub>Generic packed footprint</sub><br>Sunrise Inn</td>
    <td align="center"><a href="docs/assets/vegas/venues/tahiti-suites.png"><img src="docs/assets/vegas/venues/tahiti-suites.png" width="105" alt="Tahiti Suites"></a><br><sub>Generic packed footprint</sub><br>Tahiti Suites</td>
    <td align="center"><a href="docs/assets/vegas/venues/tahiti-village-resort.png"><img src="docs/assets/vegas/venues/tahiti-village-resort.png" width="105" alt="Tahiti Village Resort"></a><br><sub>Generic packed footprint</sub><br>Tahiti Village Resort</td>
    <td align="center"><a href="docs/assets/vegas/venues/the-berkley-las-vegas.png"><img src="docs/assets/vegas/venues/the-berkley-las-vegas.png" width="105" alt="The Berkley, Las Vegas"></a><br><sub>Generic packed footprint</sub><br>The Berkley, Las Vegas</td>
    <td align="center"><a href="docs/assets/vegas/venues/the-carriage-house.png"><img src="docs/assets/vegas/venues/the-carriage-house.png" width="105" alt="The Carriage House"></a><br><sub>Generic packed footprint</sub><br>The Carriage House</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/vegas/venues/the-club-las.png"><img src="docs/assets/vegas/venues/the-club-las.png" width="105" alt="The Club LAS"></a><br><sub>Generic packed footprint</sub><br>The Club LAS</td>
    <td align="center"><a href="docs/assets/vegas/venues/the-cosmopolitan-of-las-vegas.png"><img src="docs/assets/vegas/venues/the-cosmopolitan-of-las-vegas.png" width="105" alt="The Cosmopolitan of Las Vegas"></a><br><sub>Generic packed footprint</sub><br>The Cosmopolitan of Las Vegas</td>
    <td align="center"><a href="docs/assets/vegas/venues/the-d-las-vegas.png"><img src="docs/assets/vegas/venues/the-d-las-vegas.png" width="105" alt="The D Las Vegas"></a><br><sub>Generic packed footprint</sub><br>The D Las Vegas</td>
    <td align="center"><a href="docs/assets/vegas/venues/the-diplomat-apartment.png"><img src="docs/assets/vegas/venues/the-diplomat-apartment.png" width="105" alt="The Diplomat Apartment"></a><br><sub>Generic packed footprint</sub><br>The Diplomat Apartment</td>
    <td align="center"><a href="docs/assets/vegas/venues/the-english-hotel.png"><img src="docs/assets/vegas/venues/the-english-hotel.png" width="105" alt="The English Hotel"></a><br><sub>Generic packed footprint</sub><br>The English Hotel</td>
    <td align="center"><a href="docs/assets/vegas/venues/the-mansion-at-mgm-grand.png"><img src="docs/assets/vegas/venues/the-mansion-at-mgm-grand.png" width="105" alt="The Mansion at MGM Grand"></a><br><sub>Generic packed footprint</sub><br>The Mansion at MGM Grand</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/vegas/venues/the-rita-suites.png"><img src="docs/assets/vegas/venues/the-rita-suites.png" width="105" alt="The Rita Suites"></a><br><sub>Generic packed footprint</sub><br>The Rita Suites</td>
    <td align="center"><a href="docs/assets/vegas/venues/the-signature-at-mgm-grand.png"><img src="docs/assets/vegas/venues/the-signature-at-mgm-grand.png" width="105" alt="The Signature at MGM Grand"></a><br><sub>Generic packed footprint</sub><br>The Signature at MGM Grand</td>
    <td align="center"><a href="docs/assets/vegas/venues/the-westin-las-vegas-hotel.png"><img src="docs/assets/vegas/venues/the-westin-las-vegas-hotel.png" width="105" alt="The Westin Las Vegas Hotel"></a><br><sub>Generic packed footprint</sub><br>The Westin Las Vegas Hotel</td>
    <td align="center"><a href="docs/assets/vegas/venues/towneplace-suites.png"><img src="docs/assets/vegas/venues/towneplace-suites.png" width="105" alt="TownePlace Suites"></a><br><sub>Generic packed footprint</sub><br>TownePlace Suites</td>
    <td align="center"><a href="docs/assets/vegas/venues/fairfield-inn-suites-las-vegas-airport-south.png"><img src="docs/assets/vegas/venues/fairfield-inn-suites-las-vegas-airport-south.png" width="105" alt="TownePlace Suites Las Vegas Airport South"></a><br><sub>Generic packed footprint</sub><br>TownePlace Suites Las Vegas Airport South</td>
    <td align="center"><a href="docs/assets/vegas/venues/towneplace-suites-las-vegas-stadium-district.png"><img src="docs/assets/vegas/venues/towneplace-suites-las-vegas-stadium-district.png" width="105" alt="TownePlace Suites Las Vegas Stadium District"></a><br><sub>Generic packed footprint</sub><br>TownePlace Suites Las Vegas Stadium District</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/vegas/venues/travelodge-las-vegas.png"><img src="docs/assets/vegas/venues/travelodge-las-vegas.png" width="105" alt="Travelodge Las Vegas"></a><br><sub>Generic packed footprint</sub><br>Travelodge Las Vegas</td>
    <td align="center"><a href="docs/assets/vegas/venues/days-inn.png"><img src="docs/assets/vegas/venues/days-inn.png" width="105" alt="Travelodge Las Vegas Airport North/Near the Strip"></a><br><sub>Generic packed footprint</sub><br>Travelodge Las Vegas Airport North/Near the Strip</td>
    <td align="center"><a href="docs/assets/vegas/venues/tru-by-hilton-las-vegas-airport.png"><img src="docs/assets/vegas/venues/tru-by-hilton-las-vegas-airport.png" width="105" alt="Tru by Hilton Las Vegas Airport"></a><br><sub>Generic packed footprint</sub><br>Tru by Hilton Las Vegas Airport</td>
    <td align="center"><a href="docs/assets/vegas/venues/trump-international-hotel-las-vegas.png"><img src="docs/assets/vegas/venues/trump-international-hotel-las-vegas.png" width="105" alt="Trump International Hotel Las Vegas"></a><br><sub>Generic packed footprint</sub><br>Trump International Hotel Las Vegas</td>
    <td align="center"><a href="docs/assets/vegas/venues/vanderpump.png"><img src="docs/assets/vegas/venues/vanderpump.png" width="105" alt="Vanderpump"></a><br><sub>Generic packed footprint</sub><br>Vanderpump</td>
    <td align="center"><a href="docs/assets/vegas/venues/vdara-hotel-spa.png"><img src="docs/assets/vegas/venues/vdara-hotel-spa.png" width="105" alt="Vdara Hotel &amp; Spa"></a><br><sub>Generic packed footprint</sub><br>Vdara Hotel &amp; Spa</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/vegas/venues/virgin-hotels-las-vegas-curio-collection-by-hilton.png"><img src="docs/assets/vegas/venues/virgin-hotels-las-vegas-curio-collection-by-hilton.png" width="105" alt="Virgin Hotels Las Vegas, Curio Collection by Hilton"></a><br><sub>Generic packed footprint</sub><br>Virgin Hotels Las Vegas, Curio Collection by Hilton</td>
    <td align="center"><a href="docs/assets/vegas/venues/w-las-vegas.png"><img src="docs/assets/vegas/venues/w-las-vegas.png" width="105" alt="W Las Vegas"></a><br><sub>Generic packed footprint</sub><br>W Las Vegas</td>
    <td align="center"><a href="docs/assets/vegas/venues/waldorf-astoria-las-vegas.png"><img src="docs/assets/vegas/venues/waldorf-astoria-las-vegas.png" width="105" alt="Waldorf Astoria Las Vegas"></a><br><sub>Generic packed footprint</sub><br>Waldorf Astoria Las Vegas</td>
    <td align="center"><a href="docs/assets/vegas/venues/wildfire-rancho.png"><img src="docs/assets/vegas/venues/wildfire-rancho.png" width="105" alt="Wildfire Rancho"></a><br><sub>Nearby packed footprint</sub><br>Wildfire Rancho</td>
    <td align="center"><a href="docs/assets/vegas/venues/worldmark-las-vegas.png"><img src="docs/assets/vegas/venues/worldmark-las-vegas.png" width="105" alt="WorldMark Las Vegas"></a><br><sub>Nearby packed footprint</sub><br>WorldMark Las Vegas</td>
    <td align="center"><a href="docs/assets/vegas/venues/worldmark-las-vegas-boulevard.png"><img src="docs/assets/vegas/venues/worldmark-las-vegas-boulevard.png" width="105" alt="WorldMark Las Vegas - Boulevard"></a><br><sub>Nearby packed footprint</sub><br>WorldMark Las Vegas - Boulevard</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/vegas/venues/worldmark-las-vegas-tropicana-avenue.png"><img src="docs/assets/vegas/venues/worldmark-las-vegas-tropicana-avenue.png" width="105" alt="WorldMark Las Vegas - Tropicana Avenue"></a><br><sub>Generic packed footprint</sub><br>WorldMark Las Vegas - Tropicana Avenue</td>
    <td align="center"><a href="docs/assets/vegas/venues/wyndham-desert-blue.png"><img src="docs/assets/vegas/venues/wyndham-desert-blue.png" width="105" alt="Wyndham Desert Blue"></a><br><sub>Generic packed footprint</sub><br>Wyndham Desert Blue</td>
    <td align="center"><a href="docs/assets/vegas/venues/wyndham-grand-desert.png"><img src="docs/assets/vegas/venues/wyndham-grand-desert.png" width="105" alt="Wyndham Grand Desert"></a><br><sub>Generic packed footprint</sub><br>Wyndham Grand Desert</td>
    <td align="center"><a href="docs/assets/vegas/venues/wynn-las-vegas.png"><img src="docs/assets/vegas/venues/wynn-las-vegas.png" width="105" alt="Wynn Las Vegas"></a><br><sub>Generic packed footprint</sub><br>Wynn Las Vegas</td>
  </tr>
</table>

### Bellagio

<table>
  <tr>
    <td align="center"><a href="docs/assets/vegas/venues/bellagio-conservatory-roof-way-114718351.png"><img src="docs/assets/vegas/venues/bellagio-conservatory-roof-way-114718351.png" width="105" alt="Bellagio Conservatory roof"></a><br><sub>Reviewed packed component</sub><br>Bellagio Conservatory roof</td>
    <td align="center"><a href="docs/assets/vegas/venues/bellagio-spa-tower-way-114716301.png"><img src="docs/assets/vegas/venues/bellagio-spa-tower-way-114716301.png" width="105" alt="Bellagio Spa Tower"></a><br><sub>Reviewed packed component</sub><br>Bellagio Spa Tower</td>
    <td align="center"><a href="docs/assets/vegas/venues/bellagio-spa-tower-base-way-25723910.png"><img src="docs/assets/vegas/venues/bellagio-spa-tower-base-way-25723910.png" width="105" alt="Bellagio Spa Tower base"></a><br><sub>Reviewed packed component</sub><br>Bellagio Spa Tower base</td>
    <td align="center"><a href="docs/assets/vegas/venues/bellagio-domed-crown-way-114703388.png"><img src="docs/assets/vegas/venues/bellagio-domed-crown-way-114703388.png" width="105" alt="Bellagio domed crown"></a><br><sub>Reviewed packed component</sub><br>Bellagio domed crown</td>
    <td align="center"><a href="docs/assets/vegas/venues/bellagio-main-tower-way-25723909.png"><img src="docs/assets/vegas/venues/bellagio-main-tower-way-25723909.png" width="105" alt="Bellagio main tower"></a><br><sub>Reviewed packed component</sub><br>Bellagio main tower</td>
    <td align="center"><a href="docs/assets/vegas/venues/bellagio-north-entrance-roof-way-114714407.png"><img src="docs/assets/vegas/venues/bellagio-north-entrance-roof-way-114714407.png" width="105" alt="Bellagio north entrance roof"></a><br><sub>Reviewed packed component</sub><br>Bellagio north entrance roof</td>
  </tr>
  <tr>
    <td align="center"><a href="docs/assets/vegas/venues/bellagio-podium-relation-1599958.png"><img src="docs/assets/vegas/venues/bellagio-podium-relation-1599958.png" width="105" alt="Bellagio podium"></a><br><sub>Reviewed packed component</sub><br>Bellagio podium</td>
  </tr>
</table>

### Caesars Palace

<table>
  <tr>
    <td align="center"><a href="docs/assets/vegas/venues/augustus-tower-way-1176928048.png"><img src="docs/assets/vegas/venues/augustus-tower-way-1176928048.png" width="105" alt="Augustus Tower"></a><br><sub>Reviewed packed component</sub><br>Augustus Tower</td>
    <td align="center"><a href="docs/assets/vegas/venues/octavius-tower-way-126267789.png"><img src="docs/assets/vegas/venues/octavius-tower-way-126267789.png" width="105" alt="Octavius Tower"></a><br><sub>Reviewed packed component</sub><br>Octavius Tower</td>
  </tr>
</table>

### Flamingo

<table>
  <tr>
    <td align="center"><a href="docs/assets/vegas/venues/flamingo-casino-podium-way-116761428.png"><img src="docs/assets/vegas/venues/flamingo-casino-podium-way-116761428.png" width="105" alt="Flamingo casino podium"></a><br><sub>Reviewed packed component</sub><br>Flamingo casino podium</td>
    <td align="center"><a href="docs/assets/vegas/venues/flamingo-low-rise-wing-way-310585854.png"><img src="docs/assets/vegas/venues/flamingo-low-rise-wing-way-310585854.png" width="105" alt="Flamingo low-rise wing"></a><br><sub>Reviewed packed component</sub><br>Flamingo low-rise wing</td>
    <td align="center"><a href="docs/assets/vegas/venues/flamingo-north-tower-way-115766957.png"><img src="docs/assets/vegas/venues/flamingo-north-tower-way-115766957.png" width="105" alt="Flamingo north tower"></a><br><sub>Reviewed packed component</sub><br>Flamingo north tower</td>
    <td align="center"><a href="docs/assets/vegas/venues/flamingo-south-tower-way-115766962.png"><img src="docs/assets/vegas/venues/flamingo-south-tower-way-115766962.png" width="105" alt="Flamingo south tower"></a><br><sub>Reviewed packed component</sub><br>Flamingo south tower</td>
    <td align="center"><a href="docs/assets/vegas/venues/flamingo-west-roof-section-way-116761280.png"><img src="docs/assets/vegas/venues/flamingo-west-roof-section-way-116761280.png" width="105" alt="Flamingo west roof section"></a><br><sub>Reviewed packed component</sub><br>Flamingo west roof section</td>
    <td align="center"><a href="docs/assets/vegas/venues/flamingo-west-wing-way-116761035.png"><img src="docs/assets/vegas/venues/flamingo-west-wing-way-116761035.png" width="105" alt="Flamingo west wing"></a><br><sub>Reviewed packed component</sub><br>Flamingo west wing</td>
  </tr>
</table>

### Horseshoe

<table>
  <tr>
    <td align="center"><a href="docs/assets/vegas/venues/horseshoe-resort-tower-way-27897922.png"><img src="docs/assets/vegas/venues/horseshoe-resort-tower-way-27897922.png" width="105" alt="Horseshoe Resort Tower"></a><br><sub>Reviewed packed component</sub><br>Horseshoe Resort Tower</td>
    <td align="center"><a href="docs/assets/vegas/venues/horseshoe-east-pavilion-way-1582510991.png"><img src="docs/assets/vegas/venues/horseshoe-east-pavilion-way-1582510991.png" width="105" alt="Horseshoe east pavilion"></a><br><sub>Reviewed packed component</sub><br>Horseshoe east pavilion</td>
    <td align="center"><a href="docs/assets/vegas/venues/horseshoe-east-podium-way-1582510990.png"><img src="docs/assets/vegas/venues/horseshoe-east-podium-way-1582510990.png" width="105" alt="Horseshoe east podium"></a><br><sub>Reviewed packed component</sub><br>Horseshoe east podium</td>
    <td align="center"><a href="docs/assets/vegas/venues/horseshoe-podium-way-116867081.png"><img src="docs/assets/vegas/venues/horseshoe-podium-way-116867081.png" width="105" alt="Horseshoe podium"></a><br><sub>Reviewed packed component</sub><br>Horseshoe podium</td>
    <td align="center"><a href="docs/assets/vegas/venues/horseshoe-west-podium-way-1582510992.png"><img src="docs/assets/vegas/venues/horseshoe-west-podium-way-1582510992.png" width="105" alt="Horseshoe west podium"></a><br><sub>Reviewed packed component</sub><br>Horseshoe west podium</td>
  </tr>
</table>

### Paris

<table>
  <tr>
    <td align="center"><a href="docs/assets/vegas/venues/paris-casino-podium-way-33974140.png"><img src="docs/assets/vegas/venues/paris-casino-podium-way-33974140.png" width="105" alt="Paris casino podium"></a><br><sub>Reviewed packed component</sub><br>Paris casino podium</td>
    <td align="center"><a href="docs/assets/vegas/venues/paris-main-tower-way-27831697.png"><img src="docs/assets/vegas/venues/paris-main-tower-way-27831697.png" width="105" alt="Paris main tower"></a><br><sub>Reviewed packed component</sub><br>Paris main tower</td>
    <td align="center"><a href="docs/assets/vegas/venues/versailles-tower-way-27897921.png"><img src="docs/assets/vegas/venues/versailles-tower-way-27897921.png" width="105" alt="Versailles Tower"></a><br><sub>Reviewed packed component</sub><br>Versailles Tower</td>
  </tr>
</table>

### Planet Hollywood

<table>
  <tr>
    <td align="center"><a href="docs/assets/vegas/venues/planet-hollywood-main-tower-way-134921373.png"><img src="docs/assets/vegas/venues/planet-hollywood-main-tower-way-134921373.png" width="105" alt="Planet Hollywood main tower"></a><br><sub>Reviewed packed component</sub><br>Planet Hollywood main tower</td>
  </tr>
</table>

### The Cosmopolitan

<table>
  <tr>
    <td align="center"><a href="docs/assets/vegas/venues/cosmopolitan-boulevard-tower-way-134795273.png"><img src="docs/assets/vegas/venues/cosmopolitan-boulevard-tower-way-134795273.png" width="105" alt="Cosmopolitan Boulevard Tower"></a><br><sub>Reviewed packed component</sub><br>Cosmopolitan Boulevard Tower</td>
    <td align="center"><a href="docs/assets/vegas/venues/cosmopolitan-boulevard-low-rise-way-472669744.png"><img src="docs/assets/vegas/venues/cosmopolitan-boulevard-low-rise-way-472669744.png" width="105" alt="Cosmopolitan Boulevard low-rise"></a><br><sub>Reviewed packed component</sub><br>Cosmopolitan Boulevard low-rise</td>
    <td align="center"><a href="docs/assets/vegas/venues/cosmopolitan-chelsea-tower-way-27897905.png"><img src="docs/assets/vegas/venues/cosmopolitan-chelsea-tower-way-27897905.png" width="105" alt="Cosmopolitan Chelsea Tower"></a><br><sub>Reviewed packed component</sub><br>Cosmopolitan Chelsea Tower</td>
  </tr>
</table>

### The LINQ

<table>
  <tr>
    <td align="center"><a href="docs/assets/vegas/venues/linq-casino-podium-way-115771295.png"><img src="docs/assets/vegas/venues/linq-casino-podium-way-115771295.png" width="105" alt="LINQ casino podium"></a><br><sub>Reviewed packed component</sub><br>LINQ casino podium</td>
    <td align="center"><a href="docs/assets/vegas/venues/linq-hotel-tower-way-115773426.png"><img src="docs/assets/vegas/venues/linq-hotel-tower-way-115773426.png" width="105" alt="LINQ hotel tower"></a><br><sub>Reviewed packed component</sub><br>LINQ hotel tower</td>
  </tr>
</table>

### The Palazzo

<table>
  <tr>
    <td align="center"><a href="docs/assets/vegas/venues/palazzo-tower-way-30424125.png"><img src="docs/assets/vegas/venues/palazzo-tower-way-30424125.png" width="105" alt="Palazzo Tower"></a><br><sub>Reviewed packed component</sub><br>Palazzo Tower</td>
    <td align="center"><a href="docs/assets/vegas/venues/palazzo-podium-way-180584269.png"><img src="docs/assets/vegas/venues/palazzo-podium-way-180584269.png" width="105" alt="Palazzo podium"></a><br><sub>Reviewed packed component</sub><br>Palazzo podium</td>
  </tr>
</table>

### The Venetian

<table>
  <tr>
    <td align="center"><a href="docs/assets/vegas/venues/campanile-way-43875964.png"><img src="docs/assets/vegas/venues/campanile-way-43875964.png" width="105" alt="Campanile"></a><br><sub>Reviewed packed component</sub><br>Campanile</td>
    <td align="center"><a href="docs/assets/vegas/venues/venetian-expo-way-111367819.png"><img src="docs/assets/vegas/venues/venetian-expo-way-111367819.png" width="105" alt="Venetian Expo"></a><br><sub>Reviewed packed component</sub><br>Venetian Expo</td>
    <td align="center"><a href="docs/assets/vegas/venues/venetian-tower-way-30618090.png"><img src="docs/assets/vegas/venues/venetian-tower-way-30618090.png" width="105" alt="Venetian Tower"></a><br><sub>Reviewed packed component</sub><br>Venetian Tower</td>
    <td align="center"><a href="docs/assets/vegas/venues/venetian-podium-relation-7770314.png"><img src="docs/assets/vegas/venues/venetian-podium-relation-7770314.png" width="105" alt="Venetian podium"></a><br><sub>Reviewed packed component</sub><br>Venetian podium</td>
    <td align="center"><a href="docs/assets/vegas/venues/venezia-tower-way-111322067.png"><img src="docs/assets/vegas/venues/venezia-tower-way-111322067.png" width="105" alt="Venezia Tower"></a><br><sub>Reviewed packed component</sub><br>Venezia Tower</td>
  </tr>
</table>

## World generation

Metropolises cover a broad basin with connected residential, commercial, and industrial districts. Seed 15 spans approximately **23,936 × 24,704 world units**. Ordinary settlements grow along regional roads.

<a href="docs/assets/planning/metropolis-seed-15.png"><img src="docs/assets/planning/metropolis-seed-15.png" width="640" alt="Map of the full metropolis from seed 15"></a>

<table>
  <tr>
    <td align="center"><a href="docs/assets/planning/urban-149--6--4.png"><img src="docs/assets/planning/urban-149--6--4.png" width="320" alt="Map of generated streets and buildings"></a><br>Largest developed footprint · seed 149</td>
    <td align="center"><a href="docs/assets/planning/urban-241--2--4.png"><img src="docs/assets/planning/urban-241--2--4.png" width="320" alt="Map of generated districts and approach roads"></a><br>Longest surveyed settlement · seed 241</td>
  </tr>
</table>

Metropolitan airports connect two runways to a shared gate apron and a city street.

<a href="docs/assets/planning/metropolitan-airport.png"><img src="docs/assets/planning/metropolitan-airport.png" width="640" alt="Map of the metropolitan airport from seed 19"></a>

Metropolitan trains stop at elevated stations and connect the airport to two city corridors.

<a href="docs/assets/planning/metro-rail-seed-15.png"><img src="docs/assets/planning/metro-rail-seed-15.png" width="640" alt="Metropolitan rail routes and stations in seed 15"></a>

Major city roads continue into villages, farmland, and wooded foothills.

<a href="docs/assets/planning/outskirts-seed-15.png"><img src="docs/assets/planning/outskirts-seed-15.png" width="640" alt="Metropolitan outskirts in seed 15"></a>

[City dimensions, street grids, and placement rules](docs/WORLD-GENERATION.md)

<!-- END GALLERY -->

## Controls

| When | Right button | Left button |
| --- | --- | --- |
| Aircraft picker | Browse the eight aircraft | Take off |
| In flight | Tap for flight info. Hold until the progress bar fills to return to the picker and start a new world. | — |

The first picker launches a random aircraft after ten seconds if no buttons are pressed.

## Install and build

Generate the scenery artwork directly from the game’s geometry with `./tools/export-artwork.sh`.

[Open the firmware flasher](https://xn--sg8hhb.ws/flasher/?project=scenic-flight) for compatible round Waveshare units. See the [latest GitHub release](https://github.com/xberenguer/scenic-flight/releases/latest) for the firmware and checksum.

The web flasher installs the standalone procedural game. For Los Angeles and Las Vegas on the same handheld, download the Southwest bundle from the latest release and follow its install guide.

Test the Rust simulation and render a seeded desktop frame.

```sh
cargo test --locked --features terrain-fork,small-raster-ray-budget,ground-cover
cargo run --locked --release --features ground-cover --bin flight-preview -- render 11 6 10 0.46 240 preview.ppm detailed
```

The Waveshare app is built with `board/build-app.sh` using an ESP32-S3 Xtensa Rust toolchain. See the [board build notes](board/BUILD-NOTES.md) for the toolchain and app partition details.

Original inspiration: [Voxel Flight by lewtheegg](https://github.com/lewtheegg/voxel-flight).
