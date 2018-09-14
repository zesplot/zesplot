use plot;

use ipnetwork::Ipv6Network;
use std::net::Ipv6Addr;

use treebitmap::{IpLookupTable};

use svg::Node;
use svg::node::element::Rectangle;

use std::collections::{HashMap, HashSet};
use clap::ArgMatches;

#[derive(Debug, Clone)]
pub struct Specific {
    pub network: Ipv6Network,
    pub asn: String,
    pub datapoints: Vec<super::DataPoint>,
    pub specifics: Vec<Specific>
}

#[derive(Eq,PartialEq,Hash,Clone,Debug)]
pub struct DataPoint {
    pub ip6: Ipv6Addr,
    pub meta: u32, // meta value, e.g. TTL, MSS
}

#[derive(Copy,Clone)]
pub struct Turtle {
    x: f64, y: f64, w: f64, h: f64
}

impl DataPoint {
    fn hamming_weight(&self, prefix_len: u8) -> u32 {
        (u128::from(self.ip6) << prefix_len  >> prefix_len).count_ones()
    }
    #[allow(dead_code)]
    fn hamming_weight_iid(&self) -> u32 {
        self.hamming_weight(64)
    }
    #[allow(dead_code)]
    fn ttl_to_start_value(&mut self) -> () {
        self.meta = match self.meta {
            0...31 => 32,
            32...63 => 64,
            64...127 => 128,
            128...255 => 255,
            _ => self.meta
        };
    }
    #[allow(dead_code)]
    pub fn ttl_to_path_length(&mut self) -> () {
        if self.meta > 128  {
            self.meta -= 1;
        }
        self.meta = 64 - (self.meta % 64);
    }
}

pub enum ColourMode {
    Hits,
    DpAvg,
    DpMedian,
    DpVar,
    DpUniq,
    DpSum,
    HwAvg,
    Asn
}
    

pub struct PlotInfo {
    pub max_hits: usize,
    pub max_dp_avg: f64,
    pub max_dp_median: f64,
    pub max_dp_var: f64,
    pub max_dp_uniq: usize,
    pub max_dp_sum: usize,
    pub max_hw_avg: f64,
    pub colour_mode: ColourMode,
    pub dp_desc: String,
    pub asn_colours: HashMap<u32, String>
}

impl PlotInfo {
    pub fn new(asn_colours: HashMap<u32, String>) -> PlotInfo {
        PlotInfo { 
            max_hits: 0,
            max_dp_avg: 0f64,
            max_dp_median: 0f64,
            max_dp_var: 0f64,
            max_dp_uniq: 0,
            max_dp_sum: 0,
            max_hw_avg: 0f64,
            colour_mode: ColourMode::Hits,
            dp_desc: "".to_string(),
            asn_colours
        }
    }
    pub fn set_maxes(&mut self, table: &IpLookupTable<Ipv6Addr,Specific>, matches: &ArgMatches) {
        for (_,_,s) in table.iter() {
            if s.datapoints.len() > self.max_hits {
                //max_hits = s.datapoints.len();
                self.max_hits = s.datapoints.len();
            }
            // based on dp.meta:
            if s.dp_avg() > self.max_dp_avg {
                self.max_dp_avg = s.dp_avg();
            }
            if s.dp_median() > self.max_dp_median {
                self.max_dp_median = s.dp_median();
            }
            if s.dp_var() > self.max_dp_var {
                self.max_dp_var = s.dp_var();
            }
            if s.dp_uniq() > self.max_dp_uniq {
                self.max_dp_uniq = s.dp_uniq();
            }
            if s.dp_sum() > self.max_dp_sum {
                self.max_dp_sum = s.dp_sum();
            }
            // hamming weight:
            if s.hw_avg() > self.max_hw_avg {
                self.max_hw_avg = s.hw_avg();
            }
        }
        info!("maximums (for --scale-max):");
        info!("max_hits: {}", self.max_hits);
        if matches.is_present("scale-max") {
            warn!("overruling max_hits, was {}, now is {}", self.max_hits, matches.value_of("scale-max").unwrap());
            self.max_hits = matches.value_of("scale-max").unwrap().parse::<usize>().unwrap();
        }

        self.dp_desc = if matches.is_present("legend-label") {
            matches.value_of("legend-label").unwrap().to_string()
        } else {
            match matches.value_of("colour-input").unwrap_or(plot::COLOUR_INPUT) {
                "ttl"   => "TTL".to_string(),
                "mss"   => "TCP MSS".to_string(),
                "dns"   => "DNS RA bit".to_string(),
                "hw"    => {self.colour_mode = ColourMode::HwAvg;  "Hamming Weight".to_string()},
                "hits"|_ => "Hits".to_string()
            }
        };

        if matches.is_present("dp-function") {
            self.colour_mode = match matches.value_of("dp-function").unwrap() {
                "avg" => ColourMode::DpAvg,
                "median" => ColourMode::DpMedian,
                "var" => ColourMode::DpVar,
                "uniq" => ColourMode::DpUniq,
                "sum" => ColourMode::DpSum,
                _   =>  ColourMode::Hits,
            };
        } else if matches.is_present("asn-colours") {
            self.colour_mode = ColourMode::Asn;
        } else if self.dp_desc == "TTL" || self.dp_desc == "TCP MSS" { //ugly..
            self.colour_mode = ColourMode::DpAvg;
        }



    }
}

pub fn var(s: &[u32]) -> f64 {
    if s.len() < 2 {
        return 0.0;
    }
    let sum: u32 = s.iter().sum();
    let mean = f64::from(sum) / s.len() as f64;
    //let var = s.iter().fold(0.0, |var, dp| var as f64 + (*dp as f64 - mean).powf(2.0) ) / (s.len() -1) as f64;
    //var
    s.iter().fold(0.0, |var, dp| var as f64 + (f64::from(*dp) - mean).powf(2.0) ) / (s.len() -1) as f64
}


pub fn median(s: &[u32]) -> f64 {
    if s.is_empty() {
        return 0.0;
    }
    let mut sorted = s.to_owned();
    sorted.sort();
    if sorted.len() % 2  == 0 {
        //((*sorted.get(sorted.len() / 2).unwrap() as f64 + *sorted.get(sorted.len() / 2 - 1).unwrap() as f64) / 2.0) as f64
        ((f64::from(sorted[sorted.len() / 2]) + f64::from(sorted[sorted.len() / 2 - 1])) / 2.0) as f64
    } else {
        //*sorted.get(sorted.len() / 2).unwrap() as f64
        f64::from(sorted[sorted.len() / 2])
    }
}


impl Specific {
    pub fn push_dp(&mut self, dp: super::DataPoint) -> () {
        self.datapoints.push(dp);
    }

    pub fn dp_avg(&self) -> f64 {
        let sum = self.datapoints.iter().fold(0, |s, i| s + i.meta);
        f64::from(sum) / self.datapoints.len() as f64
    }

    pub fn dp_var(&self) -> f64 {
        var(&self.datapoints.iter().map(|dp| dp.meta).collect::<Vec<u32>>().as_slice())
    }

    pub fn dp_median(&self) -> f64 {
        median(&self.datapoints.iter().map(|dp| dp.meta).collect::<Vec<u32>>().as_slice())
    }

    pub fn dp_uniq(&self) -> usize {
        let mut uniq_meta: HashSet<u32> = HashSet::new();
        for dp in &self.datapoints {
            uniq_meta.insert(dp.meta);
        }
        uniq_meta.len()
    }

    pub fn dp_sum(&self) -> usize {
        self.datapoints.iter().fold(0, |s, i| s + i.meta as usize)
    }

    pub fn hw_avg(&self) -> f64 {
        let sum = self.datapoints.iter().fold(0, |s, i| s + i.hamming_weight(self.prefix_len()));
        f64::from(sum) / self.datapoints.len() as f64
    }

    #[allow(dead_code)]
    pub fn dps_ttl_to_path_length(&mut self) -> () {
        for mut dp in &mut self.datapoints {
            dp.ttl_to_path_length();
        }
    }

    
    pub fn all_hits(&self) -> usize {
        self.hits() + self.hits_in_specifics()
    }

    pub fn hits(&self) -> usize {
        self.datapoints.len()
    }

    pub fn hits_in_specifics(&self) -> usize {
        let mut hits = 0;
        for s in &self.specifics {
            hits += s.hits();
            hits += s.hits_in_specifics();
        }
        hits
    }

    pub fn size(&self, unsized_rectangles: bool) -> u128 {
        if unsized_rectangles {
            1u128
        } else {
            self.__size()
        }
    }

    pub fn __size(&self) -> u128 {
        // while 2^ is more accurate, 1.2^ results in more readable plots
        // possibly parametrize this
        1.2_f64.powf(128.0 - f64::from(self.network.prefix())) as u128
    }
    
    pub fn prefix_len(&self) -> u8 {
        self.network.prefix()
    }

    pub fn to_string(&self) -> String {
        format!("AS{}", &self.asn)
    }

    pub fn asn(&self) -> u32 {
        if let Ok(i) = self.asn.parse::<u32>(){
            i
        } else {
            0
        }
    }

//#[allow(clippy::too_many_arguments)]
    //pub fn to_rect(&self, x: f64, y: f64, w: f64, h: f64, w_factor: f64, h_factor: f64, plot_info: &PlotInfo) -> super::Rectangle {
    pub fn to_rect(&self, t: Turtle, w_factor: f64, h_factor: f64, plot_info: &PlotInfo) -> Rectangle {
        let Turtle {x, y, w, h} = t;
        let mut r = Rectangle::new()
            .set("x", x)
            .set("y", y)
            .set("width", w * w_factor)
            .set("height", h * h_factor)
            .set("stroke-width", 0.5_f64.min(0.0001_f64.max(w * h * 0.0005 * h_factor)))
            .set("stroke", "#aaaaaa")
            .set("opacity", 1.0)
            .set("data-asn", self.asn.to_string())
            .set("data-prefix", self.network.to_string())
            .set("data-self-hits", self.hits())
            .set("data-hits", self.all_hits())
            .set("data-dp-desc", plot_info.dp_desc.clone())
            .set("data-dp-avg", format!("{:.1}", self.dp_avg()))
            .set("data-dp-median", format!("{:.1}", self.dp_median()))
            .set("data-dp-var", format!("{:.1}", self.dp_var()))
            .set("data-dp-uniq", format!("{:.1}", self.dp_uniq()))
            .set("data-dp-sum", format!("{:.1}", self.dp_sum()))
            .set("data-hw-avg", format!("{:.1}", self.hw_avg()))
            ;

        match plot_info.colour_mode {
            ColourMode::Hits => r.assign("fill", colour(self.hits() as u32, plot_info.max_hits as u32)),
            ColourMode::DpAvg => r.assign("fill", colour(self.dp_avg() as u32, plot_info.max_dp_avg as u32)),
            ColourMode::DpMedian => r.assign("fill", colour(self.dp_avg() as u32, plot_info.max_dp_median as u32)),
            ColourMode::DpVar => r.assign("fill", colour(self.dp_var() as u32, plot_info.max_dp_var as u32)),
            ColourMode::DpUniq => r.assign("fill", colour(self.dp_uniq() as u32, plot_info.max_dp_uniq as u32)),
            ColourMode::DpSum => r.assign("fill", colour(self.dp_sum() as u32, plot_info.max_dp_sum as u32)),
            ColourMode::HwAvg => r.assign("fill", colour(self.hw_avg() as u32, plot_info.max_hw_avg as u32)),
            ColourMode::Asn => r.assign("fill", colour_from_map(self.asn(), &plot_info.asn_colours))
        }
        r

        // TODO: re-implement the drawing of addresses as dots within the prefix rectangle
        // NB: the stuff below was an earlier attempt based on the OLD data model!
        /*
        if matches.is_present("draw-hits") {
            let mut rng = thread_rng();
            let sample = sample(&mut rng, &area.route.datapoints, 1000); 
            //println!("took {} as sample from {}", sample.len(), area.route.datapoints.len());
            let mut g_hits = Group::new(); 
            let first_ip = u128::from(area.route.prefix.iter().next().unwrap());
            let mut u = area.surface / (area.route.prefix.size()) as f64; 

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

    }

//#[allow(clippy::too_many_arguments)]
    //pub fn rects_in_specifics(&self, x: f64, y: f64, w: f64, h: f64, w_factor: f64, h_factor: f64, plot_info: &PlotInfo) -> Vec<super::Rectangle> {
    pub fn rects_in_specifics(&self, t: Turtle, w_factor: f64, h_factor: f64, plot_info: &PlotInfo) -> Vec<Rectangle> {
        if self.specifics.is_empty() {
            return vec![]
        }
        let Turtle {x, y, w, h} = t;
        let w_factor  = w_factor / self.specifics.len() as f64;
        let mut results = Vec::new();
        let mut x = x;
        for s in &self.specifics {
            results.push(s.to_rect(Turtle{x, y, w, h}, w_factor, h_factor, plot_info));
            let sub_w_factor  = w_factor; 
            results.append(&mut s.rects_in_specifics(Turtle{x, y, w, h}, sub_w_factor, h_factor / 2.0, plot_info));
            x += w * w_factor; 
        }

    results
    }

    pub fn all_rects(&self, area: &Area, plot_info: &PlotInfo) -> Vec<Rectangle> {
        let t = Turtle {x: area.x, y: area.y, w: area.w, h: area.h};
        let mut result = vec![self.to_rect(t, 1.0, 1.0, plot_info)];
        result.append(&mut self.rects_in_specifics(t, 1.0, 0.5, plot_info));
        result
    }
}


//pub fn specs_to_hier_with_rest_index(specifics: &Vec<Specific>, index: usize) -> (Vec<Specific>, usize) {
pub fn specs_to_hier_with_rest_index(specifics: &[Specific], index: usize) -> (Vec<Specific>, usize) {
    let current_specific: &Specific;
    if let Some((first, rest)) = specifics[index..].split_first() {
        current_specific = first;
        if rest.is_empty() {
            //println!("NO REST, returning {}", first.network.ip());
            return (vec![first.clone()], 1);
        }

        let mut nested_specs: Vec<Specific> = Vec::new();
        let mut consumed_specs = 1;
        for s in rest.iter() {
            if current_specific.network.contains(s.network.ip()) {
                //println!("  in current: {}", s.network.ip());
                nested_specs.push(s.clone());
                consumed_specs += 1;
            } else {
                //println!("  NOT in current: {}", s.network.ip());
                break;
            }

        }

        let result = vec![Specific { network: first.network, asn: first.asn.clone(), datapoints: first.datapoints.clone(),
                specifics: specs_to_hier(&nested_specs) }];
        return (result, consumed_specs)
    } else {
        println!("could not satisfy Some(), len: {}", specifics.len());
    }
    println!("returning empty vector..");
    (vec![], 0)
}

//pub fn specs_to_hier(specifics: &Vec<Specific>) -> Vec<Specific> {
pub fn specs_to_hier(specifics: &[Specific]) -> Vec<Specific> {
    let mut done = false;
    let mut all_results: Vec<Specific> = vec![];
    let mut start_from = 0;
    
    if specifics.is_empty() {
        return vec![];
    }
    
    if specifics.len() == 1 {
        //return specifics.clone();
        return specifics.to_owned();
    }
    
    while !done {
        let (mut result, num_consumed) = specs_to_hier_with_rest_index(&specifics, start_from);
        if result.is_empty() && num_consumed  == 0 {
            done = true;
        }
        start_from += num_consumed;
        if specifics.len() == start_from {
            done = true;
        }
        all_results.append(&mut result);
    }
    
    all_results
}

pub struct Area {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub surface: f64,
    pub specific: Specific,
}

pub struct Row {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub vertical: bool,
    pub areas: Vec<Area>,
}

impl Area {
    pub fn new(surface: f64, ratio: f64, specific: Specific) -> Area {
        let w = surface.powf(ratio);
        let h = surface.powf(1.0 - ratio);
        Area { x: 0.0, y: 0.0, w, h, surface, specific }
    }
    pub fn get_ratio(&self) -> f64 {
        if self.h >= self.w {
            self.w  / self.h
        } else {
            self.h / self.w
        }
    }
}


impl Row {
    pub fn new(x: f64, y: f64, vertical: bool, mut area: Area) -> Row {
        let max_h = plot::HEIGHT - y;
        let max_w = plot::WIDTH - x;
        if vertical {
            area.h = max_h;
            area.w = area.surface / area.h;
        } else {
            area.w = max_w;
            area.h = area.surface / area.w;
        }
        Row {x, y, w: area.w, h: area.h, vertical, areas:vec![area]}
    }

    pub fn try(&mut self, area: Area) -> Option<Area> {
        let cur_worst = self.calc_worst();
        self.push(area);

        if self.calc_worst() >= cur_worst {
            None
        } else {
            self.pop()
        }
    }


    pub fn reflow(&mut self) -> () {
        if self.vertical {
            let new_w = self.area() / self.h;
            self.w = new_w;
            let mut cur_y = self.y;
            for a in &mut self.areas {
                a.h = a.surface / new_w;
                a.w = new_w;
                a.x = self.x;
                a.y = cur_y;
                cur_y += a.h;
            }
        } else {
            let new_h = self.area() / self.w;
            self.h = new_h;
            let mut cur_x = self.x;
            for a in &mut self.areas {
                a.w = a.surface / new_h;
                a.h = new_h;
                a.y = self.y;
                a.x = cur_x;
                cur_x += a.w;
            }
        }
    }

    fn pop(&mut self) -> Option<Area> {
        let popped = self.areas.pop();
        self.reflow();
        popped
    }

    fn push(&mut self, mut area: Area) -> () {
        if self.vertical {
            area.x = self.x;
        } else {
            area.y = self.y;
        }

        self.areas.push(area);
        self.reflow();
    }

    fn area(&self) -> f64 {
        self.areas.iter().fold(0.0, |mut s, a| {s += a.surface; s})
    }

    fn calc_worst(&self) -> f64 {
        self.areas.iter().fold(1.0, |mut w, a| {
            if a.get_ratio() < w {
                w = a.get_ratio();
            }
            w
        })
    }

}


// for things that are less spread, e.g. avg TTL, the non-log version might give better output
// try whether some threshold within the same function works
// e.g. if max > 1024, then log2()
// const COLOUR_SCALE: Vec::<u32> = (0..0xff+1).map(|e| 0xff | (e << 8)).collect();
//FIXME: recreating the scale everytime is ugly

fn colour(i: u32, max: u32) -> String {
    if i == 0 {
        return "#eeeeee".to_string();
    }

    let mut scale: Vec<u32> = (0..=0xff).map(|e| 0xff | (e << 8)).collect();
    scale.append(&mut (0..=0xff).rev().map(|e| (0xff << 8) | e).collect::<Vec<u32>>() );
    scale.append(&mut (0..=0xff).map(|e| 0xff00 | (e << 16) | e).collect::<Vec<u32>>() );
    scale.append(&mut (0..=0xff).rev().map(|e| 0x00ff_0000 | (e << 8)).collect::<Vec<u32>>() );

    if max > 1024 {
        let norm = scale.len() as f64 / (f64::from(max)).log2();
        let mut index = (f64::from(i).log2() * norm) as usize;
        //FIXME: this should not be necessary..
        if index >= scale.len() {
            index = scale.len() - 1;
        }
        if index == 0 {
            index = 1;
        }
        format!("#{:06x}", &scale[index])
    } else {
        let norm = scale.len() as f64 / f64::from(max);
        let mut index = (f64::from(i) * norm) as usize;
        //FIXME: this should not be necessary..
        if index >= scale.len() {
            index = scale.len() - 1;
        }
        if index == 0 {
            index = 1;
        }
        format!("#{:06x}", &scale[index])
    }
}

// used for asn -> id mapping
fn colour_from_map(asn: u32, mapping: &HashMap<u32, String>) -> String {

    /*
    if !mapping.contains_key(&asn) {
        return "#eeeeee".to_string();
    }

    let uniq_values: HashSet<String> = HashSet::from_iter(mapping.values().cloned());
    let mut uniq_sorted_values: Vec<String> = uniq_values.into_iter().collect();
    uniq_sorted_values.sort();
    let num_distinct_colours = uniq_sorted_values.len();
    let index = uniq_sorted_values.iter().position(|e| e == mapping.get(&asn).unwrap()).unwrap();
    */


    let scale: HashMap<String, &str> = [  ("cluster0".to_string(),   "#ff0000"),
                                            ("cluster1".to_string(), "#ffff00"),
                                            ("cluster2".to_string(), "#00ff00"),
                                            ("cluster3".to_string(), "#ff00ff"),
                                            ("cluster4".to_string(), "#00ffff"),
                                            ("cluster5".to_string(), "#0000ff"),
                                            ].iter().cloned().collect();

    // unwrap_or for non-existing asn-to-cluster mappings, eventually ending up in the gray #eee colour
    scale.get(mapping.get(&asn).unwrap_or(&"_".to_string())).unwrap_or(&"#eeeeee").to_string()
    //colour(index as u32, num_distinct_colours as u32)
}


#[cfg(test)]
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

#[test]
fn ttl_to_start_value() {
    let mut dp = super::DataPoint { ip6: "2001:db8::1".parse().unwrap(), meta: 111 } ;
    dp.ttl_to_start_value();
    assert_eq!(dp.meta, 128);

    let mut dp = super::DataPoint { ip6: "2001:db8::1".parse().unwrap(), meta: 59 } ;
    dp.ttl_to_start_value();
    assert_eq!(dp.meta, 64);

    let mut dp = super::DataPoint { ip6: "2001:db8::1".parse().unwrap(), meta: 29 } ;
    dp.ttl_to_start_value();
    assert_eq!(dp.meta, 32);
}

#[test]
fn ttl_to_path_length() {
    let mut dp = super::DataPoint { ip6: "2001:db8::1".parse().unwrap(), meta: 111 } ;
    dp.ttl_to_path_length();
    assert_eq!(dp.meta, 17);

    let mut dp = super::DataPoint { ip6: "2001:db8::1".parse().unwrap(), meta: 59 } ;
    dp.ttl_to_path_length();
    assert_eq!(dp.meta, 5);

    let mut dp = super::DataPoint { ip6: "2001:db8::1".parse().unwrap(), meta: 29 } ;
    dp.ttl_to_path_length();
    assert_eq!(dp.meta, 35);
}
#[test]
fn test_var() {
    let s: Vec<u32> = vec![];
    assert_eq!(var(&s), 0.0);

    let s: Vec<u32> = vec![1];
    assert_eq!(var(&s), 0.0);

    let s: Vec<u32> = vec![1,2,3];
    assert_eq!(var(&s), 1.0);

    let s: Vec<u32> = vec![10,20,30];
    assert_eq!(var(&s), 100.0);

    let s: Vec<u32> = vec![10,10,10,10,10,10,10,11];
    assert_eq!(var(&s), 0.125);
}

#[test]
fn test_median() {
    let s: Vec<u32> = vec![];
    assert_eq!(median(&s), 0.0);

    let s: Vec<u32> = vec![1];
    assert_eq!(median(&s), 1.0);

    let s: Vec<u32> = vec![0, 1];
    assert_eq!(median(&s), 0.5);

    let s: Vec<u32> = vec![0, 1, 2];
    assert_eq!(median(&s), 1.0);

    let s: Vec<u32> = vec![9, 9, 8, 3, 1];
    assert_eq!(median(&s), 8.0);

    let s: Vec<u32> = vec![9, 8, 3, 1];
    assert_eq!(median(&s), 5.5);
}
