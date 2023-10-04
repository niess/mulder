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
    subparsers = parser.add_subparsers(
        title = "command",
        help = "Command to execute",
        dest = "command"
    )

    compute = subparsers.add_parser("compute",
        help = "Compute materials tables."
    )
    compute.add_argument("files",
        help = "Materials description file(s).",
        nargs = "*"
    )
    compute.add_argument("-b", "--bremsstrahlung",
        help = "Specify the bremsstralung model.",
        choices = ["ABB", "KKP", "SSR"],
        default = "SSR"
    )
    compute.add_argument("-n", "--photonuclear",
        help = "Specify the photonuclear model.",
        choices = ["BBKS", "BM", "DRSS"],
        default = "DRSS"
    )
    compute.add_argument("-p", "--pair-production",
        help = "Specify the pair-production model.",
        choices = ["KKP", "SSR"],
        default = "SSR"
    )

    config = subparsers.add_parser("config",
        help = "Print configuration data."
    )
    config.add_argument("-c", "--cache",
        help = "Mulder default cache location.",
        action = "store_true"
    )
    config.add_argument("-p", "--prefix",
        help = "Mulder installation prefix.",
        action = "store_true"
    )
    config.add_argument("-v", "--version",
        help = "Mulder version.",
        action = "store_true"
    )

    args = parser.parse_args()


    if args.command == "compute":
        mulder.compute(
            *args.files,
            bremsstrahlung = args.bremsstrahlung,
            pair_production = args.pair_production,
            photonuclear = args.photonuclear,
        )

    elif args.command == "config":
        result = []
        if args.cache:
            result.append(mulder.DEFAULT_CACHE)
        if args.prefix:
            result.append(os.path.dirname(__file__))
        if args.version:
            result.append(mulder.VERSION)
        if result:
            print(" ".join(result))

    else:
        parser.print_help()


if __name__ == "__main__":
    main()
