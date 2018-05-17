#![feature(i128, i128_type)]

mod treemap;
use treemap::{Area,Row,Route};
use treemap::*;

use std::collections::HashSet;
use std::collections::HashMap;

extern crate colored;
use colored::*;

extern crate easy_csv;
#[macro_use]
extern crate easy_csv_derive;
extern crate csv;

use easy_csv::{CSVIterator};


use std::time::{Instant};

extern crate treebitmap;
use treebitmap::{IpLookupTable, IpLookupTableOps};
use std::io;

use std::process::exit;

extern crate svg;
extern crate ipnetwork;
extern crate rand;
#[macro_use] extern crate clap;

use clap::{Arg, App};

use svg::*;
use svg::node::Text as Tekst;
use svg::node::element::{Rectangle, Circle, Text, Group};

use ipnetwork::Ipv6Network;
use std::net::Ipv6Addr;
//use num::PrimInt;
//use num::pow::pow;

use std::io::{BufReader};
use std::io::prelude::*;
use std::fs::File;
use std::path::Path;

use rand::{thread_rng, sample};

const WIDTH: f64 = 160.0;
const HEIGHT: f64 = 100.0;
const PLOT_LIMIT: u64 = 2000;
const COLOR_INPUT: &str = "hits";




fn _color(i: u32) -> String  {
    if i == 0 {
        "#eeeeee".to_string()
    } else {
        format!("#00{:02x}{:02x}", 0xFF-i, i)
    }
}

// for the original 'colour == number of hits', we want log2
// for things that are less spread, e.g. avg TTL, the non-log version might give better output
// TODO: try whether some threshold within the same function works
// e.g. if max > 1024, then log2()
fn color(i: u32, max: u32) -> String  {
    if i == 0 {
        "#eeeeee".to_string()
    } else {
        if max > 1024 {
            let norm_factor = (1.0 / ((max as f32).log2() / 255.0)) as f32;
            let v = (norm_factor *(i as f32).log2()) as u32;
            format!("#{:02x}00{:02x}", v, 0xFF-v)
        } else {
            let norm_factor = (1.0 / ((max as f32) / 255.0)) as f32;
            let v = (norm_factor *(i as f32)) as u32;
            format!("#{:02x}00{:02x}", v, 0xFF-v)
        }
    }
}


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

fn prefixes_from_file<'a>(f: &'a str) -> io::Result<IpLookupTable<Ipv6Addr,Route>> {
    let mut file = File::open(f)?;
    let mut s = String::new();
    file.read_to_string(&mut s)?;
    let mut table: IpLookupTable<Ipv6Addr,Route> = IpLookupTable::new();
    for line in s.lines() {
        let parts = line.split_whitespace().collect::<Vec<&str>>();
        //let route: Ipv6Network = parts[0].parse().unwrap();
        if let Ok(route) = parts[0].parse::<Ipv6Network>(){

            let asn = parts[1]; //.parse::<u32>();
                table.insert(route.ip(), route.prefix().into(),
                        Route { prefix: route, asn: asn.to_string(), datapoints: Vec::new()});
            // TODO remove parsing to u32 because of asn_asn,asn notation in pfx2as
            //if let Ok(asn) = asn.parse::<u32>() {
            //    table.insert(route.ip(), route.prefix().into(),
            //            //Route { prefix: route, asn: asn.parse::<u32>().unwrap(), hits: Vec::new()});
            //            Route { prefix: route, asn: asn, hits: Vec::new()});
            //} else {
            //    eprintln!("choked on {} while reading prefixes file", line);
            //}
        } else {
                eprintln!("choked on {} while reading prefixes file", line);
        }
    }; 
    Ok(table)
}

fn prefixes_from_file2<'a>(f: &'a str) -> io::Result<IpLookupTable<Ipv6Addr,Specific>> {
    let mut file = File::open(f)?;
    let mut s = String::new();
    file.read_to_string(&mut s)?;
    let mut table: IpLookupTable<Ipv6Addr,Specific> = IpLookupTable::new();
    for line in s.lines() {
        let parts = line.split_whitespace().collect::<Vec<&str>>();
        //let route: Ipv6Network = parts[0].parse().unwrap();
        if let Ok(route) = parts[0].parse::<Ipv6Network>(){

            let asn = parts[1]; //.parse::<u32>();
                table.insert(route.ip(), route.prefix().into(),
                        Specific { network: route, asn: asn.to_string(), datapoints: Vec::new(), specifics: Vec::new()});
            // TODO remove parsing to u32 because of asn_asn,asn notation in pfx2as
            //if let Ok(asn) = asn.parse::<u32>() {
            //    table.insert(route.ip(), route.prefix().into(),
            //            //Route { prefix: route, asn: asn.parse::<u32>().unwrap(), hits: Vec::new()});
            //            Route { prefix: route, asn: asn, hits: Vec::new()});
            //} else {
            //    eprintln!("choked on {} while reading prefixes file", line);
            //}
        } else {
                eprintln!("choked on {} while reading prefixes file", line);
        }
    }; 
    Ok(table)
}


#[derive(Debug,CSVParsable)] //Deserialize
struct ZmapRecord {
    saddr: String,
//    daddr: String,
//    ipid: u8,
    ttl: u8,
//    sport: u16,
//    dport: u16,
//    classification: String,
//    repeat: u8,
//    cooldown: u8,
//    timestamp_ts: u64,
//    timestamp_us: u32,
//    success: u8,
    //tcpmss: u16
}

#[derive(Debug,CSVParsable)] //Deserialize
struct ZmapRecordTcpmss {
    saddr: String,
    tcpmss: u16
}

#[derive(Eq,PartialEq,Hash,Clone,Debug)]
pub struct DataPoint {
    ip6: Ipv6Addr,
    meta: u32, // meta value, e.g. TTL
}

impl DataPoint {
    fn hamming_weight(&self, prefix_len: u8) -> u32 {
        (u128::from(self.ip6) << prefix_len  >> prefix_len).count_ones()
    }
    fn hamming_weight_iid(&self) -> u32 {
        self.hamming_weight(64)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn hamming_weight() {
        let dp = super::DataPoint { ip6: "2001:db8::1".parse().unwrap(), meta: 0 };
        assert_eq!(dp.hamming_weight(64), 1);
        let dp = super::DataPoint { ip6: "2001:db8::2".parse().unwrap(), meta: 0 };
        assert_eq!(dp.hamming_weight(64), 1);
        let dp = super::DataPoint { ip6: "2001:db8::1:1:1:1".parse().unwrap(), meta: 0 };
        assert_eq!(dp.hamming_weight(64), 4);
        let dp = super::DataPoint { ip6: "2001:db8::1:1:1:1".parse().unwrap(), meta: 0 };
        assert_eq!(dp.hamming_weight(96), 2);
        let dp = super::DataPoint { ip6: "2001:db8::3:3:3:3".parse().unwrap(), meta: 0 };
        assert_eq!(dp.hamming_weight(64), 2+2+2+2);
    }
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
                        .arg(Arg::with_name("unsized-rectangles")
                             .short("u")
                             .long("unsized")
                             .help("Do not size the rectangles based on prefix length, but size them all equally")
                        )
                        .arg(Arg::with_name("color-input")
                             .short("c")
                             .long("color-input")
                             .help("Base the colours on any of the following:
                                \"hits\" (default)
                                \"hw\" (average hamming weight in prefix)
                                \"mss\" (average TCP MSS in prefix)
                                \"ttl\" (average TTL of responses in prefix, only when using ZMAP input)")
                             .takes_value(true)
                             .required(true)
                        )
                        .arg(Arg::with_name("draw-hits")
                             .short("d")
                             .long("draw-hits")
                             .help("Plot addresses on their respective areas")
                        )
                        .arg(Arg::with_name("plot-limit")
                             .short("l")
                             .long("limit")
                             .help(&format!("Limits number of areas plotted. 0 for unlimited. Default {}", PLOT_LIMIT))
                             .takes_value(true)
                        )
                        .arg(Arg::with_name("create-html")
                             .long("html")
                             .help(&format!("Create HTML wrapper output in ./html"))
                        )
                        .arg(Arg::with_name("create-prefixes")
                             .long("create-prefixes")
                             .help(&format!("Create file containing prefixes based on hits from address-file, and exit"))
                        )
                        .get_matches();

    eprintln!("-- reading input files");

    let mut datapoints: Vec<DataPoint> = Vec::new();
    //let mut datapoints: Vec<DataPoint> = Vec::with_capacity(5_000_000);
    //TODO: do we want to filter duplicate addresses from the input file?
    // the current test file contains ~2k duplicates on 4.6M entries
    let mut uniq_ip6s: HashSet<Ipv6Addr> = HashSet::new();
    let mut uniq_dps: HashSet<DataPoint> = HashSet::new();

    let mut now = Instant::now();
    if matches.value_of("address-file").unwrap().contains(".csv") {
        // expect ZMAP output as input
        
        let mut rdr = csv::Reader::from_file(matches.value_of("address-file").unwrap()).unwrap();
        //let iter = CSVIterator::<ZmapRecord,_>::new(&mut rdr).unwrap();
        match matches.value_of("color-input").unwrap() {
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
                }
            }
        }
        //for zmap_record in iter {
        //    let z = zmap_record.unwrap();
        //    datapoints.push(
        //        DataPoint { 
        //            ip6: z.saddr.parse().unwrap(),
        //            meta: z.tcpmss.into()
        //        }
        //    );
        //    //uniq_dps.insert(
        //    //    DataPoint { 
        //    //        ip6: z.saddr.parse().unwrap(),
        //    //        meta: z.ttl.into()
        //    //    }
        //    //);
        //    //if !uniq_ip6s.insert(z.saddr.parse().unwrap()) {
        //    //    //eprintln!("duplicate: {}", z.saddr.parse::<Ipv6Addr>().unwrap());
        //    //}
        //}
        

        // attempt at improving read speed:
        /*
        let mut file = File::open(matches.value_of("address-file").unwrap()).unwrap();
        let mut s = String::new();
        file.read_to_string(&mut s).unwrap();
        let mut rdr = csv::Reader::from_string(s);
        eprintln!("[TIME] file read: {}.{:.2}s", now.elapsed().as_secs(),  now.elapsed().subsec_nanos() / 1_000_000);
        //let iter = CSVIterator::<ZmapRecord,_>::new(&mut rdr).unwrap();
        let iter = CSVIterator::<ZmapRecord,_>::new(&mut rdr).unwrap();
        let res: Vec<ZmapRecord> = iter.filter_map(|e| e.ok()).collect();
        eprintln!("[TIME] iter.collect(): {}.{:.2}s", now.elapsed().as_secs(),  now.elapsed().subsec_nanos() / 1_000_000);
        //for zmap_record in iter {
        for zmap_record in res {
            let z = zmap_record;
            datapoints.push(
                DataPoint { 
                    ip6: z.saddr.parse().unwrap(),
                    meta: z.ttl.into()
                }
            );
        }
        */


        // this is not significantly faster:
        //datapoints.append(&mut iter.map(|i| i.unwrap().saddr.parse().unwrap()).collect::<Vec<_>>());

//
//        for result in rdr.deserialize() {
//            // The iterator yields Result<StringRecord, Error>, so we check the
//            // error here.
//            let record : ZmapRecord = result.unwrap();
//            //println!("{:?}", record.saddr);
//            datapoints.push(record.saddr.parse().unwrap());
//        }
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

    eprintln!("uniq_ip6s: {}", uniq_ip6s.len());
    eprintln!("uniq_dps: {}", uniq_dps.len());

    now = Instant::now();
    let table = prefixes_from_file(matches.value_of("prefix-file").unwrap()).unwrap();
    let table2 = prefixes_from_file2(matches.value_of("prefix-file").unwrap()).unwrap();

    //eprintln!("-- matching /128s with prefixes");

    eprintln!("prefixes: {} , addresses: {}", table.iter().count(), datapoints.len());
    let mut prefix_mismatches = 0;
    for dp in datapoints.into_iter() {
        if let Some((_, _, r)) = table.longest_match(dp.ip6) {
            r.push_dp(dp.clone());
        } else {
            //eprintln!("could not match {:?}", dp.ip6);
            prefix_mismatches += 1;
        }
        if let Some((_, _, r)) = table2.longest_match(dp.ip6) {
            r.push_dp(dp);
        }
    }
    eprintln!("table 2 filled");
    
    if prefix_mismatches > 0 {
        let s = format!("Could not match {} addresses", prefix_mismatches).to_string().on_red().bold();
        eprintln!("{}", s);
    }


    // maximum values to determine colour scale later on
    let mut max_hits = 0;
    let mut max_meta = 0f64; // based on DataPoint.meta, e.g. TTL
    let mut max_hamming_weight = 0f64;
    let mut total_area = 0_u128;
    let unsized_rectangles = matches.is_present("unsized-rectangles");
    
    // sum up the sizes of all the prefixes:
    // and find the max hits for the colour scale
    for (_,_,r) in table.iter() {
        total_area += r.size(unsized_rectangles);
        if r.datapoints.len() > max_hits {
            max_hits = r.datapoints.len();
        }
        if r.dp_avg() > max_meta {
            max_meta = r.dp_avg();
        }
        if r.hw_avg() > max_hamming_weight {
            max_hamming_weight = r.hw_avg();
        }
    }
    //eprintln!("total_area: {}", total_area);
    //eprintln!("max_hits: {}", max_hits);
    //eprintln!("max_meta: {}", max_meta);
    //eprintln!("max_hamming_weight: {}", max_hamming_weight);

    let mut routes: Vec<Route> = table.into_iter().map(|(_,_,r)| r).collect();
    let mut specifics: Vec<Specific>  = specs_to_hier2(&table2.into_iter().map(|(_,_,s)| s).collect());






    if matches.is_present("filter-empty-prefixes") {
        let pre_filter_len = routes.len();
        routes.retain(|r| r.datapoints.len() > 0);
        total_area = routes.iter().fold(0, |mut s, r|{s += r.size(unsized_rectangles); s});
        eprintln!("filtered {} empty prefixes, left: {}", pre_filter_len - routes.len(), routes.len());
    } else {
        eprintln!("no filtering of empty prefixes");
    }

    eprintln!("# of specifics: {}", specifics.len());
    eprintln!("# of hits in all specifics: {}", specifics.iter().fold(0, |sum, s| sum + s.all_hits())  );
    //println!("---");
    //for s in &specifics {
    //    println!("{} {}", s.network, s.all_hits());
    //    //println!("  {:?}", s.datapoints);
    //    for s2 in &s.specifics {
    //        println!("  {} {}", s2.network, s2.all_hits());
    //        //println!("    {:?}", s2.datapoints);
    //        for s3 in &s2.specifics {
    //            println!("    {}", s3.network);
    //            for s4 in &s3.specifics {
    //                println!("      {}", s4.network);
    //                for s5 in &s4.specifics {
    //                    println!("        {} {}", s5.network, s5.all_hits());
    //                }
    //            }
    //        }
    //    }
    //}
    //println!("---");

    if matches.is_present("create-prefixes") {
        routes.retain(|r| r.datapoints.len() > 0);
        let prefix_output_fn = format!("output/{}.prefixes",
                    Path::new(matches.value_of("address-file").unwrap()).file_name().unwrap().to_str().unwrap(),
        );
        eprintln!("creating prefix file {}", prefix_output_fn);
        let mut file = File::create(prefix_output_fn).unwrap();
        for r in routes {
            let _ = writeln!(file, "{} {}", r.prefix, r.asn);
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
    let init_ar: f64 = 1_f64 / (8.0/1.0);

    let norm_factor = (WIDTH * HEIGHT) / total_area as f64;

    let mut areas: Vec<Area> = Vec::new();
    //let mut areas2: Vec<Area2> = Vec::new();

    // sort by both size and ASN, so ASs are grouped in the final plot
    // FIXME size() is confusing:
    //   there is the actual prefix size, e.g. /32
    //   and there is our size() that might do (128-32)^2, or something else
    //   for now, use prefix_len() to sort
    //   and keep size() to adjust sizes of the rectangles to get some reasonable output

    //routes.sort_by(|a, b| b.size().cmp(&a.size()).then(a.asn.cmp(&b.asn))  );
    routes.sort_by(|a, b| b.prefix_len().cmp(&a.prefix_len()).reverse().then(a.asn.cmp(&b.asn))  );

    for r in routes {
        //areas.push(Area::new(r.size(unsized_rectangles) as f64 * norm_factor, init_ar, r  ));
    }

    specifics.sort_by(|a, b| b.prefix_len().cmp(&a.prefix_len()).reverse().then(a.asn.cmp(&b.asn))  );

    for s in specifics {
        //println!("{}", s.network);
        areas.push(Area::new(s.size(unsized_rectangles) as f64 * norm_factor, init_ar, s  ));
    }



// NEW HIERACHICAL MODEL, try to visualize it now:


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

    //let mut rects: Vec<Rectangle> = Vec::new();
    //let mut labels: Vec<Text> = Vec::new();
    let mut groups: Vec<Group> = Vec::new();
    let mut areas_plotted: u64 = 0;

    //let plot_limit = matches.value_of("plot-limit").unwrap_or(PLOT_LIMIT);
    let plot_limit = value_t!(matches, "plot-limit", u64).unwrap_or(PLOT_LIMIT);
    //eprintln!("plot_limit: {}", plot_limit);
    for row in rows {
        //println!("new row: {}", direction);
        
        if plot_limit > 0 && areas_plotted >= plot_limit {
            break;
        }

        for area in row.areas {
            let mut group = Group::new()
                // FIXME route -> specific
                //.set("data-asn", area.specific.asn.to_string())
                //.set("data-prefix", area.specific.network.to_string())
                //.set("data-hits", area.route.datapoints.len().to_string())
                //.set("data-dp-avg", format!("{:.1}", area.route.dp_avg()))
                //.set("data-hw-avg", format!("{:.1}", area.route.hw_avg()))
                ;

            let mut border = 0.0005 * area.surface;
            if border > 0.4 {
                border = 0.4;
            }

            let mut rect = Rectangle::new()
                .set("x", area.x)
                .set("y", area.y)
                .set("width", area.w)
                .set("height", area.h)
                //.set("fill", color(area.route.datapoints.len() as u32, max_hits as u32)) 
                //.set("fill", color(area.route.hw_avg() as u32, max_hamming_weight as u32)) 
                //.set("fill", color(area.route.dp_avg() as u32, max_meta as u32)) 
                .set("stroke-width", border)
                .set("stroke", "black")
                .set("opacity", 1.0)
                ;

            //FIXME route -> specific
            //let rect = match matches.value_of("color-input").unwrap_or(COLOR_INPUT) {
            //    "hw"        => rect.set("fill", color(area.route.hw_avg() as u32, max_hamming_weight as u32)),
            //    "ttl"|"mss" => rect.set("fill", color(area.route.dp_avg() as u32, max_meta as u32)),
            //    "hits"|_    => rect.set("fill", color(area.route.datapoints.len() as u32, max_hits as u32)),
            //};
            let rect = rect.set("fill", "white");
            //group.append(rect);

// HIERARCHICAL STUFF ATTEMPT
            //if area.specific.specifics.len() > 0 {
            //    let mut sub_area_x = area.x;
            //    let mut sub_area_y = area.y;
            //    for s in &area.specific.specifics {
            //        println!("parent size: {}", s.size(false) as f64 );
            //        println!("sub size: {}", s.size(false) as f64 / area.specific.size(false) as f64);
            //        //let sub_width = 2.0 * area.w * (s.size(false) as f64 / area.specific.size(false) as f64 ) ;
            //        let sub_width = (area.w / area.specific.specifics.len() as f64);
            //        let mut rect = Rectangle::new()
            //            .set("x", sub_area_x)
            //            .set("y", sub_area_y)
            //            .set("width", sub_width)
            //            .set("height", area.h / 2.0 )
            //            //.set("stroke-width", border)
            //            .set("stroke-width", border / 2.0)
            //            .set("stroke", "black")
            //            .set("opacity", 1.0)
            //            .set("fill", "white")
            //            ;
            //            group.append(rect);
            //        sub_area_x += sub_width;
            //        sub_area_y += 0.0 ;
            //    }

            //}

            let sub_rects = area.specific.all_rects(&area);
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


            if area.w > 0.5 {
                let mut label = Text::new()
                    .set("x", area.x + area.w/2.0)
                    .set("y", area.y + area.h/2.0)
                    .set("font-family", "mono")
                    .set("font-size", format!("{}%", area.w.min(area.h))) // == f64::min
                    .set("text-anchor", "middle");
                    label.append(Tekst::new(area.specific.to_string()))
                    ;
                group.append(label);
            }
            groups.push(group);



            areas_plotted += 1;
        }
    }

    eprintln!("plotting {} rectangles, limit was {}", areas_plotted, plot_limit);

    let mut document = Document::new()
                        .set("viewBox", (0, 0, WIDTH, HEIGHT))
                        .set("id", "treeplot")
                        ;
    for g in groups {
        document.append(g);
    }

    //eprintln!("-- creating output files");

    let output_fn_sized = if matches.is_present("unsized-rectangles") {
        "unsized"
    } else {
        "sized"
    };
    let output_fn_filtered = if matches.is_present("filter-empty-prefixes") {
        "filtered"
    } else {
        "unfiltered"
    };
    let output_fn = format!("output/{}.{}.{}.{}.svg", Path::new(matches.value_of("address-file").unwrap()).file_name().unwrap().to_str().unwrap(),
        matches.value_of("color-input").unwrap_or(COLOR_INPUT),
        output_fn_sized,
        output_fn_filtered
        );
    eprintln!("creating {}", output_fn);
    svg::save(output_fn, &document).unwrap();

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

        let mut html_file = File::create("html/index.html").unwrap();
        html_file.write_all(&html.as_bytes()).unwrap();
    }

}
