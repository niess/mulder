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

    compile = subparsers.add_parser("compile",
        help = "Compile materials tables."
    )
    compile.add_argument("files",
        help = "Materials description file(s).",
        nargs = "*"
    )
    compile.add_argument("-b", "--bremsstrahlung",
        help = "Specify the bremsstralung model.",
        choices = ["ABB94", "KKP95", "SSR19"],
        default = "SSR19"
    )
    compile.add_argument("-n", "--photonuclear",
        help = "Specify the photonuclear model.",
        choices = ["BBKS03", "BM02", "DRSS01"],
        default = "DRSS01"
    )
    compile.add_argument("-p", "--pair-production",
        help = "Specify the pair-production model.",
        choices = ["KKP68", "SSR19"],
        default = "SSR19"
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


    if args.command == "compile":
        mulder.compile(
            *args.files,
            bremsstrahlung = args.bremsstrahlung,
            pair_production = args.pair_production,
            photonuclear = args.photonuclear,
        )

    elif args.command == "config":
        result = []
        if args.cache:
            result.append(str(mulder.config.DEFAULT_CACHE))
        if args.prefix:
            result.append(mulder.config.PREFIX)
        if args.version:
            result.append(mulder.config.VERSION)
        if result:
            print(" ".join(result))

    else:
        parser.print_help()


if __name__ == "__main__":
    main()
