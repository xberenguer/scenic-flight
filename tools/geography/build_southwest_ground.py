#!/usr/bin/env python3
"""Pack a USGS GeoTIFF into the atlas's 150 m UTM 11N elevation layer."""

import argparse
import gzip
from pathlib import Path
import struct

import numpy as np
import rasterio
from rasterio.enums import Resampling
from rasterio.transform import Affine
from rasterio.warp import reproject

ORIGIN = (384629, 3768405)
STEP = 150
WEST, NORTH = -43200, 263100
WIDTH, HEIGHT = 2441, 2042


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("input", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    elevation = np.full((HEIGHT, WIDTH), np.nan, dtype=np.float32)
    transform = Affine.translation(ORIGIN[0] + WEST, ORIGIN[1] + NORTH) * Affine.scale(STEP, -STEP)
    with rasterio.open(args.input) as source:
        reproject(rasterio.band(source, 1), elevation, src_transform=source.transform,
                  src_crs=source.crs, src_nodata=source.nodata, dst_crs="EPSG:32611",
                  dst_transform=transform, dst_nodata=np.nan, resampling=Resampling.bilinear)
    if not np.isfinite(elevation).all():
        raise ValueError("USGS source does not cover the entire Southwest grid")
    header = struct.pack("<4s5i", b"SWE1", WEST, NORTH, STEP, WIDTH, HEIGHT)
    with args.output.open("wb") as output:
        with gzip.GzipFile(fileobj=output, mode="wb", mtime=0, compresslevel=6) as compressed:
            compressed.write(header)
            compressed.write(elevation.round().astype("<i2").tobytes())


if __name__ == "__main__":
    main()
