use plot;

use ipnetwork::Ipv6Network;
use std::net::Ipv6Addr;

use treebitmap::{IpLookupTable};

use svg::Node;
use svg::node::element::Rectangle;

use std::collections::{HashMap, HashSet};
use clap::ArgMatches;

use std::cmp::Ordering;
use std::iter;
use std::f64;

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

#[derive(Debug)]
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

#[derive(Debug)]
pub enum DpFunction {
    Mean,
    Median,
    Var,
    Uniq,
    Sum,
}
    

#[derive(Debug)]
pub struct PlotParams {
    pub sized: bool,
    pub bit_size_factor: f64,  // default 2.0, so a /48 is twice the size of a /49
    pub legend_label: String,
    pub show_legend: bool,
    pub colour_scale: plot::ColourScale,
    pub filter_threshold: u64,
    pub dp_function: Option<DpFunction>,
    // asn_colours? or make that a type of ColourScale?
}

impl PlotParams {
    pub fn new(table: &IpLookupTable<Ipv6Addr,Specific>, matches: &ArgMatches) -> PlotParams {
        let sized = !matches.is_present("unsized-rectangles");
        let bit_size_factor = value_t!(matches.value_of("bit-size-factor"), f64) .unwrap_or_else(|_| 2.0_f64);

        // nothing passed? -> hits , no dp-function

        // other colour is triggered by --csv with a second column
        // only then a DpFunction should be active
        // (or do we want uniq(addresses) as well?) -> that's more like the hamming weight thing
        // (also, we still have iTTL functions, DNS RA bit extraction..)
        // DpFunctions: mean, median, var, uniq, sum 
        // values: ttl, mss, --csv

        let mut colour_metric = "hits"; 

        //FIXME we already parse --csv in read_datapoints_from_file ..
        if matches.is_present("csv-columns"){
            let csv_columns: Vec<&str> = matches.value_of("csv-columns").unwrap().split(',').collect();
            if csv_columns.len() > 1 {
                if matches.is_present("dp-function"){
                    colour_metric = csv_columns[1];
                } else {
                    warn!("No --dp-function passed, ignoring second column '{}' in --csv", csv_columns[1]);
                }
            }
        }


        // _if_ there is a second CSV column passed, there MUST be a dp-function.
        // default to DpMean

        let dp_function = if matches.is_present("dp-function"){
            match matches.value_of("dp-function").unwrap() {
                "mean"      => Some(DpFunction::Mean),
                "median"    => Some(DpFunction::Median),
                "var"       => Some(DpFunction::Var),
                "uniq"      => Some(DpFunction::Uniq),
                "sum"       => Some(DpFunction::Sum),
                _           => { warn!("unknown dp-function passed, using 'mean'"); Some(DpFunction::Mean) },
            }
        } else {
            None
        };

        let legend_label = if matches.is_present("legend-label") {
            matches.value_of("legend-label").unwrap().to_string()
        } else if dp_function.is_some() {
            match dp_function {
                Some(DpFunction::Mean)   =>   format!("mean({})", colour_metric),
                Some(DpFunction::Median) =>   format!("median({})", colour_metric),
                Some(DpFunction::Var)    =>   format!("var({})", colour_metric),
                Some(DpFunction::Uniq)   =>   format!("uniq({})", colour_metric),
                Some(DpFunction::Sum)    =>   format!("sum({})", colour_metric),
                _  => { warn!("unknown dp-function when constructing legend label"); "FIXME".to_string() },
            }
        } else {
            colour_metric.to_string()
        };

        let show_legend = !matches.is_present("hide-legend"); //TODO implement in clap

        // FIXME if we do not filter, make sure filter_threshold in PlotParams is 0
        // otherwise things just get confusing
        // so let --filter be an alias for --filter-threshold 1,
        // and check on the value of ft instead of the boolean 'filter'
        let filter_threshold = value_t!(matches.value_of("filter-threshold"), u64).unwrap_or_else(|_| 1);

        // determine min/max/medium for either hits or dp-function
        // TODO remove this, we update it after filtering anyway
        let mut meta_dps: Vec<f64>  = match dp_function {
            Some(DpFunction::Mean)      => table.iter().map(|(_,_,s)| s.dp_mean()).collect(),
            Some(DpFunction::Median)    => table.iter().map(|(_,_,s)| s.dp_median()).collect(),
            Some(DpFunction::Var)       => table.iter().map(|(_,_,s)| s.dp_var()).collect(),
            Some(DpFunction::Uniq)      => table.iter().map(|(_,_,s)| s.dp_uniq()).collect(),
            Some(DpFunction::Sum)       => table.iter().map(|(_,_,s)| s.dp_sum()).collect(),
            None                        => table.iter().map(|(_,_,s)| s.datapoints.len() as f64).collect(),
        };

        meta_dps.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Less));
        let (min, median, max) = (meta_dps[0], meta_dps[meta_dps.len()/2], meta_dps[meta_dps.len()-1]);
            
        let colour_scale = plot::ColourScale::new(min, median, max);

        PlotParams {
            sized,
            bit_size_factor,
            legend_label,
            show_legend,
            colour_scale,
            filter_threshold,
            dp_function,
            }

    }

    pub fn update_colour_scale(&mut self, specifics: &[Specific]) {
        let dp_fn: fn(&Specific) -> f64 = match self.dp_function {
            Some(DpFunction::Mean)      => Specific::dp_mean,
            Some(DpFunction::Median)    => Specific::dp_median,
            Some(DpFunction::Var)       => Specific::dp_var,
            Some(DpFunction::Uniq)      => Specific::dp_uniq,
            Some(DpFunction::Sum)       => Specific::dp_sum,
            None                        => Specific::hits2,
        };
        // specifics could be nested, so iterate recursively using deep_iter()
        let mut meta_dps: Vec<f64>  = specifics.iter()
            .flat_map(|s| s.deep_iter())
            .map(dp_fn)
            .collect()
            ;

        // if we have no datapoints, return gracefully
        if meta_dps.is_empty() {
            self.colour_scale = plot::ColourScale::new(0.0, 0.0, 0.0);
            return
        }

        // filter out NaNs and 0: they will be plotted grey anyway,
        // so do not let them influence the colour scale..
        meta_dps.retain(|f| !f.is_nan() && *f > 0.0);
        meta_dps.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Less));

        let (min, max) = (meta_dps[0], meta_dps[meta_dps.len()-1]);
        let median = if meta_dps.len() % 2 == 0 {
            (meta_dps[meta_dps.len()/2] + meta_dps[meta_dps.len()/2 - 1]) / 2.0
        } else {
            meta_dps[meta_dps.len()/2]
        };

        self.colour_scale = plot::ColourScale::new(min, median, max);
    }


    // or do this by passing &plot_params to output::construct_fn ?
    //pub fn to_filename(&self) -> String {
    //    let mut filename = "".to_string();
    //    filename.push_str("asd");
    //    filename
    //}
}


impl Specific {
    pub fn push_dp(&mut self, dp: super::DataPoint) -> () {
        self.datapoints.push(dp);
    }

    // Datapoint / Stat functions

    pub fn dp_mean(&self) -> f64 {
        self.dp_sum() / self.datapoints.len() as f64
    }

    pub fn dp_var(&self) -> f64 {
        if self.datapoints.len() < 2 {
            return f64::NAN;
        }
        let mean = self.dp_mean();
        self.datapoints.iter().map(|dp| dp.meta).fold(0.0, |var, dp|
            var as f64 + (f64::from(dp) - mean).powf(2.0) ) / (self.datapoints.len() -1) as f64
    }

    pub fn dp_median(&self) -> f64 {
        if self.datapoints.is_empty() {
            return f64::NAN;
        }
        let mut sorted = self.datapoints.iter().map(|dp| dp.meta).collect::<Vec<u32>>();
        sorted.sort();
        if sorted.len() % 2  == 0 {
            ((f64::from(sorted[sorted.len() / 2]) + f64::from(sorted[sorted.len() / 2 - 1])) / 2.0) as f64
        } else {
            f64::from(sorted[sorted.len() / 2])
        }
    }

    pub fn dp_uniq(&self) -> f64 {
        let mut uniq_meta: HashSet<u32> = HashSet::new();
        for dp in &self.datapoints {
            uniq_meta.insert(dp.meta);
        }
        uniq_meta.len() as f64
    }

    pub fn dp_sum(&self) -> f64 {
        f64::from(self.datapoints.iter().map(|e| e.meta).sum::<u32>())
    }


    // Other functions

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

    // to iterate recursively over self+children:
    pub fn deep_iter(&self) -> impl Iterator<Item =&'_ Specific> {
        iter::once(self).chain(self.iter_specs())
    }
    fn iter_specs<'a>(&'a self) -> Box<Iterator<Item=&'a Specific> + 'a> {
        // Box needed: https://github.com/rust-lang/rust/issues/39555
        Box::new(self.specifics.iter().flat_map(|s| s.deep_iter()))
    }

    // TODO: use deep_iter ?
    // TODO: get rid of usize
    pub fn all_hits(&self) -> usize {
        self.hits() + self.hits_in_specifics()
    }

    pub fn hits(&self) -> usize {
        self.datapoints.len()
    }

    pub fn hits2(&self) -> f64 {
        self.datapoints.len() as f64
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

    #[allow(many_single_char_names)]
    pub fn to_rect(&self, t: Turtle, w_factor: f64, h_factor: f64, plot_params: &PlotParams) -> Rectangle {
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
            //.set("data-dp-desc", plot_info.dp_desc.clone())
            .set("data-dp-desc", plot_params.legend_label.clone())
            //TODO: only set these attributes if actual meta data was provided for input
            // i.e. if there was a second CSV column
            .set("data-dp-mean", format!("{:.1}", self.dp_mean()))
            .set("data-dp-median", format!("{:.1}", self.dp_median()))
            .set("data-dp-var", format!("{:.1}", self.dp_var()))
            .set("data-dp-uniq", format!("{:.1}", self.dp_uniq()))
            .set("data-dp-sum", format!("{:.1}", self.dp_sum()))
            .set("data-hw-avg", format!("{:.1}", self.hw_avg()))
            ;

        let dp_fn: fn(&Specific) -> f64 = match plot_params.dp_function {
            Some(DpFunction::Mean)      => Specific::dp_mean,
            Some(DpFunction::Median)    => Specific::dp_median,
            Some(DpFunction::Var)       => Specific::dp_var,
            Some(DpFunction::Uniq)      => Specific::dp_uniq,
            Some(DpFunction::Sum)       => Specific::dp_sum,
            None                        => Specific::hits2,
        };
        let (h,s,l) = plot_params.colour_scale.get(dp_fn(&self));
        r.assign("fill", format!("hsl({}, {}%, {}%)", h, s, l));
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

    pub fn rects_in_specifics(&self, t: Turtle, w_factor: f64, h_factor: f64, plot_params: &PlotParams) -> Vec<Rectangle> {
        if self.specifics.is_empty() {
            return vec![]
        }
        let Turtle {x, y, w, h} = t;
        let w_factor  = w_factor / self.specifics.len() as f64;
        let mut results = Vec::new();
        let mut x = x;
        for s in &self.specifics {
            results.push(s.to_rect(Turtle{x, y, w, h}, w_factor, h_factor, plot_params));
            let sub_w_factor  = w_factor; 
            results.append(&mut s.rects_in_specifics(Turtle{x, y, w, h}, sub_w_factor, h_factor / 2.0, plot_params));
            x += w * w_factor; 
        }

    results
    }

    pub fn all_rects(&self, area: &Area, plot_params: &PlotParams) -> Vec<Rectangle> {
        let t = Turtle {x: area.x, y: area.y, w: area.w, h: area.h};
        let mut result = vec![self.to_rect(t, 1.0, 1.0, plot_params)];
        result.append(&mut self.rects_in_specifics(t, 1.0, 0.5, plot_params));
        result
    }
}


fn specs_to_hier_with_rest_index(specifics: &[Specific], index: usize) -> (Vec<Specific>, usize) {
    let current_specific: &Specific;
    if let Some((first, rest)) = specifics[index..].split_first() {
        current_specific = first;
        if rest.is_empty() {
            return (vec![first.clone()], 1);
        }

        let mut nested_specs: Vec<Specific> = Vec::new();
        let mut consumed_specs = 1;
        for s in rest.iter() {
            if current_specific.network.contains(s.network.ip()) {
                nested_specs.push(s.clone());
                consumed_specs += 1;
            } else {
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

pub fn specs_to_hier(specifics: &[Specific]) -> Vec<Specific> {
    let mut done = false;
    let mut all_results: Vec<Specific> = vec![];
    let mut start_from = 0;
    
    if specifics.is_empty() {
        return vec![];
    }
    
    if specifics.len() == 1 {
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


pub fn areas_to_rows(mut areas: Vec<Area>) -> Vec<Row> {
    let mut rows = Vec::new();
    if areas.is_empty() {
        error!("Nothing to plot. Did you provide an empty/invalid addresses file while filtering out empty prefixes?");
        return rows;
    }
    let remaining_areas = areas.split_off(1);   
                                               
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

    rows
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
mod tests {
    use super::*;

    fn gen_specifics() -> Vec<Specific> {
        assert!(false);
        vec![
            ]
    }

    fn gen_dps() -> Vec<DataPoint> {
        (1..=10).map(|m|
            DataPoint { ip6: "2001:db8::1".parse().unwrap(), meta:  m as u32 },
        ).collect()
    }
    fn gen_dps2() -> Vec<DataPoint> {
        vec![1,1,1,1,1,1,1,2,3,10].into_iter().map(|m|
            DataPoint { ip6: "2001:db8::1".parse().unwrap(), meta:  m as u32 },
        ).collect()
    }

    fn gen_specific() -> Specific {
        Specific {
            network: "2001:db8::/32".parse::<Ipv6Network>().unwrap(),
            asn: "TEST".to_string(),
            datapoints: gen_dps(),
            specifics: vec![],
        }
    }
    fn gen_specific2() -> Specific {
        Specific {
            network: "2001:db8::/32".parse::<Ipv6Network>().unwrap(),
            asn: "TEST".to_string(),
            datapoints: gen_dps2(),
            specifics: vec![],
        }
    }
    fn gen_specific_no_dp() -> Specific {
        Specific {
            network: "2001:db8::/32".parse::<Ipv6Network>().unwrap(),
            asn: "TEST".to_string(),
            datapoints: vec![],
            specifics: vec![],
        }
    }

    #[test]
    fn dp_mean() {
        assert!(gen_specific_no_dp().dp_mean().is_nan());
        assert_eq!(5.5, gen_specific().dp_mean());
        assert_eq!(2.2, gen_specific2().dp_mean());
    }

    #[test]
    fn dp_median() {
        assert!(gen_specific_no_dp().dp_median().is_nan());
        assert_eq!(5.5, gen_specific().dp_median());
        assert_eq!(1.0, gen_specific2().dp_median());
    }

    #[test]
    fn dp_var() {
        assert!(gen_specific_no_dp().dp_var().is_nan());
        assert_eq!(9.1667, (gen_specific().dp_var() * 10_000.0).round() / 10_000.0);
        assert_eq!(7.9556, (gen_specific2().dp_var() * 10_000.0).round() / 10_000.0);
    }

    #[test]
    fn dp_uniq() {
        assert_eq!(0.0,  gen_specific_no_dp().dp_uniq());
        assert_eq!(10.0, gen_specific().dp_uniq());
        assert_eq!(4.0, gen_specific2().dp_uniq());
    }

    #[test]
    fn dp_sum() {
        assert_eq!(0.0,  gen_specific_no_dp().dp_sum());
        assert_eq!(55.0, gen_specific().dp_sum());
        assert_eq!(22.0, gen_specific2().dp_sum());
    }


    // ---------------------------


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
}
