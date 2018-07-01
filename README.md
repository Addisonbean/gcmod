# gcmod

`gcmod` is a tool for extracting the contents of and rebuilding GameCube ROMs, as well as for getting all sorts of useful information about a ROM without ever extracting the contents. 

I'm working on adding more features to actually help the modding process go smoother, and eventually I would like it to also be a powerful tool for creating mods for GameCube games. I'd also like to create a Rust API for all of this at some point, but that's a slightly lower priority right now.

## How to use

`gcmod -h` will give you an overview of the available subcommands.

```
disasm     Disassemble the main DOL file from a ROM.
extract    Extract a ROM's contents to disk.
help       Prints this message or the help of the given subcommand(s)
info       Display information about the ROM.
rebuild    Rebuilds a ROM.
```

You can also pass `-h` or `--help` after any of these subcommands to see their usage.

```
$ gcmod info -h
gcmod-info 
Display information about the ROM.

USAGE:
    gcmod info [OPTIONS] <rom_path>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -o, --offset <offset>      Print information about whichever section is at the given offset. 
    -s, --section <section>    Print information about a given section of the ROM.  [possible values: header, dol, fst,
                               apploader]

ARGS:
    <rom_path>
```

Here's a link to a reference to some documentation I'm working on for the GameCube ROM format: [GameCube ROM Info](https://docs.google.com/document/d/1uuLgEZhlXwPBKyDEFGLU_g_7azzA60bv5O3kPxXZmyE/edit?usp=sharing) (I don't update this very often, I need to stay on top of that...)

