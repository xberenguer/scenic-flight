# Scenic Flight

A Rust flight simulator for the round Waveshare ESP32-S3-Touch-AMOLED-1.75C. Pick an aircraft and watch it fly through a new procedural world.

## Visual showcase

The [gameplay showcase and artwork gallery](https://xberenguer.github.io/scenic-flight/) collects current gameplay clips, aircraft turntables, picker screens, and rare encounters.

[Play in your browser](https://xberenguer.github.io/scenic-flight/play.html), or run the gallery locally from the repository root.

```sh
python3 -m http.server 8775 --bind 127.0.0.1 --directory docs
```

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

[Open the firmware flasher](https://xn--sg8hhb.ws/flasher/?project=scenic-flight) for compatible round Waveshare units. See the [latest GitHub release](https://github.com/xberenguer/scenic-flight/releases/latest) for the firmware and checksum.

Test the Rust simulation and render a seeded desktop frame.

```sh
cargo test --locked --features terrain-fork,small-raster-ray-budget,ground-cover
cargo run --locked --release --features ground-cover --bin flight-preview -- render 11 6 10 0.46 240 preview.ppm detailed
```

The Waveshare app is built with `board/build-app.sh` using an ESP32-S3 Xtensa Rust toolchain. See the [board build notes](board/BUILD-NOTES.md) for the toolchain and app partition details.

Original inspiration: [Voxel Flight by lewtheegg](https://github.com/lewtheegg/voxel-flight).
