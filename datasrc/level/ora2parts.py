#!/usr/bin/env -S uv run --script
#
# /// script
# dependencies = ["pyora", "tomlkit"]
# ///

from PIL import Image, ImageDraw
import numpy
import itertools
import shutil
from os import path
import sys
import re

import tomlkit
import pyora

# This matches everything that looks like a terrain type string, even if its not actually
# supported by the game engine itself.
TERRAIN_TYPE_RE = re.compile(r'^([a-z]+(?:-(?:uw|i|dyn))*)(\+nocolor)?$')

PALETTE = [
    (0, 0, 0),
    (255, 255, 255),
    (0, 0, 255),
    (255, 255, 0),
    (255, 0, 0),
    (0, 255, 0),
    (255, 0, 255),
    (127, 0, 0),
    (0, 127, 0),
    (0, 0, 127),
    (127, 127, 0),
    (127, 0, 127),
    (127, 127, 127),
    (64, 0, 0),
    (0, 64, 0),
    (0, 0, 64),
    (64, 64, 0),
    (64, 0, 64),
    (64, 64, 64),
]


def intersect_map_bounds(map_w, map_h, x, y, w, h):
    x0 = max(0, x)
    y0 = max(0, y)
    x1 = min(x+w, map_w)
    y1 = min(y+h, map_h)

    return (x0, y0, x1, y1)


def get_terrain_name_part(layer_name):
    terrain_name = TERRAIN_TYPE_RE.match(layer_name)
    if not terrain_name:
        raise ValueError(f"Layer name {layer_name} does not match terrain name regex!")

    return terrain_name.group(1)


def render_collisionmap(project):
    map_w, map_h = project.dimensions
    imagedata = numpy.zeros((map_h, map_w), dtype=numpy.uint8)
    colormap = {'space': 0, 'ground': 1, 'water': 2} # terrain type name -> palette index

    def get_terrain_color(name):
        try:
            return colormap[name]
        except KeyError:
            idx = len(colormap)
            colormap[name] = idx
            return idx

    def paint_layer(layer):
        image = layer.get_image_data(raw=True)
        x, y = layer.offsets
        w, h = image.size

        x, y, x2, y2 = intersect_map_bounds(map_w, map_h, x, y, w, h)

        if w != x2-x or h != y2-y:
            image = image.crop((x - layer.offsets[0], y - layer.offsets[1], x2 - layer.offsets[0], y2 - layer.offsets[1]))
            w, h = image.size

        terrain_name = get_terrain_name_part(layer.name)
        color_idx = get_terrain_color(terrain_name)
        mask = numpy.array(image.get_flattened_data(3)).reshape(h, w) > 0
        imagedata[y:y2, x:x2] = imagedata[y:y2, x:x2] * ~mask + color_idx * mask

    def paint(stack, prefix=''):
        for layer in reversed(list(stack.children)):
            if not layer.visible:
                print(prefix, "not visible:", layer.name)
                continue

            if TERRAIN_TYPE_RE.match(layer.name):
                print(prefix, "paint:", layer.name)
                paint_layer(layer)
            elif isinstance(layer, pyora.Group):
                print(prefix, "group:", layer.name)
                paint(layer, prefix + '  ')
            else:
                print(prefix, "skip", layer.name)

    paint(project.root)

    image = Image.fromarray(imagedata, 'P')
    image.putpalette(list(itertools.chain(*PALETTE[:len(colormap)])))
    return image, colormap


def calculate_bounds(offset, layer):
    offset = (offset[0] + layer.offsets[0], offset[1] + layer.offsets[1])
    if layer.is_group:
        # layer.dimensions currently not implemented for groups
        dims = [calculate_bounds(offset, l) for l in layer]
        x, y, w, h = dims[0]
        left, top, right, bottom = x, y, x+w, y+h
        for d in dims[1:]:
            left = min(x, d[0])
            top = min(y, d[1])
            right = max(right, d[0] + d[2])
            bottom = max(bottom, d[1] + d[3])

        return (left, top, right-left, bottom-top)
    else:
        return (*offset, *layer.dimensions)


def get_toplevel_group_bounds(project):
    subimages = {}
    for layer in project.root:
        if isinstance(layer, pyora.Group):
            subimages[layer.name] = calculate_bounds((0, 0), layer)
        else:
            print("Warning: top-level item", layer.name, "is not a group!")

    if not subimages:
        subimages["image"] = [0, 0, *project.dimensions]

    return subimages


def main(input_path, target_dir):
    root, _ = path.splitext(input_path)
    basename = path.basename(root)

    # Output file names
    toml_filename = basename + ".toml"
    artwork_filename = basename + "-art.png"
    terrain_filename = basename + "-terrain.png"

    levelinfo = {
        "artwork": artwork_filename,
        "terrain": terrain_filename,
    }

    # Load OpenRaster file
    project = pyora.Project.load(input_path)
    print("Image size is", project.dimensions)

    # Extract subimage regions
    levelinfo["subimages"] = get_toplevel_group_bounds(project)

    # Make collisionmap
    print("Rendering collisionmap...")
    cmap, colormap = render_collisionmap(project)
    levelinfo["terrain-palette"] = colormap

    print("Saving collisionmap:", terrain_filename)
    cmap.save(path.join(target_dir, terrain_filename))

    # Save artwork (mergedimage)
    artwork = project.get_image_data(use_original=True)
    print("Saving artwork:", artwork_filename)
    artwork.save(path.join(target_dir, artwork_filename))

    # Write config
    print("Writing TOML file:", toml_filename)
    with open(path.join(target_dir, toml_filename), 'w') as tf:
        tomlkit.dump(levelinfo, tf)


if __name__ == "__main__":
    if len(sys.argv) != 3:
        print("Usage: ora2parts.py <filename.ora> <target dir>")
    else:
        main(sys.argv[1], sys.argv[2])
