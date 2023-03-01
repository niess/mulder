from argparse import ArgumentParser
from pathlib import Path
from typing import Optional

import numpy

from . import create_map, generate_physics, git_revision, PREFIX, version


def convert(path: Path, offset: Optional[float]=None):
    """Convert GeoTIFF data to Turtle PNG"""

    from geotiff import GeoTiff # Optional dependency

    assert(path.suffix == ".tif") # Input file must be in GeoTIFF format
    g = GeoTiff(path)

    known_crs = {
        2154: "Lambert 93"
    }
    try:
        projection = known_crs[g.crs_code]
    except KeyError:
        raise ValueError(f"unknown CRS ({g.crs_code})")

    data = numpy.array(g.read())
    if offset is not None:
        data = data + offset

    ny, nx = g.tif_shape
    (xmin, ymin), (xmax, ymax) = g.tif_bBox

    path = path.with_suffix(".png")
    create_map(str(path), projection, (xmin, xmax), (ymin, ymax), data)


def mulder():
    """Entry point for the mulder utility"""

    copyright = "Copyright (C) 2023 Université Clermont Auvergne, "\
                "CNRS/IN2P3, LPC"

    parser = ArgumentParser(
        prog="mulder",
        epilog=copyright,
        description="""Swiss army knife utility for the Mulder library.""")

    subparsers = parser.add_subparsers(title="command",
                                       help="command to execute",
                                       dest="command")

    # Configuration command
    config_parser = subparsers.add_parser(
        name="config",
        epilog=copyright,
        description="Configuration utility for the Mulder library.")

    config_parser.add_argument("-c", "--cflags", action="store_true",
        help="print compiler flags")

    config_parser.add_argument("-g", "--git-revision", action="store_true",
        help="print git revision hash")

    config_parser.add_argument("-l", "--libs", action="store_true",
        help="print linker flags")

    config_parser.add_argument("-p", "--prefix", action="store_true",
        help="print installation prefix")

    config_parser.add_argument("-v", "--version", action="store_true",
        help="print library version")


    # Convert command
    convert_parser = subparsers.add_parser(
        name="convert",
        epilog=copyright,
        description="Convert GeoTIFF data to Turtle PNG")

    convert_parser.add_argument("path",
        help="path to the initial GeoTIFF file")

    convert_parser.add_argument("-o", "--offset",
        help="any altitude offset", type=float)


    # Generate command
    convert_parser = subparsers.add_parser(
        name="generate",
        epilog=copyright,
        description="Generate physics table(s) for Pumas")

    convert_parser.add_argument("path",
        help="path to a Pumas Materials Description File")

    convert_parser.add_argument("-d", "--destination",
        help="destination directory for physics tables")


    # Parse and process arguments
    args = parser.parse_args()

    if args.command == "config":
        flags = []
        if args.cflags:
            flags.append(f"-I{PREFIX}/include")
        if args.git_revision:
            flags.append(git_revision)
        if args.libs:
            flags.append(f"-L{PREFIX}/lib -Wl,-rpath,{PREFIX}/lib -lmulder")
        if args.prefix:
            flags.append(PREFIX)
        if args.version:
            flags.append(version)

        if flags:
            print(" ".join(flags))
        else:
            parser.print_usage()

    elif args.command == "convert":
        convert(Path(args.path), args.offset)

    elif args.command == "generate":
        generate_physics(args.path, args.destination)

    else:
            parser.print_usage()


if __name__ == "__main__":
    mulder()
