extern crate svg;

use svg::*;
use svg::node::*;
use svg::node::element::*;
use svg::node::element::{Path, Rectangle};
use svg::node::element::path::Data;


use std::io::{self, BufReader};
use std::io::prelude::*;
use std::fs::File;

//fn ratio(r: Rectangle) -> f64 {
//    //let (_, _, a) = r;
//    //r.description.get("w").unwrap()
//}

fn main() {
    //let prefixes = vec!["2001:db8:1::/48", "2001:db8:2::/48", "2001:db8:3:1:/64"];
    //let areas: Vec<f64> = vec![128.0 - 29.0, 128.0 - 32.0, 128.0 - 32.0, 128.0 - 48.0, 128.0 - 48.0, 128.0 - 64.0];

    let mut areas: Vec<f64> = Vec::new();

    for line in BufReader::new(File::open("input.txt").unwrap()).lines() {
        areas.push(line.unwrap().parse().unwrap());
    }
    //areas.sort(); //FIXME currently reading sorted input
    
    // initial aspect ratio
    let init_ar: f64 = 1_f64 / (8.0/3.0);

    let area_total = areas.iter().fold(0.0, |mut s, i| { s += *i; s} );
    let norm_factor = (100.0 * 100.0) / area_total;
    println!("total: {}", area_total);
    let mut rects = Vec::new();
    let mut i = 0;

    let (mut cur_x, mut cur_y) = (0.0, 0.0);
    let mut direction = false; // boolean to work either horizontally or vertically
    let mut new_row = false;
    for a in areas {
        // normalize size of a
        let a = a * norm_factor;

        if new_row {
            cur_y = 0.0;
            new_row = false;
        }

        let mut w = a.powf(init_ar);
        let mut h = a.powf(1.0 - init_ar);

        if !direction {
            let tmp = w;
            w = h;
            h = tmp;
        }

        if w > (100.0 - cur_x) {
            let tmp = (100.0 - cur_x) / w; 
            h = h * (1.0/tmp);
            w = 100.0 - cur_x;
            direction = false;
        } else if h > (100.0 - cur_y) {
            let tmp = (100.0 - cur_y) / h; 
            w = w * (1.0/tmp);
            h = 100.0 - cur_y;
            direction = true;
            new_row = true;
        }

        println!("{} = {} x {}", a, w, h); 
        let rect = Rectangle::new()
                .set("x", cur_x)
                .set("y", cur_y)
                .set("width", w)
                .set("height", h)
                .set("fill", "magenta")
                .set("stroke-width", 1)
                .set("stroke", "green")
                .set("opacity", 0.25)
                ;
        if direction {
            cur_x = cur_x + w;
        } else {
            cur_y = cur_y + h;
        }
        //direction = !direction;
        rects.push(rect);
        i = i + 1;
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
        .set("stroke-width", 1)
        .set("d", data);

//    let rect = Rectangle::new()
//                .set("x", 0)
//                .set("y", 0)
//                .set("width", 100)
//                .set("height", 100)
//                .set("d", data);

    let mut document = Document::new().set("viewBox", (0, 0, 105, 105))
        .add(path);
    for r in rects {
        document.append(r);
    }


    svg::save("image.svg", &document).unwrap();

}
