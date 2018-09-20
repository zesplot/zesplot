use std::io;
use std::path::Path;
use clap::ArgMatches;
use svg;
use plot;

use std::io::{BufReader};
use std::io::prelude::*;
use std::fs::File;


fn construct_fn(matches: &ArgMatches) -> String {
    let mut output_fn = String::new();

    if matches.is_present("output-fn") {
        return matches.value_of("output-fn").unwrap().to_string();
    } else {
        output_fn.push_str(&Path::new(matches.value_of("address-file").unwrap()).file_name().unwrap().to_str().unwrap());
    }

    if matches.is_present("unsized-rectangles") {
        output_fn.push_str(".unsized");
    } else {
        output_fn.push_str(".sized");
    }
    if matches.is_present("filter-empty-prefixes") {
        output_fn.push_str(&format!(".filtered.ft{}", matches.value_of("filter-threshold").unwrap_or("1")));
    } else {
        output_fn.push_str(".unfiltered");
    }

    output_fn.push_str(&format!(".{}", matches.value_of("colour-input").unwrap_or(plot::COLOUR_INPUT)));
    output_fn
}

pub fn create_svg<'a>(matches: &ArgMatches, document: &svg::Document, output_dir: &'a str) -> io::Result<String> {
    let output_fn_svg = format!("{}/{}.svg", output_dir, construct_fn(&matches));
    println!("output.rs creating {}", output_fn_svg);
    svg::save(&output_fn_svg, document)?;

    Ok(output_fn_svg.to_string())
}

pub fn create_html<'a>(matches: &ArgMatches, document: &svg::Document, output_dir: &'a str) -> io::Result<String> {
    let mut raw_svg = Vec::new();
    let _ = svg::write(&mut raw_svg, document);

    let mut template = String::new();
    let template_fn = matches.value_of("html-template").unwrap();
    BufReader::new(
        File::open(template_fn)?
        ).read_to_string(&mut template).unwrap();

    let html = template.replace("__SVG__", &String::from_utf8_lossy(&raw_svg));
    let output_fn_html = format!("{}/{}.html", output_dir, construct_fn(&matches));

    println!("creating {}", output_fn_html);
    let mut html_file = File::create(&output_fn_html)?;
    html_file.write_all(&html.as_bytes()).unwrap();

    // create a file with a static name for easy experimenting with parameters
    let mut html_file = File::create(format!("{}/index.html", output_dir))?;
    html_file.write_all(&html.as_bytes()).unwrap();

    Ok(output_fn_html.to_string())
}
