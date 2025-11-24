import argparse
import os

import mulder


def main():
    """Entry point for the CLI."""

    parser = argparse.ArgumentParser(
        prog = "python3 -m mulder",
        description = "Command-line utility for Mulder.",
        epilog = "Copyright (C) Université Clermont Auvergne, CNRS/IN2P3, LPCA"
    )
    parser.add_argument("-c", "--cache",
        help = "Mulder default cache location.",
        action = "store_true"
    )
    parser.add_argument("-p", "--prefix",
        help = "Mulder installation prefix.",
        action = "store_true"
    )
    parser.add_argument("-v", "--version",
        help = "Mulder version.",
        action = "store_true"
    )

    args = parser.parse_args()

    result = []
    if args.cache:
        result.append(str(mulder.config.DEFAULT_CACHE))
    if args.prefix:
        result.append(str(mulder.config.PREFIX))
    if args.version:
        result.append(mulder.config.VERSION)

    if result:
        print(" ".join(result))
    else:
        parser.print_help()


if __name__ == "__main__":
    main()
