use ipnetwork::Ipv6Network;
use std::net::Ipv6Addr;

//pub struct Prefix {
//    pub network: Ipv6Network,
//    pub asn: String,
//    pub specifics: Vec<Specific>
//}

#[derive(Debug, Clone)]
pub struct Specific {
    pub network: Ipv6Network,
    pub asn: String,
    pub datapoints: Vec<super::DataPoint>,
    pub specifics: Vec<Specific>
}

impl Specific {
    pub fn push_dp(&mut self, dp: super::DataPoint) -> () {
        self.datapoints.push(dp);
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
        if exp > 64 {
            exp = 64;
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

    pub fn to_rect(&self, x: f64, y: f64, w: f64, h: f64, w_factor: f64, h_factor: f64, max_hits: usize) -> super::Rectangle {
        super::Rectangle::new()
            .set("x", x)
            .set("y", y)
            .set("width", w * w_factor)
            .set("height", h * h_factor)
            //.set("fill", "white")
            .set("fill", super::color2(self.hits() as u32, max_hits as u32)) 
            //.set("fill", color(area.route.hw_avg() as u32, max_hamming_weight as u32)) 
            //.set("fill", color(area.route.dp_avg() as u32, max_meta as u32)) 
            .set("stroke-width", w * h * 0.0005 * h_factor)
            .set("stroke", "#aaaaaa")
            .set("opacity", 1.0)
            .set("data-asn", self.asn.to_string())
            .set("data-prefix", self.network.to_string())
            .set("data-hits", self.all_hits())
            //.set("data-dp-avg", format!("{:.1}", area.route.dp_avg()))
            //.set("data-hw-avg", format!("{:.1}", area.route.hw_avg()))
    }

    //pub fn rects_in_specifics(&self, area: &Area, w_factor: f64) -> Vec<super::Rectangle> {
    pub fn rects_in_specifics(&self, x: f64, y: f64, w: f64, h: f64, w_factor: f64, h_factor: f64, max_hits: usize) -> Vec<super::Rectangle> {
        if self.specifics.len() == 0 {
            return vec![]
        }
        let w_factor  = w_factor / self.specifics.len() as f64;
        let mut results = Vec::new();
        //let mut sub_area_x = area.x;
        //let mut sub_area_y = area.y;
        let mut x = x;
        for s in &self.specifics {
            results.push(s.to_rect(x, y, w, h, w_factor, h_factor, max_hits));
            let sub_w_factor  = w_factor; // / 1.0 / s.specifics.len() as f64;
            results.append(&mut s.rects_in_specifics(x, y, w, h, sub_w_factor, h_factor / 2.0, max_hits));
            x += w * w_factor; // * self.specifics.len() as f64;

            //println!("parent size: {}", s.size(false) as f64 );
            //println!("sub size: {}", s.size(false) as f64 / area.specific.size(false) as f64);
            ////let sub_width = 2.0 * area.w * (s.size(false) as f64 / area.specific.size(false) as f64 ) ;
            //let sub_width = (area.w / area.specific.specifics.len() as f64);
            //let mut rect = super::Rectangle::new()
            //    .set("x", sub_area_x)
            //    .set("y", sub_area_y)
            //    .set("width", sub_width)
            //    .set("height", area.h / 2.0 )
            //    //.set("stroke-width", border)
            //    .set("stroke-width", 0.1)
            //    .set("stroke", "black")
            //    .set("opacity", 1.0)
            //    .set("fill", "white")
            //    ;
            //    //group.append(rect);
            //    results.push(rect);
            //sub_area_x += sub_width;
            //sub_area_y += 0.0 ;
        }

    results
    }

    pub fn all_rects(&self, area: &Area, max_hits: usize) -> Vec<super::Rectangle> {
        //let mut result = self.rects_in_specifics(area, 1.0);
        //result.push(self.to_rect(area, 1.0));
        //result
        let mut result = vec![self.to_rect(area.x, area.y, area.w, area.h, 1.0, 1.0, max_hits)];
        result.append(&mut self.rects_in_specifics(area.x, area.y, area.w, area.h, 1.0, 0.5, max_hits));
        result
    }
}

/*
pub fn specs_to_hier(specifics: &Vec<Specific>) -> Vec<Specific> {
    println!("in specs_to_hier");
    let mut current_specific: &Specific;
    if let Some((first, rest)) = specifics.split_first() {
        println!("first: {:?}", first.network);
        current_specific = first;
        let mut nested_specs: Vec<Specific> = Vec::new();
        let mut remaining_specs: Vec<Specific> = Vec::new();
        for (i, s) in rest.iter().enumerate() {
            if current_specific.network.contains(s.network.ip()) {
                println!("nested s: {:?}", s.network);
                nested_specs.push(s.clone());
            } else {
                println!("creating remaining_specs with {:?}", s.network);
                current_specific = s;
                remaining_specs = rest[i..].to_vec();
                println!("  post remaining_specs");
                break;
            }

        }
        println!(" -- nested_specs, current_specific: {:?} --", first.network);

        // trying add:
        let mut result = vec![Specific { network: first.network, datapoints: Vec::new(), specifics: specs_to_hier(&nested_specs) }];
        let mut result_remaining = specs_to_hier(&remaining_specs);
        result.append(&mut result_remaining); //?
        result

        // attempt 2:
        //let mut resulting_specifics = specs_to_hier(&nested_specs);
        //resulting_specifics.append(&mut remaining_specs);
        //let result = vec![Specific { network: first.network, specifics: resulting_specifics }];
        //result

    } else {
        println!("could not satisfy Some(), len of specifics: {}", specifics.len());
        // so, there is only one? specific left in the vector
        // not necessarily, check
        //return specifics.first().unwrap() // are we sure this is there?
        //vec![specifics.first().unwrap().clone()]
        vec![]
    }
}
*/

/*
pub fn specs_to_hier_with_rest(specifics: &Vec<Specific>) -> (Vec<Specific>, Vec<Specific>) {
    let mut current_specific: &Specific;
    if let Some((first, rest)) = specifics.split_first() {
        current_specific = first;
        let mut nested_specs: Vec<Specific> = Vec::new();
        let mut remaining_specs: Vec<Specific> = Vec::new();
        for (i, s) in rest.iter().enumerate() {
            if current_specific.network.contains(s.network.ip()) {
                nested_specs.push(s.clone());
            } else {
                //current_specific = s;
                remaining_specs = rest[i..].to_vec();
                break;
            }

        }

        let result = vec![Specific { network: first.network, datapoints: Vec::new(), specifics: specs_to_hier(&nested_specs) }];
        return (result, remaining_specs)
    }
    (vec![], vec![])
}
*/

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

        //println!("result for network {:?}, {} nested, {} consumed", first.network, nested_specs.len(), consumed_specs);
        //let result = if true || nested_specs.len() > 1 {
        //    vec![Specific { network: first.network, datapoints: first.datapoints.clone(),
        //        specifics: specs_to_hier2(&nested_specs) }]
        //    } else {
        //        vec![Specific { network: first.network, datapoints: first.datapoints.clone(),
        //            specifics: nested_specs }]
        //    };
        let result = vec![Specific { network: first.network, asn: first.asn.clone(), datapoints: first.datapoints.clone(),
                specifics: specs_to_hier2(&nested_specs) }];
        return (result, consumed_specs)
    } else {
        println!("could not satisfy Some(), len: {}", specifics.len());
    }
    println!("returning empty vector..");
    (vec![], 0)
}

pub fn specs_to_hier2(specifics: &Vec<Specific>) -> Vec<Specific> {
    //println!("specs_to_hier2, specifics.len(): {} ", specifics.len());
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

//pub fn __route_to_specifics2(routes: &Vec<Route>) -> Vec<Specific> {
//    specs_to_hier2(&routes.iter().map(|r| Specific {network: r.prefix, datapoints: r.datapoints.clone(), specifics: Vec::new() } ).collect())
//}

//pub fn __route_to_specifics(routes: &Vec<Route>) -> Vec<Specific> {
//    //specs_to_hier(&routes.iter().map(|r| Specific {network: r.prefix, datapoints: r.datapoints.clone(), specifics: Vec::new() } ).collect())
//    //specs_to_hier(&routes.iter().map(|r| Specific {network: r.prefix, datapoints: Vec::new(), specifics: Vec::new() } ).collect())
//    let mut specs: Vec<Specific> = Vec::new();
//    for r in routes {
//        let s = Specific {network: r.prefix, datapoints: r.datapoints.clone(), specifics: Vec::new() } ;
//        //let s = Specific {network: r.prefix, datapoints: Vec::new(), specifics: Vec::new() } ;
//        specs.push(s);
//    }
//    specs_to_hier2(&specs)
//}

/*
pub fn __vec_to_hier(routes: &Vec<Route>) -> Vec<Specific> {
    if routes.len() == 1 {
        //Some( Prefix { network: routes.first().unwrap().prefix, asn: routes.first().unwrap().asn, specifics: Vec::new() } );
    } else {
        let mut current_route: &Route;
        if let Some((first, rest)) = routes.split_first() {
            println!("first: {:?}", first.prefix);
            current_route = first;
            let mut specifics: Vec<Specific> = Vec::new();
            for r in rest {
                if current_route.prefix.contains(r.prefix.ip()) {
                    // more specific
                    println!("more specific: {:?}", r.prefix);
                    specifics.push(Specific {network: r.prefix, specifics: Vec::new()});
                } else {
                    // new prefix
                    println!("new prefix: {:?}", r.prefix);

                    // process specifics of previous first
                    //for s in &specifics {
                    //    println!("s: {:?}", s.network);
                    //}
                    println!("calling specs_to_hier");
                    return specs_to_hier(&specifics);

                    //continue
                    //vec_to_hier(&rest.to_vec());
                }
            }
       
    }
}
    println!("vec_to_hier, returning empty vec");
    vec![]
}
*/

pub struct Area {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub surface: f64,
    //pub route: Route,
    pub specific: Specific,
}

//pub struct Area2 {
//    pub x: f64,
//    pub y: f64,
//    pub w: f64,
//    pub h: f64,
//    pub surface: f64,
//    pub specific: Specific,
//}

pub struct Row {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub vertical: bool,
    pub areas: Vec<Area>,
}

pub struct Route {
    pub prefix: Ipv6Network,
    //pub asn:    u32,
    pub asn:    String,
    pub datapoints: Vec<super::DataPoint>,

}

impl Route {
    pub fn size(&self, unsized_rectangles: bool) -> u128 {
        if unsized_rectangles {
            return 1u128
        } else {
            self.__size()
        }
    }

    pub fn __size(&self) -> u128 {
        //FIXME: this is just a workaround.. is there a better way to do this?
        let mut exp = self.prefix.prefix() as u32;
        if exp < 24 {
            exp = 24;
        }
        if exp > 64 {
            exp = 64;
        }
        //exp = 64; // TODO this is just to get equal sized squares
        let r = 2_u128.pow(128 - exp);
        r
    }

    pub fn _size(&self) -> u128 {
        128 - self.prefix_len() as u128
    }

    pub fn to_string(&self) -> String {
        format!("AS{}", &self.asn)
    }

    pub fn push_dp(&mut self, dp: super::DataPoint) -> () {
        self.datapoints.push(dp);
    }
    pub fn prefix_len(&self) -> u8  {
        self.prefix.prefix()
    }

    pub fn dp_avg(&self) -> f64 {
        let sum = self.datapoints.iter().fold(0, |s, i| s + i.meta);
        sum as f64 / self.datapoints.len() as f64
    }

    pub fn hw_avg(&self) -> f64 {
        let sum = self.datapoints.iter().fold(0, |s, i| s + i.hamming_weight(self.prefix_len()));
        sum as f64 / self.datapoints.len() as f64
    }
}

impl Area {
    //pub fn new(surface: f64, ratio: f64, route: Route) -> Area {
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
