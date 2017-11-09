extern crate svg;

use svg::*;
use svg::node::element::{Path, Rectangle};
use svg::node::element::path::Data;


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
    id: String,
}

impl Area {
    fn new(surface: f64, ratio: f64, id: String) -> Area {
        let w = surface.powf(ratio);
        let h = surface.powf(1.0 - ratio);
        //println!("area::new {} * {}", w, h);
        Area { x: 0.0, y: 0.0, w, h, surface, id }
    }
    fn get_ratio(&self) -> f64 {
        if &self.h >= &self.w {
            &self.w  / &self.h
        } else {
            &self.h / &self.w
        }
    }
}

struct Row {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    vertical: bool,
    areas: Vec<Area>,
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

fn main() {

    let mut inputs: Vec<f64> = Vec::new();

    for line in BufReader::new(File::open("allrirs.txt").unwrap()).lines() {
        //inputs.push(line.unwrap().parse().unwrap());
        inputs.push(2_f64.powf(128_f64 - line.unwrap().parse::<f64>().unwrap()  ));
    }
    //areas.sort(); //FIXME currently reading sorted input
    
    // initial aspect ratio
    let init_ar: f64 = 1_f64 / (8.0/3.0);

    let input_area_total = inputs.iter().fold(0.0, |mut s, i| { s += *i; s} );
    let norm_factor = (WIDTH * HEIGHT) / input_area_total;

    let mut areas: Vec<Area> = Vec::new();
    for i in inputs {
        areas.push(Area::new(i * norm_factor, init_ar, i.to_string()));
    }



    let mut rows = Vec::new();
    //let (first_area, remaining_areas) = areas.split_first().unwrap();
    let remaining_areas = areas.split_off(1); //.pop().unwrap();
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

    let colors = vec![  "#ff0000",
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
            //println!("{},{} {} * {}", area.x, area.y, area.w, area.h);
            let rect = Rectangle::new()
                .set("x", area.x)
                .set("y", area.y)
                .set("width", area.w)
                .set("height", area.h)
                //.set("fill", color)
                .set("fill", colors[i % colors.len()])
                .set("stroke-width", 0.0005 * area.surface)
                .set("stroke", "black")
                .set("opacity", 0.5)
                //.set("opacity", (area.x * area.y) / 10000_f64)
                
                .set("data-id", area.id)
                ;
            rects.push(rect);
            i += 1;
        }
    }

    let data = Data::new()
        .move_to((0, 0))
        .line_by((0, 100))
        .line_by((100, 0))
        .line_by((0, -100))
        .close();

    let path = Path::new()
        .set("fill", "none")
        .set("stroke", "black")
        .set("stroke-width", 0.02)
        .set("d", data);


    let mut document = Document::new().set("viewBox", (0, 0, WIDTH, HEIGHT))
        .add(path);
    for r in rects {
        document.append(r);
    }


    svg::save("image.svg", &document).unwrap();

}
