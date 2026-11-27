#!/usr/bin/env python3
"""Render reference maps from the same packed data sampled by the game."""
import argparse
import gzip
from pathlib import Path
import struct

import numpy as np
from rasterio.io import MemoryFile

MAPS = {
    "strip": (-1750, -2700, 2600, 4650, 7),
    "resorts": (-840, -480, 1200, 1710, 3),
    "airport": (-850, -5800, 5350, -540, 7),
    "downtown": (275, 4580, 4780, 8360, 7),
    "wide": (-34000, -12500, 44000, 22700, 90),
}

COLORS = {
    1: (68, 72, 77), 2: (65, 128, 161), 3: (157, 153, 145), 5: (241, 238, 217),
    6: (35, 42, 47), 7: (92, 124, 81), 8: (104, 94, 80), 10: (98, 179, 213),
    19: (212, 177, 134), 20: (43, 39, 45),
    21: (162, 154, 142), 22: (201, 195, 182), 23: (172, 152, 157),
    24: (119, 131, 138), 25: (123, 112, 104), 26: (127, 92, 75),
    27: (110, 132, 148), 28: (152, 139, 118), 29: (151, 123, 107),
    30: (154, 157, 157),
}


def save_raster(file, array):
    with MemoryFile() as memory:
        with memory.open(driver="PNG", width=array.shape[1], height=array.shape[0],
                         count=3, dtype="uint8") as raster:
            raster.write(np.moveaxis(array, 2, 0))
        file.write_bytes(memory.read())


def plot(pack, directory):
    data = gzip.decompress(pack.read_bytes())
    _, west, north, cell, dw, dh, bx, bz, step, tw, th, tile_count, _, _, _, _ = struct.unpack_from("<4s15i", data)
    dem = np.frombuffer(data, dtype="<u2", count=dw*dh, offset=64).reshape(dh, dw)
    water = np.frombuffer(data, dtype="uint8", count=dw*dh, offset=64+dw*dh*2).reshape(dh, dw)
    index_at = 64 + dw*dh*3
    index = np.frombuffer(data, dtype="<u4", count=tw*th, offset=index_at).reshape(th,tw)
    blocks = np.frombuffer(data, dtype="<u2", count=tile_count*256, offset=index_at+tw*th*4).reshape(tile_count,256)
    for name, (left, bottom, right, top, spacing) in MAPS.items():
        x, z = np.meshgrid(np.arange(left+spacing//2,right,spacing), np.arange(top-spacing//2,bottom,-spacing))
        dx = np.clip((x-west)//cell, 0, dw-1)
        dz = np.clip((north-z)//cell, 0, dh-1)
        relief = np.clip(dem[dz,dx].astype(np.int16)-390, 0, 1500) * 0.047
        picture = np.stack([129+relief, 112+relief, 91+relief], axis=2).astype(np.uint8)
        slope = dem[dz,np.minimum(dx+1,dw-1)].astype(np.int16) - dem[dz,np.maximum(dx-1,0)].astype(np.int16)
        picture = np.clip(picture.astype(np.int16)+np.clip(slope/4,-24,24)[:,:,None],0,255).astype(np.uint8)
        picture[water[dz,dx]>0] = COLORS[2]
        px, pz = (x-bx)//step, (z-bz)//step
        valid = (px>=0)&(pz>=0)&(px<tw*16)&(pz<th*16)
        tile = index[np.clip(pz//16,0,th-1),np.clip(px//16,0,tw-1)]
        value = blocks[np.maximum(tile,1)-1,(pz%16)*16+px%16]
        material = np.where(valid & (tile>0), value & 127, 0)
        for key, color in COLORS.items():
            picture[material==key] = color
        buildings = (material>=11)&(material<=18)
        roof = np.clip(141+(value>>7)/3,140,237).astype(np.uint8)
        picture[buildings] = np.stack((roof,np.clip(roof.astype(np.int16)-19,0,255),np.clip(roof.astype(np.int16)-31,0,255)),axis=2)[buildings]
        save_raster(directory/f"las-vegas-{name}.png",picture)


if __name__ == "__main__":
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("pack",type=Path)
    parser.add_argument("directory",type=Path)
    options=parser.parse_args()
    options.directory.mkdir(parents=True,exist_ok=True)
    plot(options.pack,options.directory)
