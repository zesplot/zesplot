#![feature(drain_filter)]

extern crate svg;
extern crate ipnetwork;
extern crate num;

use svg::*;
use svg::node::Text as Tekst;
use svg::node::element::{Rectangle, Text};

use ipnetwork::Ipv6Network;
use std::net::Ipv6Addr;
use num::PrimInt;

use std::io::{BufReader};
use std::io::prelude::*;
use std::fs::File;
//use std::cmp;


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
        //2.pow(64 - self.prefix.prefix.num as u32)
        2.pow(64 - self.prefix.prefix() as u32)
    }

    fn to_string(&self) -> String {
        //format!("AS{}<br/>{}", &self.asn, &self.prefix.to_string())
        format!("AS{}", &self.asn)
    }
}

impl Area {
    fn new(surface: f64, ratio: f64, route: Route) -> Area {
        let w = surface.powf(ratio);
        let h = surface.powf(1.0 - ratio);
        //println!("area::new {} * {}", w, h);
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
        //println!("Row::new at {},{}", x, y);
        let max_h = HEIGHT - y;
        let max_w = WIDTH - x;
        //if area.h > max_h && max_h > 0.0 {
        if vertical {
            area.h = max_h;
            area.w = area.surface / area.h;
        //} else if area.w > max_w {
        } else {
            area.w = max_w;
            area.h = area.surface / area.w;
        }
        //println!("  {} * {}", area.w, area.h);
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
        //println!("reflow:");
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
                //println!("  area {} set cur_y to {}", a.surface, cur_y);
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
                //println!("  area {} set cur_y to {}", a.surface, cur_x);
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

fn color(i: u32) -> String  {
    if i == 0 {
        "#ff0000".to_string()
    } else {
        format!("#00{:02x}{:02x}", 0xFF-i, i)
    }
}

fn main() {

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
                let _h: Vec<Ipv6Addr> = dots.drain_filter(|d| route.contains(*d)).collect();
                //let hits = _v.len();
                //println!("got {} hits for {}", hits, asn);
                let r = Route {prefix: route, asn, hits: _h.len() as u32};
                total_area += r.size();
                routes.push(r)
            },
            Err(e) => println!("Error in {}: {}", parts[1],  e)
        }
    }


    // initial aspect ratio FIXME this doesn't affect anything, remove
    let init_ar: f64 = 1_f64 / (8.0/1.0);

    let norm_factor = (WIDTH * HEIGHT) / total_area as f64;

    let mut areas: Vec<Area> = Vec::new();

    //TODO can we order by prefix, so they appear closer in the plot?
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
                new_row_x += cur_row_w;
                //println!("new horizontal row at {},{}", new_row_x, new_row_y);
                rows.push(Row::new(new_row_x, new_row_y, false, area));
            } else {
                new_row_y += cur_row_h;
                // create new vertical row
                //println!("new vertical row at {},{}", new_row_x, new_row_y);
                rows.push(Row::new(new_row_x, new_row_y, true, area));
            }
            rows.last_mut().unwrap().reflow();
        }

        i = i + 1;
    }


    println!(" --- drawing --- ");
    let mut rects: Vec<Rectangle> = Vec::new();
    let mut labels: Vec<Text> = Vec::new();

    // TODO remove?
    let _colors = vec![  "#ff0000",
                        "#00ff00",
                        "#0000ff",
                        "#ffff00",
                        "#00ffff",
                        //"#ff00ff",
                        ];
    let mut i = 0;
    for row in rows {
        //println!("new row: {}", direction);
        for area in row.areas {
            if area.surface < 2.0 { break; }
            //println!("{},{} {} * {}", area.x, area.y, area.w, area.h);
            let mut border = 0.0005 * area.surface;
            if border > 0.4 {
                border = 0.4;
            }
            //let _v: Vec<Ipv6Addr> = dots.drain_filter(|d| area.route.prefix.contains(*d)).collect();
            //let hits = _v.len();

            let rect = Rectangle::new()
                .set("x", area.x)
                .set("y", area.y)
                .set("width", area.w)
                .set("height", area.h)
                //.set("fill", colors[hits % colors.len()]) // FIXME this is bullshit, we need some kind of normalized color scale
                .set("fill", color(area.route.hits)) // FIXME this is bullshit, we need some kind of normalized color scale
                .set("stroke-width", border)
                .set("stroke", "black")
                .set("opacity", 0.5)
                .set("data-prefix", area.route.prefix.to_string())
                .set("data-hits", area.route.hits.to_string())
                .set("title", area.route.to_string())
                ;
            rects.push(rect);
            if area.w > 5.0 {
                let mut label = Text::new()
                    .set("x", area.x + area.w/2.0)
                    .set("y", area.y + area.h/2.0)
                    .set("font-family", "mono")
                    .set("font-size", format!("{}%", area.w))
                    .set("text-anchor", "middle");
                    label.append(Tekst::new(area.route.to_string()))
                    //label.append(Tekst::new(area.route.prefix.to_string()))
                    ;
                labels.push(label);
            }


            i += 1;
        }
    }

    println!("created {} rects", i);

    let mut document = Document::new().set("viewBox", (0, 0, WIDTH, HEIGHT));
    for r in rects {
        document.append(r);
    }
    for l in labels {
        document.append(l);
    }


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
