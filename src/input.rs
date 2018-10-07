use treemap::{Specific, DataPoint, PlotParams};
use treebitmap::{IpLookupTable};

use std::net::Ipv6Addr;
use ipnetwork::Ipv6Network;

use std::io;
use std::io::prelude::*;
use std::collections::{HashMap,HashSet};
use std::fs::File;
use std::io::{BufReader, BufWriter};

use std::time::Instant;
use std::process::exit;
use std::path::Path;

use csv;
use flate2::read::GzDecoder;

use clap::ArgMatches;


pub fn process_inputs(matches: &ArgMatches) -> (Vec<Specific> , PlotParams) {

    let mut datapoints: Vec<DataPoint> = Vec::new();
    let now = Instant::now();
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
        let output_fh = File::create(address_output_fn).unwrap();
        let mut buf = BufWriter::new(output_fh);
        for (_,_,s) in table.iter() {
            for dp in &s.datapoints {
                writeln!(buf, "{}", dp.ip6);
            }
        }
        let _ = buf.flush();
        exit(0);
    }


    let mut plot_params = PlotParams::new(&table, &matches);
    debug!("{:#?}", plot_params);

    // read extra ASN colour info, if any
    if matches.is_present("asn-colours") {
        plot_params.set_asn_colours(asn_colours_from_file(matches.value_of("asn-colours").unwrap()).unwrap());
    }



    debug!("{:#?}", plot_params);


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

    (specifics, plot_params)
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

fn prefixes_from_file(input_fn: &str) -> io::Result<IpLookupTable<Ipv6Addr,Specific>> {
    let mut input = File::open(input_fn)?;
    let mut uncompressed = String::new();
    if input_fn.ends_with(".gz") {
        let mut reader = GzDecoder::new(input);
        reader.read_to_string(&mut uncompressed)?;
    } else {
        let _ = input.read_to_string(&mut uncompressed);
    }

    let mut table: IpLookupTable<Ipv6Addr,Specific> = IpLookupTable::new();

    for line in uncompressed.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        match parts.len() {
            // two column input, e.g. "2001:db8::/32 1234"
            2   => {
                if let Ok(route) = parts[0].parse::<Ipv6Network>() {
                    table.insert(route.ip(), route.prefix().into(),
                        Specific {
                            network: route,
                            asn: parts[1].to_string(), 
                            datapoints: Vec::new(),
                            specifics: Vec::new(),
                            });
                }
            },
            // three column input, e.g. "2001:db8:: 32 1234"
            3   => {
                if let Ok(addr) = parts[0].parse::<Ipv6Addr>() {
                    if let Ok(route) = Ipv6Network::new(addr, parts[1].parse::<u8>().unwrap()) {
                        //.map_err(|_| ZesplotError::Custom(format!("Failed to parse {} as a prefix length", parts[1])))?) {
                    table.insert(route.ip(), route.prefix().into(),
                        Specific {
                            network: route,
                            asn: parts[2].to_string(), 
                            datapoints: Vec::new(),
                            specifics: Vec::new(),
                            });
                    }
                }


            },
            _   => { panic!("can't parse input file, expecting either 2 or 3 columns")},
        }
        //println!("{}", line);
    }


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
