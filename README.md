# What is zesplot?

Zesplot is an attempt at visualising IPv6 addresses and their prefixes. It is
based on squarified treemaps, producing space-filling plots with relative
sizing. Colouring can be based on the number of addresses in a prefix, or, when
used in combination with zmap output, metrics like the median TTL observed in a
prefix.

The idea for zesplot was born after attending the
[RIPE IPv6 Hackathon](https://labs.ripe.net/Members/becha/results-hackathon-version-6) 
and a first version was presented in [MAPRG](https://datatracker.ietf.org/meeting/101/materials/slides-101-maprg-zesplot-an-attempt-to-visualise-ipv6-address-space-00) at IETF101 in London.

Most of the current features were implemented for our IMC'18 paper titled
['Clusters in the Expanse: Understanding and Unbiasing IPv6 Hitlists'](https://ipv6hitlist.github.io).



## Compiling

Zesplot is implemented in Rust so we can leverage `cargo` to build it. Consider
using rustup ( https://rustup.rs/ ) if you are new to Rust. ~~We need to use
nightly in order to leverage the 128 bit integer features not yet in the stable
channel. Using rustup, run `rustup override set nightly` from within the cloned
repository. Check using `rustc -V` whether you are now indeed using nightly.~~
As the 128 bit integer support is now stabilized, we do not need rust nightly
anymore.

Once you have Rust up and running, and cloned this repository, use either

	cargo build

or

	cargo build --release

to compile zesplot. The resulting binaries will be respectively
`target/debug/zesplot` and `target/release/zesplot`.

## Using zesplot

**NB** zesplot is currently being refactored. The information here aims at 
being compatible with HEAD on the master branch, though it might be slightly
outdated. 

### mandatory inputs: --prefixes and --addresses

Zesplot always expects `--prefixes` and `--addresses`, or their shorter aliases
`-p` and `-a`. 

#### Minimal example

Using the (possibly outdated) `ipv6_prefixes.txt` in this repo, and a list of
addresses you like to plot (e.g. addresses from a webserver access log), we can
create a zesplot:

```bash
zesplot --prefixes ipv6_prefixes.txt --addresses my_addresses.txt
```

The prefixes are coloured based on the number of 'address hits' in that prefix.

### Filtering prefixes

Maybe you only want to plot prefixes for which addresses exist in the address list:
```bash
zesplot --prefixes ipv6_prefixes.txt --addresses my_addresses.txt --filter
```

Or, only plot prefixes that have at least 10 hits on the address list:
```bash
zesplot --prefixes ipv6_prefixes.txt --addresses my_addresses.txt --filter-threshold 10
```

### Specifying the output directory and filenames

Zesplot will generate a filename based on some of the input parameters. This
filename can be overriden via `--output-fn`, which comes in handy when trying
out different plots and just want to F5 your browser to see the new plot:

```bash
zesplot --prefixes ipv6_prefixes.txt --addresses my_addresses.txt --output-dir /tmp/ --output-fn my_zesplot.svg
```

### Metadata via CSV input

If we pass `--csv addr`, zesplot will parse the file passed via `--addresses`
as being CSV, expecting a column `addr` to contain the addresses. If we pass
two column names, the second column name will be used as metadata on which we
can apply statistical functions. Consider a CSV file with a column `ttl`
containing the TTL value (or in proper v6 terminology, Hop Limit) for every
address, e.g. a zmap output file:

```bash
zesplot --prefixes ipv6_prefixes.txt --addresses input.csv --csv addr,ttl --dp-function median
```

For every prefix, the median TTL is calculated, and the resulting plot is
coloured based on these median values. Other `--dp-function` options are
`mean`, `var`, `uniq` and `sum`.


### More in --help

The current `--help` output (also shown at the end of this README) shows some
additional features. Note that some of the options might be renamed in the
refactoring effort. Some options might break as well during the refactoring
effort on our way to v0.2.0.



## Example output

For both static and interactive examples, check out [this
page](https://ipv6hitlist.github.io/zesplot/) related to our IMC paper.

As a sneak preview, and to brighten up this page:

![zesplot example output](doc/example_output.png)


More examples showcasing the various features of zesplot will be described
soon, though the page linked above gives a good impression of all the
possibilities.

```
$ zesmap -h

zesplot 0.1.0
Luuk Hendriks

USAGE:
    zesplot [FLAGS] [OPTIONS] --addresses <address-file> --prefixes <prefix-file>

FLAGS:
        --create-addresses    Create file containing addresses based on hits from address-file, and exit
        --create-prefixes     Create file containing prefixes based on hits from address-file, and exit
    -f, --filter              Filter out empty prefixes, only plotting prefixes containing addresses from the
                              --addresses. Equal to --filter-threshold 1
    -h, --help                Prints help information
        --no-labels           Omit the text labels in the final plot
    -u, --unsized             Do not size the rectangles based on prefix length, but size them all equally
    -V, --version             Prints version information
    -v                        Verbose output. Use -vv for debug output

OPTIONS:
    -a, --addresses <address-file>                       IPv6 addresses to plot on map
        --asn-colours <asn-colours>
            Additional colours for ASNs. File should contain lines, formatted '$ASN $ID'.
                                            Every unique ID will be assigned a separate colour.
        --csv <csv-columns>
            When passing csv input in --addresses, use --csv $addr[,$dp] to denote which columns to use for addresses
            and datapoints, e.g. TTL or MSS
        --dp-function <dp-function>
            Base the colour on a function on the datapoints (passed via the second column in --csv  within a prefix:
                                            "avg" mean of the values
                                            "median" median of the values
                                            "var" variance of the values
                                            "uniq" number of unique values
                                            "sum" sum of values
        --filter-threshold <filter-threshold>            Set minimum threshold for --filter. Default 1.
        --filter-threshold-asn <filter-threshold-asn>
            Set minimum threshold for --filter for hits per ASN instead of per prefix. Default 1.

        --html <html-template>                           Create HTML wrapper based on passed template
        --legend-label <legend-label>                    Set a custom label for the legend
        --output-dir <output-dir>
            Specific where to save generated files. Default is current working dir.

        --output-fn <output-fn>
            Override the generated output filenames. File extensions (.svg, .html) will be appended.

    -l, --limit <plot-limit>                             Limits number of areas plotted. 0 for unlimited. Default 2000
    -p, --prefixes <prefix-file>                         Prefixes to map
        --scale-max <scale-max>                          Overrule maximum of colour scale, only for -c hits

```
