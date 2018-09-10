//#![feature(tool_lints)] // clippy

#[macro_use] extern crate log;
extern crate simplelog;
use simplelog::{SimpleLogger, LevelFilter, Config};

mod treemap;
use treemap::{Area,Row,DataPoint,PlotInfo,specs_to_hier,Specific};

mod plot;

use std::collections::HashSet;
use std::collections::HashMap;

extern crate easy_csv;
#[macro_use] extern crate easy_csv_derive;
extern crate csv;

extern crate hex;
extern crate treebitmap;

mod input;
use input::*;

mod output;


use std::time::{Instant};

use std::process::exit;

extern crate svg;
extern crate ipnetwork;
extern crate rand;
#[macro_use] extern crate clap;

use clap::{Arg, App};


use std::io::prelude::*;
use std::fs::File;
use std::path::Path;

fn main() {

    let matches = App::new("zesplot")
                        .version("0.1")
                        .author("Luuk Hendriks")
                        .arg(Arg::with_name("verbose")
                            .short("v")
                            .multiple(true)
                            .help("Verbose output. Use -vv for debug output")
                        )
                        .arg(Arg::with_name("prefix-file")
                             .short("p")
                             .long("prefixes")
                             .help("Prefixes to map")
                             .takes_value(true)
                             .required(true)
                        )
                        .arg(Arg::with_name("address-file")
                             .short("a")
                             .long("addresses")
                             .help("IPv6 addresses to plot on map")
                             .takes_value(true)
                             .required(true)
                        )
                        .arg(Arg::with_name("filter-empty-prefixes")
                             .short("f")
                             .long("filter")
                             .help("Filter out empty prefixes, only plotting prefixes containing addresses from the --addressess")
                        )
                        // we might want to merge --filter-threshold  with --filter
                        // can takes_value be optional?
                        .arg(Arg::with_name("filter-threshold") 
                             .long("filter-threshold")
                             .aliases(&["ft"])
                             .takes_value(true)
                             .help("Set minimum threshold for --filter. Default 1.")
                        )
                        .arg(Arg::with_name("filter-threshold-asn") 
                             .long("filter-threshold-asn")
                             .aliases(&["fta"])
                             .takes_value(true)
                             .help("Set minimum threshold for --filter for hits per ASN instead of per prefix. Default 1.")
                        )
                        .arg(Arg::with_name("unsized-rectangles")
                             .short("u")
                             .long("unsized")
                             .help("Do not size the rectangles based on prefix length, but size them all equally")
                        )
                        .arg(Arg::with_name("colour-input")
                             .short("c")
                             .long("colour-input")
                             .help("Base the colours on one of the following:
                                \"hits\" (default)
                                \"hw\" (average hamming weight in prefix)
                                \"mss\" (average TCP MSS in prefix)
                                \"ttl\" (average TTL of responses in prefix, only when using ZMAP input)")
                             .takes_value(true)
                             .required(true)
                        )
                        .arg(Arg::with_name("scale-max")
                            .long("--scale-max")
                            .help("Overrule maximum of colour scale, only for -c hits")
                            .takes_value(true)
                        )
                        .arg(Arg::with_name("dp-function")
                             .long("dp-function")
                             .help("Base the colour on a function on the datapoints (for TTL or MSS) within a prefix:
                                \"avg\" mean of the values
                                \"median\" median of the values
                                \"var\" variance of the values
                                \"uniq\" number of unique values"
                            )
                             .takes_value(true)
                        )
                        .arg(Arg::with_name("legend-label")
                            .long("legend-label")
                            .help("Set a custom label for the legend")
                            .takes_value(true)
                        )
                        .arg(Arg::with_name("asn-colours")
                            .long("asn-colours")
                            .help("Additional colours for ASNs. File should contain lines, formatted '$ASN $ID'.
                                Every unique ID will be assigned a colour on the scale.")
                            .takes_value(true)
                        )
                        //.arg(Arg::with_name("draw-hits")
                        //     .short("d")
                        //     .long("draw-hits")
                        //     .help("Plot addresses on their respective areas")
                        //)
                        .arg(Arg::with_name("plot-limit")
                             .short("l")
                             .long("limit")
                             .help(&format!("Limits number of areas plotted. 0 for unlimited. Default {}", plot::PLOT_LIMIT))
                             .takes_value(true)
                        )
                        .arg(Arg::with_name("no-labels")
                             .long("no-labels")
                             .help("Omit the text labels in the final plot")
                        )
                        .arg(Arg::with_name("html-template")
                             .long("html")
                             .help("Create HTML wrapper based on passed template")
                             .takes_value(true)
                        )
                        .arg(Arg::with_name("output-fn")
                             .long("output-fn")
                             .help("Override the generated output filenames. File extensions (.svg, .html) will be appended.")
                             .takes_value(true)
                        )
                        .arg(Arg::with_name("output-dir")
                             .long("output-dir")
                             .help("Specific where to save generated files. Default is current working dir.")
                             .takes_value(true)
                        )
                        .arg(Arg::with_name("create-prefixes")
                             .long("create-prefixes")
                             .help("Create file containing prefixes based on hits from address-file, and exit")
                        )
                        .arg(Arg::with_name("create-addresses")
                             .long("create-addresses")
                             .help("Create file containing addresses based on hits from address-file, and exit")
                        )
                        .get_matches();


    let _ = match matches.occurrences_of("verbose") {
                0   => SimpleLogger::init(LevelFilter::Warn, Config::default()),
                1   => SimpleLogger::init(LevelFilter::Info, Config::default()),
                2|_ => SimpleLogger::init(LevelFilter::Debug, Config::default()),
    };

    info!("-- reading input files");

    let mut datapoints: Vec<DataPoint> = Vec::new();
    let now = Instant::now();
    //let datapoints = read_datapoints_from_file(matches.value_of("address-file").unwrap(),
    //                                            matches.value_of("colour-input").unwrap()).unwrap();
    //let datapoints;
    match read_datapoints_from_file(matches.value_of("address-file").unwrap(),
                                    matches.value_of("colour-input").unwrap()) {
        Ok(dps) => datapoints = dps,
        Err(e) => error!("Can not read datapoints from address-file: {}", e),
    };
                      

    info!("file read: {}.{:.2}s", now.elapsed().as_secs(), now.elapsed().subsec_millis());

    let mut table = prefixes_from_file(matches.value_of("prefix-file").unwrap()).unwrap();

    info!("prefixes: {} , addresses: {}", table.iter().count(), datapoints.len());
    let mut prefix_mismatches = 0;
    let mut asn_to_hits: HashMap<String, usize> = HashMap::new();
    for dp in datapoints.into_iter() {
        if let Some((_, _, s)) = table.longest_match_mut(dp.ip6) {
            s.push_dp(dp);
            let asn_hitcount = asn_to_hits.entry(s.asn.clone()).or_insert(0);
            *asn_hitcount += 1;
        } else {
            prefix_mismatches += 1;
        }
    }

    let unique_asns: HashSet<String> = asn_to_hits.keys().cloned().collect();
    
    if prefix_mismatches > 0 {
        warn!("Could not match {} addresses", prefix_mismatches);
    }

    // this is also used later on when creating svg/html files
    let output_dir = matches.value_of("output-dir").unwrap_or_else(|| "./");

    if matches.is_present("create-addresses") {
        let address_output_fn = format!("{}/{}.addresses",
                    output_dir,
                    Path::new(matches.value_of("address-file").unwrap()).file_name().unwrap().to_str().unwrap(),
        );
        info!("creating address file {}", address_output_fn);
        let mut file = File::create(address_output_fn).unwrap();
        for (_,_,s) in table.iter() {
            for dp in s.datapoints.iter() {
                let _ = writeln!(file, "{}", dp.ip6);
            }
        }
        exit(0);
    }


    // read extra ASN colour info, if any
    let asn_colours: &mut HashMap<u32, String> = &mut HashMap::new();
    if matches.is_present("asn-colours") {
        *asn_colours = asn_colours_from_file(matches.value_of("asn-colours").unwrap()).unwrap();
    }

    let mut plot_info = PlotInfo::new(asn_colours);
    plot_info.set_maxes(&table, &matches);
    let unsized_rectangles = matches.is_present("unsized-rectangles");

    let mut specifics: Vec<Specific>  = table.into_iter().map(|(_,_,s)| s).collect();
    let mut specifics_with_hits = 0;
    for s in &specifics {
        if s.hits() > 0 {
            specifics_with_hits += 1;
        }
    }

    info!("# of specifics: {}", specifics.len());
    info!("# of specifics with hits: {}", specifics_with_hits);
    info!("# of ASNs with hits: {}", unique_asns.len());
    info!("# of hits in all specifics: {}", specifics.iter().fold(0, |sum, s| sum + s.all_hits())  );

    if matches.is_present("filter-threshold-asn") {
        let minimum = value_t!(matches.value_of("filter-threshold-asn"), usize).unwrap_or_else(|_| 0);
        warn!("got --filter-threshold-asns, only plotting ASNs with minimum hits of {}", minimum);
        let pre_filter_len_specs = specifics.len();
        specifics.retain(|s| *asn_to_hits.get(&s.asn).unwrap_or(&0) >= minimum);
        warn!("filtered {} specifics, left: {}", pre_filter_len_specs - specifics.len(), specifics.len());
    }

    specifics = specs_to_hier(&specifics);
    info!("# of top-level specifics: {}", specifics.len());

    //let mut specifics: Vec<Specific>  = specs_to_hier(&table.into_iter().map(|(_,_,s)| s).collect());
    // without hierarchy: //TODO make this a switch
    //let mut specifics: Vec<Specific>  = (table.into_iter().map(|(_,_,s)| s).collect());

    // we calculate the total_area after turning the specifics into an hierarchical model
    // because the hierchical model will have less 'first level' rectangles, thus a smaller total_area
    let mut total_area = specifics.iter().fold(0, |sum, s|{sum + s.size(unsized_rectangles)});



    if matches.is_present("filter-empty-prefixes") {
        //TODO: currently, we plot everything that either contains hits, or has more-specifics that contain hits
        // if a prefix has multiple more-specifics, and only one has hits, all specifics are plotted
        // filtering out empty more-specifics might be useful
        let pre_filter_len_specs = specifics.len();
        //specifics.retain(|s| s.all_hits() >= 1);
        let filter_threshold = value_t!(matches.value_of("filter-threshold"), usize).unwrap_or_else(|_| 1);
        info!("filter_threshold: {}", filter_threshold);
        specifics.retain(|s| s.all_hits() >= filter_threshold);
        total_area = specifics.iter().fold(0, |sum, s|{sum + s.size(unsized_rectangles)});
        info!("filtered {} empty specifics, left: {}", pre_filter_len_specs - specifics.len(), specifics.len());

    } else {
        info!("no filtering of empty prefixes");
    }

    // this is affected by how we impement the filtering of empty prefixes
    // do we want to keep empty more-specifics of parents with hits?
    // idea: be lenient in create-prefixes, so we have the option to be more restrictive in the filtering
    if matches.is_present("create-prefixes") {
        specifics.retain(|s| s.all_hits() > 0);
        let prefix_output_fn = format!("{}/{}.prefixes",
                    output_dir,
                    Path::new(matches.value_of("address-file").unwrap()).file_name().unwrap().to_str().unwrap(),
        );
        println!("creating prefix file {}", prefix_output_fn);
        let mut file = File::create(prefix_output_fn).unwrap();
        for s in specifics {
            let _ = writeln!(file, "{} {}", s.network, s.asn);
        }
        exit(0);
    }

    // initial aspect ratio FIXME this doesn't affect anything, remove
    let init_ar: f64 = 1_f64 / (4.0/1.0);

    let norm_factor = (plot::WIDTH * plot::HEIGHT) / total_area as f64;

    let mut areas: Vec<Area> = Vec::new();

    // sort by both size and ASN, so ASs are grouped in the final plot
    specifics.sort_by(|a, b| b.prefix_len().cmp(&a.prefix_len()).reverse().then(a.asn.cmp(&b.asn))  );

    for s in specifics {
        areas.push(Area::new(s.size(unsized_rectangles) as f64 * norm_factor, init_ar, s  ));
    }


    let mut rows = Vec::new();
    //let (first_area, remaining_areas) = areas.split_first().unwrap();
    let remaining_areas = areas.split_off(1);   // FIXME crashes when there is only a single prefix.
                                                // Maybe use if let Some() =  split_first()?
    let first_area = areas.pop().unwrap();
    let (mut new_row_x, mut new_row_y) = (0.0, 0.0);
    rows.push(Row::new(new_row_x, new_row_y, true, first_area));

    for a in remaining_areas {

        // if try() returns an Area, it means the row/column was 'full'
        if let Some(area) = rows.last_mut().unwrap().try(a) {

            let cur_row_w = rows.last().unwrap().w ;
            let cur_row_h = rows.last().unwrap().h;
            let cur_row_vertical = rows.last().unwrap().vertical;
            if cur_row_vertical {
                // create new horizontal row
                new_row_x += cur_row_w;
                rows.push(Row::new(new_row_x, new_row_y, false, area));
            } else {
                // create new vertical row
                new_row_y += cur_row_h;
                rows.push(Row::new(new_row_x, new_row_y, true, area));
            }
            rows.last_mut().unwrap().reflow();
        }
    }


    info!("-- drawing svg");
    let document = plot::draw_svg(&matches, rows, &plot_info);

    info!("-- creating output files");
    match output::create_svg(&matches, &document, output_dir) {
        Ok(f) => info!("created {}", f),
        Err(e) => error!("error while creating svg file: {}", e),
    }

    if matches.is_present("html-template") {
        match output::create_html(&matches, &document, output_dir) {
            Ok(f) => info!("created {}", f),
            Err(e) => error!("error while creating HTML file: {}", e),
        }
    }

}


