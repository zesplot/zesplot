mod treemap;
use treemap::{Area,Row,DataPoint,PlotInfo,specs_to_hier,Specific,ColourMode};

mod plot;

use std::collections::HashSet;
use std::collections::HashMap;

extern crate colored;
use colored::*;

extern crate easy_csv;
#[macro_use]
extern crate easy_csv_derive;
extern crate csv;

extern crate hex;

use easy_csv::{CSVIterator};


use std::time::{Instant};

extern crate treebitmap;
use treebitmap::{IpLookupTable};
use std::io;

use std::process::exit;

extern crate svg;
extern crate ipnetwork;
extern crate rand;
#[macro_use] extern crate clap;

use clap::{Arg, App};

use svg::*;
use svg::node::Text as Tekst;
use svg::node::element::{Rectangle, Text, Group};

use ipnetwork::Ipv6Network;
use std::net::Ipv6Addr;

use std::io::{BufReader};
use std::io::prelude::*;
use std::fs::File;
use std::path::Path;

// the input for prefixes_from_file is generated a la:
// ./bgpdump -M latest-bview.gz | ack "::/" cut -d'|' -f 6,7 --output-delimiter=" " | awk '{print $1,$NF}' |sort -u
// now, this still includes 6to4 2002::/16 announcements
// should we filter these out?
// IDEA: limit those prefixes to say a /32 in size? and label them e.g. 6to4 instead of ASxxxx

// bgpstream variant:
// bgpreader -c route-views6 -w 1522920000,1522928386 -k 2000::/3 > /tmp/bgpreader.test.today 
// cut -d'|' -f8,11 /tmp/bgpreader.test.today | sort -u > bgpreader.test.today.sorted

// or, simply fetched from http://data.caida.org/datasets/routing/routeviews6-prefix2as/2018/01/
// awk '{print $1"/"$2, $3}'

fn prefixes_from_file<'a>(f: &'a str) -> io::Result<IpLookupTable<Ipv6Addr,Specific>> {
    let mut file = File::open(f)?;
    let mut s = String::new();
    file.read_to_string(&mut s)?;
    let mut table: IpLookupTable<Ipv6Addr,Specific> = IpLookupTable::new();
    for line in s.lines() {
        let parts = line.split_whitespace().collect::<Vec<&str>>();
        if let Ok(route) = parts[0].parse::<Ipv6Network>(){

            // asn is not a u32, as some routes have an asn_asn_asn,asn notation in pfx2as
            let asn = parts[1];
                table.insert(route.ip(), route.prefix().into(),
                        Specific { network: route, asn: asn.to_string(), datapoints: Vec::new(), specifics: Vec::new()});
        } else {
                eprintln!("choked on {} while reading prefixes file", line);
        }
    }; 
    Ok(table)
}

fn asn_colours_from_file<'a>(f: &'a str) -> io::Result<HashMap<u32, String>> {
    let mut mapping: HashMap<u32, String> = HashMap::new();
    let mut file = File::open(f)?;
    let mut s = String::new();
    file.read_to_string(&mut s)?;
    for line in s.lines() {
        let parts = line.split_whitespace().collect::<Vec<&str>>();
        let asn = parts[0].parse::<u32>().unwrap();
        let id = parts[1];
        mapping.insert(asn, id.to_string());
    }

    Ok(mapping)
}


#[derive(Debug,CSVParsable)] //Deserialize
struct ZmapRecord {
    saddr: String,
    ttl: u8,
}

#[derive(Debug,CSVParsable)] //Deserialize
struct ZmapRecordTcpmss {
    saddr: String,
    tcpmss: u16
}
#[derive(Debug,CSVParsable)] //Deserialize
struct ZmapRecordDns {
    saddr: String,
    data: String
}

fn main() {

    let matches = App::new("zesmap")
                        .version("0.1")
                        .author("drk")
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
                             .help("Base the colours on any of the following:
                                \"hits\" (default)
                                \"hw\" (average hamming weight in prefix)
                                \"mss\" (average TCP MSS in prefix)
                                \"ttl\" (average TTL of responses in prefix, only when using ZMAP input)")
                             .takes_value(true)
                             .required(true)
                        )
                        .arg(Arg::with_name("scale-max")
                            .long("--scale-max")
                            .help("[TEMP/DEV] Overrule maximum of colour scale, only for -c hits")
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
                            .help("Additional colours for ASNs. File should contain lines with 'ASN ID'.
                                Every unique ID will be assigned a colour on the scale.")
                            .takes_value(true)
                        )
                        .arg(Arg::with_name("draw-hits")
                             .short("d")
                             .long("draw-hits")
                             .help("Plot addresses on their respective areas")
                        )
                        .arg(Arg::with_name("plot-limit")
                             .short("l")
                             .long("limit")
                             .help(&format!("Limits number of areas plotted. 0 for unlimited. Default {}", plot::PLOT_LIMIT))
                             .takes_value(true)
                        )
                        .arg(Arg::with_name("no-labels")
                             .long("no-labels")
                             .help(&format!("Omit the text labels in the final plot"))
                        )
                        .arg(Arg::with_name("create-html")
                             .long("html")
                             .help(&format!("Create HTML wrapper output in ./html"))
                        )
                        .arg(Arg::with_name("output-fn")
                             .long("output-fn")
                             .help(&format!("Override the generated output filenames. File extensions (.svg, .html) will be appended."))
                             .takes_value(true)
                        )
                        .arg(Arg::with_name("create-prefixes")
                             .long("create-prefixes")
                             .help(&format!("Create file containing prefixes based on hits from address-file, and exit"))
                        )
                        .arg(Arg::with_name("create-addresses")
                             .long("create-addresses")
                             .help(&format!("Create file containing addresses based on hits from address-file, and exit"))
                        )
                        .get_matches();

    eprintln!("-- reading input files");

    let mut datapoints: Vec<DataPoint> = Vec::new();
    let now = Instant::now();
    if matches.value_of("address-file").unwrap().contains(".csv") {
        // expect ZMAP output as input
        
        let mut rdr = csv::Reader::from_file(matches.value_of("address-file").unwrap()).unwrap();
        match matches.value_of("colour-input").unwrap() {
            "mss" => {
                let iter = CSVIterator::<ZmapRecordTcpmss,_>::new(&mut rdr).unwrap();
                for zmap_record in iter {
                    let z = zmap_record.unwrap();
                    datapoints.push(
                        DataPoint { 
                            ip6: z.saddr.parse().unwrap(),
                            meta: z.tcpmss.into()
                        }
                    );
                }
            }
            "dns" => {
                let iter = CSVIterator::<ZmapRecordDns,_>::new(&mut rdr).unwrap();
                for zmap_record in iter {
                    let z = zmap_record.unwrap();
                    datapoints.push(
                        DataPoint { 
                            ip6: z.saddr.parse().unwrap(),
                            //first bit in byte 4 is RA bit
                            meta: ((hex::decode(z.data).unwrap()[3] & 0b1000_0000) >> 7) as u32,
                            //  let bytes = u32::from_str_radix("41973333", 16).unwrap();
                        }
                    );
                }
            }
            // TODO: do we want to default to TTL? can be confusing maybe
            // we need to take the tooltip in the html into consideration
            // and perhaps only show dp-avg/var/uniq when an explicit dp is passed (ie -c mss or -c ttl)
            "ttl"|_ => {
                let iter = CSVIterator::<ZmapRecord,_>::new(&mut rdr).unwrap();
                for zmap_record in iter {
                    let z = zmap_record.unwrap();
                    datapoints.push(
                        DataPoint { 
                            ip6: z.saddr.parse().unwrap(),
                            meta: z.ttl.into()
                        }
                    );
                    datapoints.last_mut().unwrap().ttl_to_path_length();
                }
            }
        }
    } else {
        // expect a simple list of IPv6 addresses separated by newlines
        for line in BufReader::new(
                File::open(matches.value_of("address-file").unwrap()).unwrap()
            ).lines(){
                let line = line.unwrap();
                datapoints.push(DataPoint { ip6: line.parse().unwrap(), meta: 0 });
            }
    }

    eprintln!("[TIME] file read: {}.{:.2}s", now.elapsed().as_secs(),  now.elapsed().subsec_nanos() / 1_000_000);

    let mut table = prefixes_from_file(matches.value_of("prefix-file").unwrap()).unwrap();

    eprintln!("prefixes: {} , addresses: {}", table.iter().count(), datapoints.len());
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
        let s = format!("Could not match {} addresses", prefix_mismatches).to_string().on_red().bold();
        eprintln!("{}", s);
    }

    if matches.is_present("create-addresses") {
        let address_output_fn = format!("output/{}.addresses",
                    Path::new(matches.value_of("address-file").unwrap()).file_name().unwrap().to_str().unwrap(),
        );
        eprintln!("creating address file {}", address_output_fn);
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


    // maximum values to determine colour scale later on, passed via PlotInfo
    // maximum number of hits in certain prefix
    let mut max_hits = 0;
    // based on DataPoint.meta, e.g. TTL, MSS:
    let mut max_dp_avg = 0f64; 
    let mut max_dp_median = 0f64; 
    let mut max_dp_var = 0f64;
    let mut max_dp_uniq = 0_usize;
    let mut max_dp_sum = 0_usize;
    // maximum hamming weight: // TODO do we need var/median etc?
    let mut max_hw_avg = 0f64;
    let unsized_rectangles = matches.is_present("unsized-rectangles");
    
    for (_,_,s) in table.iter() {
        if s.datapoints.len() > max_hits {
            max_hits = s.datapoints.len();
        }
        // based on dp.meta:
        if s.dp_avg() > max_dp_avg {
            max_dp_avg = s.dp_avg();
        }
        if s.dp_median() > max_dp_median {
            max_dp_median = s.dp_median();
        }
        if s.dp_var() > max_dp_var {
            max_dp_var = s.dp_var();
        }
        if s.dp_uniq() > max_dp_uniq {
            max_dp_uniq = s.dp_uniq();
        }
        if s.dp_sum() > max_dp_sum {
            max_dp_sum = s.dp_sum();
        }
        // hamming weight:
        if s.hw_avg() > max_hw_avg {
            max_hw_avg = s.hw_avg();
        }
    }

    eprintln!("maximums (for --scale-max):");
    eprintln!("max_hits: {}", max_hits);
    if matches.is_present("scale-max") {
        eprintln!("overruling max_hits, was {}, now is {}", max_hits, matches.value_of("scale-max").unwrap());
        max_hits = matches.value_of("scale-max").unwrap().parse::<usize>().unwrap();
    }

    let mut specifics: Vec<Specific>  = table.into_iter().map(|(_,_,s)| s).collect();
    let mut specifics_with_hits = 0;
    for s in &specifics {
        if s.hits() > 0 {
            specifics_with_hits += 1;
        }
    }

    eprintln!("# of specifics: {}", specifics.len());
    eprintln!("# of specifics with hits: {}", specifics_with_hits);
    eprintln!("# of ASNs with hits: {}", unique_asns.len());
    eprintln!("# of hits in all specifics: {}", specifics.iter().fold(0, |sum, s| sum + s.all_hits())  );

    if matches.is_present("filter-threshold-asn") {
        let minimum = value_t!(matches.value_of("filter-threshold-asn"), usize).unwrap_or_else(|_| 0);
        eprintln!("got --filter-threshold-asns, only plotting ASNs with minimum hits of {}", minimum);
        let pre_filter_len_specs = specifics.len();
        specifics.retain(|s| asn_to_hits.get(&s.asn).unwrap_or(&0) >= &minimum);
        eprintln!("filtered {} specifics, left: {}", pre_filter_len_specs - specifics.len(), specifics.len());
    }

    specifics = specs_to_hier(&specifics);
    eprintln!("# of top-level specifics: {}", specifics.len());

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
        eprintln!("filter_threshold: {}", filter_threshold);
        specifics.retain(|s| s.all_hits() >= filter_threshold);
        total_area = specifics.iter().fold(0, |sum, s|{sum + s.size(unsized_rectangles)});
        eprintln!("filtered {} empty specifics, left: {}", pre_filter_len_specs - specifics.len(), specifics.len());

    } else {
        eprintln!("no filtering of empty prefixes");
    }

    // TODO: this is affected by how we impement the filtering of empty prefixes
    // do we want to keep empty more-specifics of parents with hits?
    // idea: be lenient in create-prefixes, so we have the option to be more restrictive in the filtering
    if matches.is_present("create-prefixes") {
        //routes.retain(|r| r.datapoints.len() > 0);
        specifics.retain(|s| s.all_hits() > 0);
        let prefix_output_fn = format!("output/{}.prefixes",
                    Path::new(matches.value_of("address-file").unwrap()).file_name().unwrap().to_str().unwrap(),
        );
        eprintln!("creating prefix file {}", prefix_output_fn);
        let mut file = File::create(prefix_output_fn).unwrap();
        for s in specifics {
            let _ = writeln!(file, "{} {}", s.network, s.asn);
        }
        exit(0);
    }

    /*
    // top 10 prefixes
    eprintln!("top 10 prefixes with most hits");
    routes.sort_by(|a, b| a.datapoints.len().cmp(&b.datapoints.len()).reverse());
    for r in routes.iter().take(10) {
        println!("{} {} : {}", r.asn, r.prefix, r.datapoints.len())
    }
    eprintln!("----");
    
    // bottom 10 smallest prefix lenghts
    eprintln!("bottom 10 prefixes with smallest prefix lenghts");
    routes.sort_by(|a, b| a.prefix_len().cmp(&b.prefix_len()).reverse());
    for r in routes.iter().take(10) {
        println!("{} {} : {}", r.asn, r.prefix, r.datapoints.len())
    }
    eprintln!("----");
    */

    // initial aspect ratio FIXME this doesn't affect anything, remove
    let init_ar: f64 = 1_f64 / (4.0/1.0);

    let norm_factor = (plot::WIDTH * plot::HEIGHT) / total_area as f64;

    let mut areas: Vec<Area> = Vec::new();

    // sort by both size and ASN, so ASs are grouped in the final plot
    specifics.sort_by(|a, b| b.prefix_len().cmp(&a.prefix_len()).reverse().then(a.asn.cmp(&b.asn))  );

    for s in specifics {
        areas.push(Area::new(s.size(unsized_rectangles) as f64 * norm_factor, init_ar, s  ));
    }


    let mut colour_mode = ColourMode::Hits;
    
    let dp_desc = if matches.is_present("legend-label") {
        matches.value_of("legend-label").unwrap().to_string()
    } else {
        match matches.value_of("colour-input").unwrap_or(plot::COLOUR_INPUT) {
            "ttl"   => "TTL".to_string(),
            "mss"   => "TCP MSS".to_string(),
            "dns"   => "DNS RA bit".to_string(),
            "hits"|_ => "Hits".to_string()
        }
    };

    if matches.is_present("dp-function") {
        colour_mode = match matches.value_of("dp-function").unwrap() {
            "avg" => ColourMode::DpAvg,
            "median" => ColourMode::DpMedian,
            "var" => ColourMode::DpVar,
            "uniq" => ColourMode::DpUniq,
            "sum" => ColourMode::DpSum,
            _   =>  colour_mode
        };
    } else if matches.is_present("asn-colours") {
        colour_mode = ColourMode::Asn;
    } else if dp_desc == "TTL" || dp_desc == "TCP MSS" { //ugly..
        eprintln!("in else");
        colour_mode = ColourMode::DpAvg;
    }

    let plot_info = PlotInfo{max_hits, max_dp_avg, max_dp_median, max_dp_var, max_dp_uniq, max_dp_sum, colour_mode, dp_desc, asn_colours};

    let mut rows = Vec::new();
    //let (first_area, remaining_areas) = areas.split_first().unwrap();
    let remaining_areas = areas.split_off(1);   // FIXME crashes when there is only a single prefix.
                                                // Maybe use if let Some() =  split_first()?
    let first_area = areas.pop().unwrap();
    let (mut new_row_x, mut new_row_y) = (0.0, 0.0);
    rows.push(Row::new(new_row_x, new_row_y, true, first_area));
    let mut i = 0;

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

        i = i + 1;
    }


    //eprintln!("-- creating svg");

    let mut groups: Vec<Group> = Vec::new();
    let mut areas_plotted: u64 = 0;

    let plot_limit = value_t!(matches, "plot-limit", u64).unwrap_or(plot::PLOT_LIMIT);
    for row in rows {
        
        if plot_limit > 0 && areas_plotted >= plot_limit {
            break;
        }

        for area in row.areas {
            let mut group = Group::new()
                //.set("data-something", area.specific.asn.to_string())
                ;

            let sub_rects = area.specific.all_rects(&area, &plot_info);
            for sub_rect in sub_rects {
                group.append(sub_rect);
            }

            // drawing hits is future work after we've successfully got the hierarchical stuff working
            /*
            if matches.is_present("draw-hits") {
                let mut rng = thread_rng();
                let sample = sample(&mut rng, &area.route.datapoints, 1000); //TODO make variable
                //println!("took {} as sample from {}", sample.len(), area.route.datapoints.len());
                let mut g_hits = Group::new(); 
                let first_ip = u128::from(area.route.prefix.iter().next().unwrap());
                let mut u = area.surface / (area.route.prefix.size()) as f64; 
                //FIXME location is still incorrect

                //u = u  / (WIDTH );
                //println!("u: {}", u);

                
                //for h in area.route.hits.iter() { 
                for h in sample {
                    let l = u128::from(h.ip6) - first_ip;
                    //println!("l: {}", Ipv6Addr::from(l));
                    let y = (l as f64 * u) / area.w;
                    let x = (l as f64 * u) % area.w;
                    //println!("x  = {}  % {} == {}", l as f64 * u, area.w, x);
                    //println!("plotting {} at {} , {}", h, x, y);

                    /*
                    g_hits.append(Rectangle::new()
                                  .set("x", area.x + x)
                                  .set("y", area.y + y)
                                  .set("width", 0.001)
                                  .set("height", 0.001)
                                  .set("stroke", "yellow")
                                  .set("stroke-width", 0.1)
                                  );
                    */
                    g_hits.append(Circle::new()
                                    .set("cx", area.x + x)
                                    .set("cy", area.y + y)
                                    .set("r", 0.1)
                                    .set("opacity", 0.1)
                                    .set("fill", "yellow")
                                    );
                }
                group.append(g_hits); 
            }
            */


            if !matches.is_present("no-labels") {
                if area.w > 0.5 {
                    let mut label = Text::new()
                        .set("class", "label")
                        .set("x", area.x + area.w/2.0)
                        .set("y", area.y + area.h/2.0)
                        .set("font-family", "mono")
                        .set("font-size", format!("{}%", area.w.min(area.h))) // == f64::min
                        .set("text-anchor", "middle");
                        label.append(Tekst::new(area.specific.to_string()))
                        ;
                    group.append(label);
                }
            }
            groups.push(group);



            areas_plotted += 1;
        }
    }

    let (defs, legend_g) = if matches.is_present("asn-colours") {
        plot::legend_discrete(&plot_info)
    } else {
        plot::legend(&plot_info)
    };

    eprintln!("plotting {} rectangles, limit was {}", areas_plotted, plot_limit);

    let mut document = Document::new()
                        .set("viewBox", (0, 0, plot::WIDTH + plot::LEGEND_MARGIN_W as f64, plot::HEIGHT))
                        .set("id", "treeplot")
                        ;
    for g in groups {
        document.append(g);
    }
    document.append(defs);
    document.append(legend_g);


    //eprintln!("-- creating output files");

    let output_fn_sized = if matches.is_present("unsized-rectangles") {
        "unsized"
    } else {
        "sized"
    };
    let output_fn_filtered = if matches.is_present("filter-empty-prefixes") {
        format!("filtered.ft{}", matches.value_of("filter-threshold").unwrap_or("1"))
    } else {
        "unfiltered".to_string()
    };
    let output_fn = if matches.is_present("output-fn") {
        matches.value_of("output-fn").unwrap().to_string()
    } else {
        format!("{}.{}.{}.{}", Path::new(matches.value_of("address-file").unwrap()).file_name().unwrap().to_str().unwrap(),
            matches.value_of("colour-input").unwrap_or(plot::COLOUR_INPUT),
            output_fn_sized,
            output_fn_filtered
            )
    };


    
    let output_fn_svg = format!("output/{}.svg", output_fn);
    eprintln!("creating {}", output_fn_svg);
    svg::save(&output_fn_svg, &document).unwrap();

    if matches.is_present("create-html") {

        svg::save("html/image.svg", &document).unwrap();
        let mut raw_svg = String::new();
        BufReader::new(
            File::open("html/image.svg").unwrap()
        ).read_to_string(&mut raw_svg).unwrap();

        let mut template = String::new();
        BufReader::new(
            File::open("html/index.html.template").unwrap()
        ).read_to_string(&mut template).unwrap();

        let html = template.replace("__SVG__", &raw_svg);

        let output_fn_html = format!("html/{}.html", output_fn);
        eprintln!("creating {}", output_fn_html);
        let mut html_file = File::create(output_fn_html).unwrap();
        html_file.write_all(&html.as_bytes()).unwrap();
        let mut html_file = File::create("html/index.html").unwrap();
        html_file.write_all(&html.as_bytes()).unwrap();
    }

}
