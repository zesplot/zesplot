use ipnetwork::Ipv6Network;
use std::net::Ipv6Addr;

pub struct Prefix {
    pub network: Ipv6Network,
    pub asn: String,
    pub specifics: Vec<Specific>
}

#[derive(Debug, Clone)]
pub struct Specific {
    pub network: Ipv6Network,
    pub specifics: Vec<Specific>
}


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
                //println!("new sub in s: {:?}", s.network);
                //println!(" -- nested_specs --");
                //specs_to_hier(&nested_specs);
                //println!(" ---- EO nested_specs --");
                //println!(" -- rest specs --");
                //specs_to_hier(&rest[i..].to_vec()); // <-- is this the remainder of the iterator???

                // this kills the current function
                // save it in remaining_specs, add it in later?
                //return vec![Specific { network: current_specific.network, specifics: specs_to_hier(&rest[i..].to_vec())}];
                println!("creating remaining_specs with {:?}", s.network);
                current_specific = s;
                remaining_specs = vec![Specific { network: s.network, specifics: specs_to_hier(&rest[i+1..].to_vec())}];
                println!("  post remaining_specs");
                break;
            }

        }
        println!(" -- nested_specs, current_specific: {:?} --", first.network);
        //specs_to_hier(&nested_specs);
        //specs_to_hier(&nested_specs)
        //println!(" ---- EO nested_specs --");

        // add in remaining_specs here?
        //vec![Specific { network: first.network, specifics: specs_to_hier(&nested_specs) }] // this works


        // trying add:
        let mut result = vec![Specific { network: first.network, specifics: specs_to_hier(&nested_specs) }];
        result.append(&mut remaining_specs); //?
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

//problem: we need to cut up the vec of Routes and call specs_to_hier for every subset
// every subset contains one subtrie of the trie
// so once specs_to_hier terminates, we have one full subtrie
// then, we can push all the subtries to a Vec again
pub fn route_to_specifics(routes: &Vec<Route>) -> Vec<Specific> {
    specs_to_hier(&routes.iter().map(|r| Specific {network: r.prefix, specifics: Vec::new() } ).collect())
}

//pub fn vec_to_hier(routes: Vec<Route>) -> Option<Prefix> {
pub fn _vec_to_hier(routes: &Vec<Route>) -> Vec<Specific> {
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

pub struct Area {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub surface: f64,
    pub route: Route,
}

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
    pub fn new(surface: f64, ratio: f64, route: Route) -> Area {
        let w = surface.powf(ratio);
        let h = surface.powf(1.0 - ratio);
        Area { x: 0.0, y: 0.0, w, h, surface, route }
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
