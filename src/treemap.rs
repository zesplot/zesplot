use ipnetwork::Ipv6Network;
use std::net::Ipv6Addr;

use svg::Node;

use std::collections::{HashMap, HashSet};
use std::iter::FromIterator;

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

impl DataPoint {
    fn hamming_weight(&self, prefix_len: u8) -> u32 {
        (u128::from(self.ip6) << prefix_len  >> prefix_len).count_ones()
    }
    fn hamming_weight_iid(&self) -> u32 {
        self.hamming_weight(64)
    }
}

pub enum ColourMode {
    Hits,
    DpAvg,
    DpVar,
    DpUniq,
    DpSum,
    Asn
}
    

pub struct PlotInfo<'a> {
    pub max_hits: usize,
    //pub max_dp: f64,
    pub max_dp_avg: f64,
    pub max_dp_var: f64,
    pub max_dp_uniq: usize,
    pub max_dp_sum: usize,
    pub colour_mode: ColourMode,
    pub dp_desc: String,
    pub asn_colours: &'a HashMap<u32, String>
}

impl Specific {
    pub fn push_dp(&mut self, dp: super::DataPoint) -> () {
        self.datapoints.push(dp);
    }

    pub fn dp_avg(&self) -> f64 {
        let sum = self.datapoints.iter().fold(0, |s, i| s + i.meta);
        sum as f64 / self.datapoints.len() as f64
    }
// var = ((Array[n] - mean) * (Array[n] - mean)) / numPoints;
    pub fn dp_var(&self) -> f64 {
        let sum = self.datapoints.iter().fold(0, |sum, dp| sum + dp.meta);
        let mean = sum as f64 / self.datapoints.len() as f64;
        let var = self.datapoints.iter().fold(0.0, |var, dp| var as f64 + (dp.meta as f64 - mean).powf(2.0) ) / self.datapoints.len() as f64;
        var
    }

    pub fn dp_uniq(&self) -> usize {
        let mut uniq_meta: HashSet<u32> = HashSet::new();
        for dp in &self.datapoints {
            uniq_meta.insert(dp.meta);
        }
        let hash_len = uniq_meta.len();
        //TODO: why is this different? (not essential for zesplot)
        //let mut tmp = self.datapoints.iter().map(|dp| dp.meta).collect::<Vec<u32>>();
        //tmp.dedup();
        //tmp.shrink_to_fit();
        //tmp.len() // as usize
        hash_len

    }

    pub fn dp_sum(&self) -> usize {
        self.datapoints.iter().fold(0, |s, i| s + i.meta as usize)
    }
    
    pub fn all_hits(&self) -> usize {
        &self.hits() + &self.hits_in_specifics()
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
            return 1u128
        } else {
            self.__size()
        }
    }

    pub fn __size(&self) -> u128 {
        //(128 - self.network.prefix()).into()
        let mut exp = self.network.prefix() as u32;
        if exp < 24 {
            exp = 24;
        }
        if exp > 48 {
            exp = 48;
        }
        let r = 2_u128.pow(128 - exp);
        r
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

    pub fn to_rect(&self, x: f64, y: f64, w: f64, h: f64, w_factor: f64, h_factor: f64, plot_info: &PlotInfo) -> super::Rectangle {
        let mut r = super::Rectangle::new()
            .set("x", x)
            .set("y", y)
            .set("width", w * w_factor)
            .set("height", h * h_factor)
            .set("stroke-width", 0.01_f64.max(w * h * 0.0005 * h_factor))
            .set("stroke", "#aaaaaa")
            .set("opacity", 1.0)
            .set("data-asn", self.asn.to_string())
            .set("data-prefix", self.network.to_string())
            .set("data-hits", self.all_hits())
            .set("data-dp-desc", plot_info.dp_desc.clone())
            .set("data-dp-avg", format!("{:.1}", self.dp_avg()))
            .set("data-dp-var", format!("{:.1}", self.dp_var()))
            .set("data-dp-uniq", format!("{:.1}", self.dp_uniq()))
            .set("data-dp-sum", format!("{:.1}", self.dp_sum()))
            ;

        match plot_info.colour_mode {
            ColourMode::Hits => r.assign("fill", colour(self.hits() as u32, plot_info.max_hits as u32)),
            ColourMode::DpAvg => r.assign("fill", colour(self.dp_avg() as u32, plot_info.max_dp_avg as u32)),
            ColourMode::DpVar => r.assign("fill", colour(self.dp_var() as u32, plot_info.max_dp_var as u32)),
            ColourMode::DpUniq => r.assign("fill", colour(self.dp_uniq() as u32, plot_info.max_dp_uniq as u32)),
            ColourMode::DpSum => r.assign("fill", colour(self.dp_sum() as u32, plot_info.max_dp_sum as u32)),
            ColourMode::Asn => r.assign("fill", colour_from_map(self.asn(), plot_info.asn_colours))
        }
        r
            //.set("data-hw-avg", format!("{:.1}", area.route.hw_avg()))
    }

    pub fn rects_in_specifics(&self, x: f64, y: f64, w: f64, h: f64, w_factor: f64, h_factor: f64, plot_info: &PlotInfo) -> Vec<super::Rectangle> {
        if self.specifics.len() == 0 {
            return vec![]
        }
        let w_factor  = w_factor / self.specifics.len() as f64;
        let mut results = Vec::new();
        let mut x = x;
        for s in &self.specifics {
            results.push(s.to_rect(x, y, w, h, w_factor, h_factor, plot_info));
            let sub_w_factor  = w_factor; // / 1.0 / s.specifics.len() as f64;
            results.append(&mut s.rects_in_specifics(x, y, w, h, sub_w_factor, h_factor / 2.0, plot_info));
            x += w * w_factor; // * self.specifics.len() as f64;
        }

    results
    }

    pub fn all_rects(&self, area: &Area, plot_info: &PlotInfo) -> Vec<super::Rectangle> {
        //let mut result = self.rects_in_specifics(area, 1.0);
        //result.push(self.to_rect(area, 1.0));
        //result
        let mut result = vec![self.to_rect(area.x, area.y, area.w, area.h, 1.0, 1.0, plot_info)];
        result.append(&mut self.rects_in_specifics(area.x, area.y, area.w, area.h, 1.0, 0.5, plot_info));
        result
    }
}


pub fn specs_to_hier_with_rest_index(specifics: &Vec<Specific>, index: usize) -> (Vec<Specific>, usize) {
    //println!("specs_rest_index, len {} start from {}", specifics.len(), index);
    let current_specific: &Specific;
    if let Some((first, rest)) = specifics[index..].split_first() {
        current_specific = first;
        if rest.len() == 0 {
            //println!("NO REST, returning {}", first.network.ip());
            return (vec![first.clone()], 1);
        }

        //println!("first : {:?}", first.network);
        let mut nested_specs: Vec<Specific> = Vec::new();
        let mut consumed_specs = 1;
        //for (_, s) in rest.iter().enumerate() {
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

pub fn specs_to_hier(specifics: &Vec<Specific>) -> Vec<Specific> {
    let mut done = false;
    let mut all_results: Vec<Specific> = vec![];
    let mut start_from = 0;
    
    if specifics.len() == 0 {
        //println!("early done 0");
        return vec![];
    }
    
    if specifics.len() == 1 {
        //println!("early done 1 for {:?}", specifics.first().unwrap());
        return specifics.clone();
    }
    
    while !done {
        let (mut result, num_consumed) = specs_to_hier_with_rest_index(&specifics, start_from);
        if result.len() == 0 && num_consumed  == 0 {
            done = true;
        }
        start_from += num_consumed;
        if specifics.len() == start_from + 0 {
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
        if &self.h >= &self.w {
            &self.w  / &self.h
        } else {
            &self.h / &self.w
        }
    }
}


impl Row {
    pub fn new(x: f64, y: f64, vertical: bool, mut area: Area) -> Row {
        let max_h = super::HEIGHT - y;
        let max_w = super::WIDTH - x;
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
        &self.push(area);

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

        //if area.specific.specifics.len() > 0 {
        //    println!("got specifics in this area");
        //    for s in &area.specific.specifics {
        //        println!("got {} size {}", s.network, s.prefix_len());
        //        let norm_factor = 1_f64;
        //        let sub_area = Area::new(s.size(true) as f64 * norm_factor, 1_f64, s.clone()  );
        //        &self.areas.push(sub_area);
        //    }
        //}

        &self.areas.push(area);
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

    let mut scale: Vec<u32> = (0..0xff+1).map(|e| 0xff | (e << 8)).collect();
    scale.append(&mut (0..0xff+1).rev().map(|e| (0xff << 8) | e).collect::<Vec<u32>>() );
    scale.append(&mut (0..0xff+1).map(|e| 0xff00 | (e << 16) | e).collect::<Vec<u32>>() );
    scale.append(&mut (0..0xff+1).rev().map(|e| 0xff0000 | (e << 8)).collect::<Vec<u32>>() );

    if max > 1024 {
        let norm = scale.len() as f64 / (max as f64).log2();
        let mut index = ((i as f64).log2() * norm) as usize;
        //FIXME: this should not be necessary..
        if index >= scale.len() {
            index = scale.len() - 1;
        }
        if index == 0 {
            index = 1;
        }
        format!("#{:06x}", scale.get(index).unwrap())
    } else {
        let norm = scale.len() as f64 / (max as f64);
        let mut index = ((i as f64) * norm) as usize;
        //FIXME: this should not be necessary..
        if index >= scale.len() {
            index = scale.len() - 1;
        }
        if index == 0 {
            index = 1;
        }
        format!("#{:06x}", scale.get(index).unwrap())
    }
}

// used for asn -> id mapping
fn colour_from_map(asn: u32, mapping: &HashMap<u32, String>) -> String {

    if !mapping.contains_key(&asn) {
        return "#eeeeee".to_string();
    }

    let uniq_values: HashSet<String> = HashSet::from_iter(mapping.values().cloned());
    let mut uniq_sorted_values: Vec<String> = uniq_values.into_iter().collect();
    uniq_sorted_values.sort();
    let num_distinct_colours = uniq_sorted_values.len();

    let index = uniq_sorted_values.iter().position(|e| e == mapping.get(&asn).unwrap()).unwrap();
    colour(index as u32, num_distinct_colours as u32)
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


