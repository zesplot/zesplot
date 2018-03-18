# What is zesplot?

Zesplot is an attempt at visualising IPv6 addresses and their prefixes. It is
based on squarified treemaps, producing space-filling plots with relative
sizing and colours. 


## Compiling

Zesplot is implemented in Rust so we can leverage `cargo` to build it. Consider
using rustup ( https://rustup.rs/ ) if you are new to Rust. Once you have Rust
up and running, and cloned this repository, use either

	cargo build

or

	cargo build --release

to compile zesplot. The resulting binaries will be respectively
`target/debug/zesplot` and `target/release/zesplot`.


```
USAGE:
    zesplot [FLAGS] [OPTIONS] --addresses <address-file> --prefixes <prefix-file>

FLAGS:
    -d, --draw-hits    Plot addresses on their respective areas
    -f, --filter       Filter out empty prefixes, only plotting prefixes containing addresses from the --addressess
    -h, --help         Prints help information
    -V, --version      Prints version information

OPTIONS:
    -a, --addresses <address-file>    IPv6 addresses to plot on map
    -l, --limit <plot-limit>          Limits number of areas plotted. 0 for unlimited. Default 2000
    -p, --prefixes <prefix-file>      Prefixes to map
```
