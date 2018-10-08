extern crate svg;
use svg::{Document, Node};
use svg::node::element::{Rectangle, Text, Group, Definitions, LinearGradient, Stop};
use svg::node::Text as Tekst;

use clap::ArgMatches;
use treemap::{PlotParams,Row};
use std::collections::HashMap;

pub const WIDTH: f64 = 160.0;
pub const HEIGHT: f64 = 100.0;
pub const PLOT_LIMIT: u64 = 2000;
pub const COLOUR_INPUT: &str = "hits";

// HSL Colour stuff
const COLOUR_MAX_HUE: f64           = 240.0; // 0 == red, 240 == blue
//const COLOUR_MAX_HUE_DISCRETE: f64  = 300.0;
const COLOUR_DISCRETE_HUE_DIFF: f64  = 60.0;
const COLOUR_SATURATION: u32        = 90;
const COLOUR_LIGHTNESS: u32         = 50;
const COLOUR_GREY: (f64, u32, u32)  = (180_f64, 0, 90); // grey


const LEGEND_GRADIENT_WIDTH: f64 = 3.0;     // width of the gradient itself
const LEGEND_GRADIENT_MARGIN: f64 = 2.0;    // margin between gradient and the plot and the ticks
const LEGEND_GRADIENT_HEIGHT: f64 = HEIGHT; // - LABEL_DP_DESC_HEIGHT;     // width of the gradient itself

//const TICK_FIRST_Y: f64 = 0.0; //LABEL_DP_DESC_HEIGHT * 1.5; 
const NO_OF_TICKS: u64 = 5;
const TICK_FONT_HEIGHT: f64 = 4.0;
const TICK_HEIGHT_DELTA: f64 = (LEGEND_GRADIENT_HEIGHT - TICK_FONT_HEIGHT) / (NO_OF_TICKS - 1) as f64; // -1 because n ticks need n-1 spaces inbetween
const TICK_X: f64 = WIDTH + LEGEND_GRADIENT_WIDTH + 2.0*LEGEND_GRADIENT_MARGIN ; 
//const TICK_FONT_SIZE: &str = "40%";&
//const TICK_FONT_SIZE: &str = &format!("{}px", TICK_FONT_HEIGHT);
const TICK_FONT_SIZE: &str = "4px";

pub const LEGEND_MARGIN_W: f64 = LEGEND_GRADIENT_WIDTH + 2.0*LEGEND_GRADIENT_MARGIN + 20.0;

#[derive(Debug)]
pub enum ColourScale {
    Continuous(ContinuousColourScale),
    Discrete(DiscreteColourScale),
}


#[derive(Debug)]
pub struct ContinuousColourScale {
    min: f64,
    median: f64,
    max: f64,
}

impl ContinuousColourScale {
    pub fn new(min: f64, median: f64, max: f64) -> ContinuousColourScale {
        ContinuousColourScale {
            min,
            median,
            max,
        }
    }


    // returns hsl format
    // h ==   0 -> red
    // h == 240 -> blue
    pub fn get(&self, dp: f64) -> (f64,u32,u32) {
        if dp == 0.0 || dp.is_nan() {
            return COLOUR_GREY;
        }

        assert!(dp >= 0.0);
        assert!(dp <= self.max);

        let range = self.max - self.min;

        let dp_norm = if range > 1024.0 {
            // go in logarithmic mode
            let norm: f64 = COLOUR_MAX_HUE / self.max.log2();
            if dp >= 1.0 {
                dp.log2() * norm
            } else {
                // log of a sub 1.0 number is negative and results in incorrect colours
                norm
            }
        } else {
            let norm: f64 = COLOUR_MAX_HUE / self.max;
            dp * norm
        };

        assert!(dp_norm >= 0.0, format!("dp_norm < 0.0: {}, original dp: {}", dp_norm, dp));
        assert!(dp_norm <= COLOUR_MAX_HUE, format!("dp_norm > COLOUR_MAX_HUE: {}, original dp: {}", dp_norm, dp));

        (COLOUR_MAX_HUE - dp_norm, COLOUR_SATURATION, COLOUR_LIGHTNESS)
    }


    #[allow(dead_code)]
    pub fn get_boxplot(&self, dp: f64) -> (f64,u32,u32) {
        assert!(dp >= 0.0);
        assert!(dp <= self.max);
        //debug!("ColourScale::get: {}", dp);
        //debug!("ColourScale: {:?}", &self);
        if dp == 0.0 || dp.is_nan() {
            return COLOUR_GREY;
        }
        if (self.max - self.min) <= 1.0 {
            return (120_f64, 80, 50); // green ('mid of scale')
        }
        let c = if dp >= self.median {
            - ((120_f64 / (self.max - self.median) as f64) * (dp - self.median) as f64)
        } else {
            ((120_f64 / (self.median - self.min) as f64) * (self.median - dp) as f64)
        };
        (120_f64 + c , 80, 50)
    }

    // use to create legend
    // for boxplot we might need something completely different..
    pub fn steps(&self, n: u64) -> (Vec<(f64,u32,u32)>, Vec<f64>) {
        let range = self.max - self.min; 
        let mut steps = Vec::new();
        let mut ticks = Vec::new();
        if range > 1024.0 {
            // logarithmic
            let step = range.log2() / (n-1) as f64;
            for i in 0..n {
                let i = i as f64 * step;
                steps.push(self.get( 2_f64.powf(i) ));
                ticks.push(2_f64.powf(i)); // self.min ?
            }
        } else {
            let step = range / (n-1) as f64;
            for i in 0..n {
                let i = i as f64 * step;
                steps.push(self.get(self.min + i));
                ticks.push(self.min + i);
            }
        }
        (steps, ticks)
    }

}

#[derive(Debug)]
pub struct DiscreteColourScale {
    asn_colours: HashMap<u32, String>,
    classes: Vec<String>,
    colours: Vec<f64>,
}

struct DiscreteColourGenerator {
    colour_count: u64,
    diff_level: u32,
    current_colour_diff: f64,
    offset: f64,
}

impl DiscreteColourGenerator {
    fn new() -> DiscreteColourGenerator {
        DiscreteColourGenerator {
            colour_count: 0,
            offset: 0.0,
            diff_level: 0,
            current_colour_diff: COLOUR_DISCRETE_HUE_DIFF,
        }
    }
}

// generator for contrasting colours
// first get 0, 60, 120, 180, ...
// then set offset to 30
// yield 30, 90, 150,
// set offset to 15
// yield 15, 75, 135
// etc.
impl Iterator for DiscreteColourGenerator {
    type Item = f64;
    fn next(&mut self) -> Option<f64> {
        let r = self.offset + (self.colour_count % 6) as f64 * self.current_colour_diff;
        self.colour_count += 1;
        if self.colour_count % 6 == 0 {
            self.diff_level += 1;
            self.offset = self.current_colour_diff / f64::from(2_u32.pow(self.diff_level));
        }
        Some(r)
    }
}

impl DiscreteColourScale {
    pub fn new(asn_colours: HashMap<u32, String>) -> DiscreteColourScale {
        let mut classes = asn_colours.values().cloned().collect::<Vec<String>>();
        classes.sort();
        classes.dedup();
        let colours = DiscreteColourGenerator::new().take(classes.len()).collect::<Vec<f64>>();
        DiscreteColourScale {
            asn_colours,
            classes,
            colours,
        }
    }
    // use with --asn-colours
    pub fn get(&self, asn: u32) -> (f64,u32,u32) {
        // if we do not have a mapping for this ASN, gracefully return grey
        if let Some(asn_colour) = self.asn_colours.get(&asn) {
            if let Some(i) = self.classes.iter().position(|c| c == asn_colour) {
                let hue = self.colours[i];
                return (hue, COLOUR_SATURATION, COLOUR_LIGHTNESS);
            }
        } else {
            debug!("no mapping for {} in --asn-colours file", &asn);
        }
        COLOUR_GREY
    }
}

pub fn draw_svg(matches: &ArgMatches, rows: Vec<Row>, plot_params: &PlotParams) -> svg::Document {
    let mut groups: Vec<Group> = Vec::new();
    let mut areas_plotted: u64 = 0;

    let plot_limit = value_t!(matches, "plot-limit", u64).unwrap_or(PLOT_LIMIT);
    for row in rows {
        
        if plot_limit > 0 && areas_plotted >= plot_limit {
            break;
        }

        for area in row.areas {
            let mut group = Group::new()
                //.set("data-something", area.specific.asn.to_string())
                ;

            let sub_rects = area.specific.all_rects(&area, &plot_params);
            for sub_rect in sub_rects {
                group.append(sub_rect);
            }



            if !matches.is_present("no-labels") && area.w > 0.5 {
                let mut label = Text::new()
                    .set("class", "label")
                    .set("x", area.x + area.w/2.0)
                    .set("y", area.y + area.h/2.0)
                    .set("font-family", "mono")
                    .set("font-size", format!("{}%", area.w.min(area.h))) // == f64::min
                    .set("text-anchor", "middle");
                    label.append(Tekst::new(area.specific.to_string()))
                    ;
                group.append(label);
            }
            groups.push(group);



            areas_plotted += 1;
        }
    }


    let (defs, legend_g) = match plot_params.colour_scale {
        ColourScale::Continuous(ref cs) => legend(&cs),
        ColourScale::Discrete(ref cs) => legend_discrete(&cs),
    };

    info!("plotting {} rectangles, limit was {}", areas_plotted, plot_limit);

    let mut document = Document::new()
                        .set("viewBox", (0, 0, WIDTH + LEGEND_MARGIN_W as f64, HEIGHT))
                        .set("id", "treeplot")
                        ;
    for g in groups {
        document.append(g);
    }

    document.append(defs);
    document.append(legend_g);
    document.append(legend_label(&plot_params));
    document

}


fn format_tick(n: f64) -> String {
    if n > 1_000_000_f64 {
        format!("{:.0}M", n/1_000_000_f64)
    } else if n > 1_000_f64 {
        format!("{:.0}K", n/1_000_f64)
    } else {
        format!("{:.0}", n)
    }
}


fn legend_label(plot_params: &PlotParams) -> Text {
    let ticks_max_width = 3.0 * TICK_FONT_HEIGHT;

    let mut legend_label = Text::new()
        .set("font-family", "serif")
        .set("font-size", TICK_FONT_SIZE)
        .set("writing-mode", "tb-rl")
        .set("x", TICK_X + ticks_max_width)
        .set("y", HEIGHT / 2.0)
        .set("text-anchor", "middle")
        .set("transform", format!("rotate(180, {}, {})", TICK_X + ticks_max_width, HEIGHT / 2.0 ))
        ;
        
        //.set("alignment-baseline", "hanging"); // this does not work in firefox
        legend_label.append(Tekst::new(plot_params.legend_label.clone()));

    legend_label
}

fn legend(colour_scale: &ContinuousColourScale) -> (Definitions, Group) {
    let mut defs = Definitions::new();
    let mut legend_g = Group::new();
    let mut gradient = LinearGradient::new()
                            .set("id", "grad0")
                            .set("x1", "0")
                            .set("x2", "0")
                            .set("y1", "0")
                            .set("y2", "1");

    // 100% == top of gradient
    let steps = NO_OF_TICKS as u64;
    let (colours, ticks) = colour_scale.steps(steps);
    for (i, (c, tick)) in colours.iter().zip(ticks.iter()).rev().enumerate() {
        let (h,s,l) = c;
        gradient.append(Stop::new()
                            .set("offset", format!("{}%", (100/(steps-1)) * i as u64))
                            .set("stop-color", format!("hsl({}, {}%, {}%)", h, s, l))
                            );

        let mut legend_tick = Text::new()
            .set("x", WIDTH + LEGEND_GRADIENT_WIDTH + LEGEND_GRADIENT_MARGIN*2.0)
            .set("y", TICK_FONT_HEIGHT + TICK_HEIGHT_DELTA*(i as f64))
            .set("font-family", "serif")
            .set("font-size", TICK_FONT_SIZE)
            .set("text-anchor", "left");
        legend_tick.append(Tekst::new(format_tick(*tick)));
        legend_g.append(legend_tick);
            
    }
    defs.append(gradient);

    let legend = Rectangle::new()
                    .set("x", WIDTH + LEGEND_GRADIENT_MARGIN)
                    .set("y", 0)
                    .set("width", LEGEND_GRADIENT_WIDTH)
                    .set("height", LEGEND_GRADIENT_HEIGHT)
                    .set("stroke-width", 0.1)
                    .set("stroke", "#aaaaaa")
                    .set("opacity", 1.0)
                    .set("fill", "url(#grad0)")
                    ;
    legend_g.append(legend);

    (defs, legend_g)
}

fn legend_discrete(discrete_colour_scale: &DiscreteColourScale) -> (Definitions, Group) {

    let definitions = Definitions::new(); // not used for this legend
    let mut legend_g = Group::new();
    
    let classes = &discrete_colour_scale.classes;
    let tick_y_diff = (HEIGHT - TICK_FONT_HEIGHT) / (classes.len() - 1) as f64;
    for (i, class) in classes.iter().enumerate() {
        let (h,s,l) = (discrete_colour_scale.colours[i], COLOUR_SATURATION, COLOUR_LIGHTNESS);
        let legend_rect = Rectangle::new()
            .set("x", WIDTH + LEGEND_GRADIENT_MARGIN)
            .set("y", tick_y_diff * (i as f64))
            .set("width", LEGEND_GRADIENT_WIDTH)
            .set("height", LEGEND_GRADIENT_WIDTH)
            .set("fill", format!("hsl({}, {}%, {}%)", h, s, l))
            ;

        let mut legend_tick = Text::new()
            .set("x", WIDTH + LEGEND_GRADIENT_WIDTH + LEGEND_GRADIENT_MARGIN*2.0)
            .set("y", TICK_FONT_HEIGHT + tick_y_diff * (i as f64))
            .set("font-family", "serif")
            .set("font-size", TICK_FONT_SIZE)
            .set("text-anchor", "left");
        legend_tick.append(Tekst::new(class.clone()));
        legend_g.append(legend_rect);
        legend_g.append(legend_tick);
            
    }

    (definitions, legend_g)
}


#[cfg(test)]
mod tests{
    use super::*;

    #[test]
    fn colour_scale_log() {
        let cs = ContinuousColourScale::new(1.0, 10.0, 2048.0);
        let (h,_,_) = cs.get(1.0);
        assert_eq!(h.round(), COLOUR_MAX_HUE);
        let (h,_,_) = cs.get(2048.0);
        assert_eq!(h.round(), 0.0);
        let (h,_,_) = cs.get(45.0);
        assert_eq!(h.round(), 120.0);
    }

    #[test]
    fn discrete_colour_gen() {
        let dcg = DiscreteColourGenerator::new();
        assert_eq!(dcg.take(6).collect::<Vec<f64>>(), vec![0.0, 60.0, 120.0, 180.0, 240.0, 300.0]);
        let dcg = DiscreteColourGenerator::new();
        assert_eq!(
            dcg.take(11).collect::<Vec<f64>>(),
            vec![0.0, 60.0, 120.0, 180.0, 240.0, 300.0, 30.0, 90.0, 150.0, 210.0, 270.0]
            );
        let dcg = DiscreteColourGenerator::new();
        assert_eq!(
            dcg.take(14).collect::<Vec<f64>>(),
            vec![0.0, 60.0, 120.0, 180.0, 240.0, 300.0, 30.0, 90.0, 150.0, 210.0, 270.0, 330.0, 15.0, 75.0]
            );
    }

    //#[test]
    //fn colour_scale_boxplot() {
    //    let cs = ColourScale::new(1.0, 10.0, 100.0);
    //    let (h,s,l) = cs.get_boxplot(1.0);
    //    assert_eq!(h, COLOUR_MAX_HUE);
    //    let (h,s,l) = cs.get_boxplot(100.0);
    //    assert_eq!(h, 0.0);

    //    let cs = ColourScale::new(10.0, 50.0, 100.0);
    //    let (h,s,l) = cs.get_boxplot(10.0);
    //    assert_eq!(h, COLOUR_MAX_HUE);
    //    let (h,s,l) = cs.get_boxplot(100.0);
    //    assert_eq!(h, 0.0);
    //}

}
