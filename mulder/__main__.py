from argparse import ArgumentParser

from . import git_revision, PREFIX, version


def config():
    """Entry point for mulder-config"""

    parser = ArgumentParser(
        prog="mulder-config",
        epilog="Copyright (C) 2023 Université Clermont Auvergne, CNRS/IN2P3, LPC",
        description="""Configuration utility for the Mulder C-library.""")

    parser.add_argument("-c", "--cflags", help="print compiler flags",
                        action="store_true")

    parser.add_argument("-g", "--git-revision", help="print git revision hash",
                        action="store_true")

    parser.add_argument("-l", "--libs", help="print linker flags",
                        action="store_true")

    parser.add_argument("-p", "--prefix", help="print installation prefix",
                        action="store_true")

    parser.add_argument("-v", "--version", help="print library version",
                        action="store_true")


    args = parser.parse_args()

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
        parser.exit()


if __name__ == "__main__":
    config()
