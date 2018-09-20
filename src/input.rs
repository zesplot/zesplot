use treemap::{Specific, DataPoint, PlotInfo};

use std::net::Ipv6Addr;
use ipnetwork::Ipv6Network;

use std::io;
use std::io::prelude::*;
use std::collections::{HashMap,HashSet};
use std::fs::File;
use std::io::BufReader;

//use super::*; // ugly 'fix' ?
//use easy_csv::{CSVIterator};
use std::time::Instant;
use std::process::exit;
use std::path::Path;

use csv;
use hex;

use treebitmap::{IpLookupTable};
use clap::ArgMatches;


//pub fn process_inputs(matches: &ArgMatches) -> IpLookupTable<Ipv6Addr,Specific> {
pub fn process_inputs(matches: &ArgMatches) -> (Vec<Specific> , PlotInfo) {

    let mut datapoints: Vec<DataPoint> = Vec::new();
    let now = Instant::now();
    //match read_datapoints_from_file(matches.value_of("address-file").unwrap(),
    //                                matches.value_of("colour-input").unwrap()) {
    match read_datapoints_from_file(&matches) {
        Ok(dps) => datapoints = dps,
        Err(e) => error!("Can not read datapoints from address-file: {}", e),
    };
                      

    info!("addresses file read: {}.{:.2}s", now.elapsed().as_secs(), now.elapsed().subsec_millis());

    let mut table = prefixes_from_file(matches.value_of("prefix-file").unwrap()).unwrap();

    info!("prefixes: {} , addresses: {}", table.iter().count(), datapoints.len());
    let mut prefix_mismatches = 0;
    let mut asn_to_hits: HashMap<String, usize> = HashMap::new();
    for dp in datapoints {
        if let Some((_, _, s)) = table.longest_match_mut(dp.ip6) {
            s.push_dp(dp);
            let asn_hitcount = asn_to_hits.entry(s.asn.clone()).or_insert(0);
            *asn_hitcount += 1;
        } else {
            prefix_mismatches += 1;
        }
    }

    let unique_asns: HashSet<String> = asn_to_hits.keys().cloned().collect();
    info!("# of ASNs with hits: {}", unique_asns.len());
    
    if prefix_mismatches > 0 {
        warn!("Could not match {} addresses", prefix_mismatches);
    }


    let output_dir = matches.value_of("output-dir").unwrap_or_else(|| "./");
    if matches.is_present("create-addresses") {
        let address_output_fn = format!("{}/{}.addresses",
                    output_dir,
                    Path::new(matches.value_of("address-file").unwrap()).file_name().unwrap().to_str().unwrap(),
        );
        info!("creating address file {}", address_output_fn);
        let mut file = File::create(address_output_fn).unwrap();
        for (_,_,s) in table.iter() {
            for dp in &s.datapoints {
                let _ = writeln!(file, "{}", dp.ip6);
            }
        }
        exit(0);
    }


    // read extra ASN colour info, if any
    let asn_colours = if matches.is_present("asn-colours") {
        asn_colours_from_file(matches.value_of("asn-colours").unwrap()).unwrap()
    } else {
        HashMap::new()
    };

    let mut plot_info = PlotInfo::new(asn_colours.clone());
    plot_info.set_maxes(&table, &matches);

    //TODO: put unsized into plot_info ?
    //let unsized_rectangles = matches.is_present("unsized-rectangles");

    let mut specifics: Vec<Specific>  = table.into_iter().map(|(_,_,s)| s).collect();
    let mut specifics_with_hits = 0;
    for s in &specifics {
        if s.hits() > 0 {
            specifics_with_hits += 1;
        }
    }

    info!("# of specifics: {}", specifics.len());
    info!("# of specifics with hits: {}", specifics_with_hits);
    info!("# of hits in all specifics: {}", specifics.iter().fold(0, |sum, s| sum + s.all_hits())  );

    if matches.is_present("filter-threshold-asn") {
        let minimum = value_t!(matches.value_of("filter-threshold-asn"), usize).unwrap_or_else(|_| 0);
        warn!("got --filter-threshold-asns, only plotting ASNs with minimum hits of {}", minimum);
        let pre_filter_len_specs = specifics.len();
        specifics.retain(|s| *asn_to_hits.get(&s.asn).unwrap_or(&0) >= minimum);
        warn!("filtered {} specifics, left: {}", pre_filter_len_specs - specifics.len(), specifics.len());
    }

    (specifics, plot_info)
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

fn prefixes_from_file(f: &str) -> io::Result<IpLookupTable<Ipv6Addr,Specific>> {
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
                warn!("choked on {} while reading prefixes file", line);
        }
    }; 
    Ok(table)
}

fn asn_colours_from_file(f: &str) -> io::Result<HashMap<u32, String>> {
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

//#[derive(Debug,CSVParsable)] //Deserialize
//struct ZmapRecord {
//    saddr: String,
//    ttl: u8,
//}
//
//#[derive(Debug,CSVParsable)] //Deserialize
//struct ZmapRecordTcpmss {
//    saddr: String,
//    tcpmss: u16
//}
//#[derive(Debug,CSVParsable)] //Deserialize
//struct ZmapRecordDns {
//    saddr: String,
//    data: String
//}

/*
pub fn read_datapoints_from_file<'a, 'b>(f: &'a str, colour_input: &'b str) -> io::Result<Vec<DataPoint>> {
    let mut datapoints: Vec<DataPoint> = Vec::new();

    if f.contains(".csv") {
        // expect ZMAP output as input
        
        let mut rdr = csv::Reader::from_file(f).expect("Failed to open addresses file");
        match colour_input {
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
                            meta: u32::from((hex::decode(z.data).unwrap()[3] & 0b1000_0000) >> 7),
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
                File::open(f).expect("Failed to open addresses file")
            ).lines(){
                let line = line.unwrap();
                datapoints.push(DataPoint { ip6: line.parse().unwrap(), meta: 0 });
            }
    }
    Ok(datapoints)
}
*/


fn read_datapoints_from_file(matches: &ArgMatches) -> io::Result<Vec<DataPoint>> {
    let mut datapoints: Vec<DataPoint>  = Vec::new();

    let address_fn = matches.value_of("address-file").unwrap();
    //if address_fn.contains(".csv") { // TODO this should based on something like --csv 'saddr'
    if matches.is_present("csv-columns"){
        // expect ZMAP/csv output as input
        info!("--csv passed, assuming addresses input in csv format");

        let csv_columns: Vec<&str> = matches.value_of("csv-columns").unwrap().split(',').collect();
        info!("--csv: found {} column(s)", csv_columns.len());
        if csv_columns.len() > 2 {
            warn!("--csv: only using first 2 columns!");
        }
        let csv_addr;
        let csv_meta;
        match csv_columns.len() {
            1 => { csv_addr = csv_columns[0]; csv_meta = ""; }
            i if i >= 2 => { csv_addr = csv_columns[0]; csv_meta = csv_columns[1]; }
            _ => { panic!("need one or two column names to parse csv input"); }
        }

        let mut rdr = csv::Reader::from_path(address_fn)?;
        
        let headers = rdr.headers().unwrap().clone();
        let mut record = csv::StringRecord::new();

        let idx_saddr = headers.iter().position(|r| r == csv_addr)
            .unwrap_or_else(|| panic!("no such column in the csv file: {}", csv_addr));
        if csv_meta != "" {
            let idx_meta = headers.iter()
                .position(|r| r == csv_meta)
                .unwrap_or_else(|| panic!("no such column in the csv file: {}", csv_meta));

            println!("indexes {} and {}", idx_saddr, idx_meta);
            while rdr.read_record(&mut record).unwrap() {
                datapoints.push(
                    DataPoint {
                        ip6: record[idx_saddr].parse().unwrap(),
                        meta: record[idx_meta].parse().unwrap()
                    }
                );
            }

        } else {
            // no second CSV column passed to use (TTL, MSS, etc), so use 0
            while rdr.read_record(&mut record).unwrap() {
                datapoints.push(
                    DataPoint {
                        ip6: record[idx_saddr].parse().unwrap(),
                        meta: 0,
                    }
                );
            }

        }

    } else {
        // expect a simple list of IPv6 addresses separated by newlines
        for line in BufReader::new(
                File::open(address_fn).expect("Failed to open addresses file")
            ).lines(){
                let line = line.unwrap();
                datapoints.push(DataPoint { ip6: line.parse().expect("invalid IPv6 address in input file"), meta: 0 });
            }
    }

    Ok(datapoints)

}
