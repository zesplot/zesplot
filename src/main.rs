extern crate svg;
extern crate ipnetwork;
extern crate num;
extern crate clap;

//TODO clap for cli params

use svg::*;
use svg::node::Text as Tekst;
use svg::node::element::{Rectangle, Text, Group};

use ipnetwork::Ipv6Network;
use std::net::Ipv6Addr;
use num::PrimInt;

use std::io::{BufReader};
use std::io::prelude::*;
use std::fs::File;


const WIDTH: f64 = 160.0;
const HEIGHT: f64 = 100.0;

//#[derive (Copy, Clone)]
struct Area {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    surface: f64,
    route: Route,
}

struct Row {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    vertical: bool,
    areas: Vec<Area>,
}

struct Route {
    prefix: Ipv6Network,
    asn:    u32,
    hits:   u32,
}

impl Route {
    fn size(&self) -> u64 {
        2.pow(64 - self.prefix.prefix() as u32)
    }

    fn to_string(&self) -> String {
        format!("AS{}", &self.asn)
    }
}

impl Area {
    fn new(surface: f64, ratio: f64, route: Route) -> Area {
        let w = surface.powf(ratio);
        let h = surface.powf(1.0 - ratio);
        Area { x: 0.0, y: 0.0, w, h, surface, route }
    }
    fn get_ratio(&self) -> f64 {
        if &self.h >= &self.w {
            &self.w  / &self.h
        } else {
            &self.h / &self.w
        }
    }
}


impl Row {
    fn new(x: f64, y: f64, vertical: bool, mut area: Area) -> Row {
        let max_h = HEIGHT - y;
        let max_w = WIDTH - x;
        if vertical {
            area.h = max_h;
            area.w = area.surface / area.h;
        } else {
            area.w = max_w;
            area.h = area.surface / area.w;
        }
        Row {x, y, w: area.w, h: area.h, vertical, areas:vec![area]}
    }

    fn try(&mut self, area: Area) -> Option<Area> {
        let cur_worst = self.calc_worst();
        &self.push(area);

        if self.calc_worst() >= cur_worst {
            None
        } else {
            self.pop()
        }
    }


    fn reflow(&mut self) -> () {
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

fn _color(i: u32) -> String  {
    if i == 0 {
        "#eeeeee".to_string()
    } else {
        format!("#00{:02x}{:02x}", 0xFF-i, i)
    }
}

fn color(i: u32, max: u32) -> String  {
    if i == 0 {
        "#eeeeee".to_string()
    } else {
        let norm_factor = (1.0 / ((max as f32).log2() / 255.0)) as f32;
        let v = (norm_factor *(i as f32).log2()) as u32;
        format!("#{:02x}00{:02x}", v, 0xFF-v)
    }
}

fn main() {

    eprintln!("-- reading input files");

    let mut dots: Vec<Ipv6Addr> = Vec::new();
    for line in BufReader::new(
        File::open("ipv6_hits.txt").unwrap()).lines()
        {
            let line = line.unwrap();
            dots.push(line.parse().unwrap());
        }
    dots.sort();

    let mut routes: Vec<Route> = Vec::new();
    let mut total_area = 0_u64;

    // TODO this input is generated a la:
    // ./bgpdump -M latest-bview.gz | ack "::/" cut -d'|' -f 6,7 --output-delimiter=" " | awk '{print $1,$NF}' |sort -u
    // now, this still includes 6to4 2002::/16 announcements
    // should we filter these out?

    for line in BufReader::new(
        File::open("ipv6_prefixes.txt").unwrap())
        .lines()
            //.take(1000) 
            {
        let line = line.unwrap();
        let parts: Vec<&str> = line.split(' ').collect();//::<(&str,&str)>();
        //let route = IPAddress::parse(parts[0]).unwrap();
        let route: Ipv6Network = parts[0].parse().unwrap();

        match parts[1].parse::<u32>() {
            Ok(asn) => {
                let r = Route {prefix: route, asn, hits: 0};
                total_area += r.size();
                routes.push(r)
            },
            Err(e) => println!("Error in {}: {}", parts[1],  e)
        }
    }

    eprintln!("-- matching /128s with prefixes");

    routes.sort_by(|a, b| a.prefix.cmp(&b.prefix));

    let mut start_i = 0;
    let mut max_hits = 0;
    for r in &mut routes {
        let mut hits = 0;
        for (i, d) in dots[start_i..].iter().enumerate() {
            if r.prefix.contains(*d) {
                hits += 1;
            } else if Ipv6Network::new(*d, 128).unwrap() > r.prefix {
                start_i = start_i + i - 1;
                break;
            }
             
        }

        r.hits = hits;
        if hits > max_hits {
            max_hits = hits;
        }
    }

    eprintln!("-- fitting areas in plot");

    let hide_empty_areas = true; // <-- TODO this should be a cli flag
    println!("pre: {} routes, total size {}", routes.len(), total_area);
    if hide_empty_areas {
        routes.retain(|r| r.hits > 0);
    }
    total_area = routes.iter().fold(0, |mut s, r|{s += r.size(); s});
    println!("post: {} routes, total size {}", routes.len(), total_area);

    // initial aspect ratio FIXME this doesn't affect anything, remove
    let init_ar: f64 = 1_f64 / (8.0/1.0);

    let norm_factor = (WIDTH * HEIGHT) / total_area as f64;

    let mut areas: Vec<Area> = Vec::new();

    routes.sort_by(|a, b| b.size().cmp(&a.size()));

    for r in routes {
        areas.push(Area::new(r.size() as f64 * norm_factor, init_ar, r  ));
    }



    let mut rows = Vec::new();
    //let (first_area, remaining_areas) = areas.split_first().unwrap();
    let remaining_areas = areas.split_off(1);
    let first_area = areas.pop().unwrap();
    let (mut new_row_x, mut new_row_y) = (0.0, 0.0);
    rows.push(Row::new(new_row_x, new_row_y, true, first_area));
    let mut i = 0;

    for a in remaining_areas {

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


    println!("-- creating svg");
    //let mut rects: Vec<Rectangle> = Vec::new();
    //let mut labels: Vec<Text> = Vec::new();
    let mut groups: Vec<Group> = Vec::new();

    let mut i = 0;
    for row in rows {
        //println!("new row: {}", direction);
        for area in row.areas {
            if area.surface < 0.5 { break; } // TODO make this a cli param
            let mut border = 0.0005 * area.surface;
            if border > 0.4 {
                border = 0.4;
            }


            let rect = Rectangle::new()
                .set("x", area.x)
                .set("y", area.y)
                .set("width", area.w)
                .set("height", area.h)
                .set("fill", color(area.route.hits, max_hits)) 
                .set("stroke-width", border)
                .set("stroke", "black")
                .set("opacity", 1.0)
                ;
            let mut group = Group::new()
                .set("data-asn", area.route.asn.to_string())
                .set("data-prefix", area.route.prefix.to_string())
                .set("data-hits", area.route.hits.to_string())
                ;
            group.append(rect);
            if area.w > 0.5 {
                let mut label = Text::new()
                    .set("x", area.x + area.w/2.0)
                    .set("y", area.y + area.h/2.0)
                    .set("font-family", "mono")
                    .set("font-size", format!("{}%", area.w))
                    .set("text-anchor", "middle");
                    label.append(Tekst::new(area.route.to_string()))
                    ;
                //labels.push(label);
                group.append(label);
            }
            groups.push(group);



            i += 1;
        }
    }

    eprintln!("  -- created {} rects", i);

    let mut document = Document::new()
                        .set("viewBox", (0, 0, WIDTH, HEIGHT))
                        .set("id", "treeplot")
                        ;
//    for r in rects {
//        document.append(r);
//    }
//    for l in labels {
//        document.append(l);
//    }
    for g in groups {
        document.append(g);
    }

    eprintln!("-- creating output files");

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

    eprintln!("-- done!");

}
