use treemap::{Specific, DataPoint};

use std::net::Ipv6Addr;
use ipnetwork::Ipv6Network;

use std::io;
use std::io::prelude::*;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;

use super::*; // ugly 'fix' ?
use easy_csv::{CSVIterator};

use csv;
use hex;

use treebitmap::{IpLookupTable};

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

pub fn prefixes_from_file<'a>(f: &'a str) -> io::Result<IpLookupTable<Ipv6Addr,Specific>> {
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

pub fn asn_colours_from_file<'a>(f: &'a str) -> io::Result<HashMap<u32, String>> {
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
                            meta: ((hex::decode(z.data).unwrap()[3] & 0b1000_0000) >> 7) as u32,
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
