extern crate svg;
use svg::{Document, Node};
use svg::node::element::{Rectangle, Text, Group, Definitions, LinearGradient, Stop};
use svg::node::Text as Tekst;

use clap::ArgMatches;
use treemap::{PlotParams,Row};

pub const WIDTH: f64 = 160.0;
pub const HEIGHT: f64 = 100.0;
pub const PLOT_LIMIT: u64 = 2000;
pub const COLOUR_INPUT: &str = "hits";

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
pub struct ColourScale {
    min: f64,
    median: f64,
    max: f64,
}

impl ColourScale {
    pub fn new(min: f64, median: f64, max: f64) -> ColourScale {
        ColourScale {
            min,
            median,
            max,
        }
    }


    // returns hsl format
    // h ==   0 -> red
    // h == 240 -> blue
    pub fn get(&self, dp: f64) -> (f64,u32,u32) {
        assert!(dp >= 0.0);
        assert!(dp <= self.max);

        if dp == 0.0 || dp.is_nan() {
            return (180_f64, 0, 90); // white grey-ish
        }

        let range = self.max - self.min;

        let dp_norm = if range > 1024.0 {
            // go in logarithmic mode
            let norm: f64 = 240.0 / self.max.log2();
            dp.log2() * norm
        } else {
            let norm: f64 = 240.0 / self.max;
            dp * norm
        };

        //println!("dp_norm: {}", dp_norm);
        (240.0 - dp_norm, 90, 50)
    }

    #[allow(dead_code)]
    pub fn get_boxplot(&self, dp: f64) -> (f64,u32,u32) {
        assert!(dp >= 0.0);
        assert!(dp <= self.max);
        //debug!("ColourScale::get: {}", dp);
        //debug!("ColourScale: {:?}", &self);
        if dp == 0.0 || dp.is_nan() {
            return (180_f64, 10, 75); // grey
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
                debug!("step loop log i:{}", i);
                let i = i as f64 * step;
                steps.push(self.get( 2_f64.powf(i) ));
                ticks.push(2_f64.powf(i)); // self.min ?
            }
        } else {
            let step = range / (n-1) as f64;
            for i in 0..n {
                debug!("step loop i:{}", i);
                let i = i as f64 * step;
                steps.push(self.get(self.min + i));
                ticks.push(self.min + i);
            }
        }
        (steps, ticks)
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


    /* FIXME disabled while refactoring to PlotParams

    let (defs, legend_g) = if matches.is_present("asn-colours") {
        legend_discrete(&plot_info)
    } else {
        legend(&plot_info)
    };
    */

    let (defs, legend_g) = legend(&plot_params);

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

fn legend(plot_params: &PlotParams) -> (Definitions, Group) {
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
    let (colours, ticks) = plot_params.colour_scale.steps(steps);
    debug!("ticks ({}): {:?}", ticks.len(), ticks);
    debug!("colours ({}): {:?}", colours.len(), colours);
    //for (i, (c, tick)) in plot_params.colour_scale.steps(steps).iter().rev().enumerate() {
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
        //legend_label_100.append(Tekst::new(format!("{:.0}", legend_100)))
        //legend_label_100.append(Tekst::new(tick_label(legend_100))) ;

        //legend_tick.append(Tekst::new(format!("{:.0}", tick)));
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

    
    // TODO: more precise way of determining actual maximum width
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
    
    legend_g.append(legend_label);


    (defs, legend_g)
}

/*

pub fn legend(plot_info: &PlotInfo) -> (Definitions, Group) {

    // vertical:
    // <linearGradient id="Gradient2" x1="0" x2="0" y1="0" y2="1">
    let mut defs = Definitions::new();
    let mut gradient = LinearGradient::new()
                            .set("id", "grad0")
                            .set("x1", "0")
                            .set("x2", "0")
                            .set("y1", "0")
                            .set("y2", "1");

    // 100% == top of gradient
    gradient.append(Stop::new()
                        .set("offset", "0%")
                        .set("stop-color", "#ff0000")
                        );
    gradient.append(Stop::new()
                        .set("offset", "25%")
                        .set("stop-color", "#ffff00")
                        );
    gradient.append(Stop::new()
                        .set("offset", "50%")
                        .set("stop-color", "#00ff00")
                        );
    gradient.append(Stop::new()
                        .set("offset", "75%")
                        .set("stop-color", "#00ffff")
                        );
    gradient.append(Stop::new()
                        .set("offset", "100%")
                        .set("stop-color", "#0000ff")
                        );
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

    // ticks

    // match on colour_mode, find out which max to use and create a title a la var(ttl)
    let legend_100 = match plot_info.colour_mode {
        ColourMode::Hits => plot_info.max_hits as f64,
        ColourMode::DpAvg => plot_info.max_dp_avg as f64,
        ColourMode::DpMedian => plot_info.max_dp_median as f64,
        ColourMode::DpVar => plot_info.max_dp_var as f64,
        ColourMode::DpUniq =>plot_info.max_dp_uniq as f64,
        ColourMode::DpSum => plot_info.max_dp_sum as f64,
        ColourMode::HwAvg => plot_info.max_hw_avg as f64,
        ColourMode::Asn => 5.0, //FIXME how do we do a scale based on plot_info.asn_colours ?
    };

    let norm = if legend_100 > 1024.0 {
        1024.0 / (legend_100 as f64).log2()
    } else {
        1024.0 / legend_100 as f64
    };

    let legend_75; 
    let legend_50; 
    let legend_25; 
    let legend_0 = 1.0;

    if legend_100 > 1024.0 {
        legend_75 = 2_f64.powf(786_f64 / norm);
        legend_50 = 2_f64.powf(512_f64 / norm);
        legend_25 = 2_f64.powf(256_f64 / norm);
        //legend_0 = 1.0;
    } else {
        legend_75 = 786_f64 / norm;
        legend_50 = 512_f64 / norm;
        legend_25 = 256_f64 / norm;
        //legend_0 = 1.0;
    }

    let mut legend_label_100 = Text::new()
        .set("x", WIDTH + LEGEND_GRADIENT_WIDTH + LEGEND_GRADIENT_MARGIN*2.0)
        .set("y", 5.0 + TICK_FIRST_Y + TICK_HEIGHT_DELTA*0.0)
        .set("font-family", "serif")
        .set("font-size", TICK_FONT_SIZE)
        .set("text-anchor", "left");
        //legend_label_100.append(Tekst::new(format!("{:.0}", legend_100)))
        legend_label_100.append(Tekst::new(tick_label(legend_100)))
        ;
    let mut legend_label_75 = Text::new()
        .set("x", WIDTH + LEGEND_GRADIENT_WIDTH + LEGEND_GRADIENT_MARGIN*2.0)
        .set("y", TICK_FIRST_Y + TICK_HEIGHT_DELTA*1.0)
        .set("font-family", "serif")
        .set("font-size", TICK_FONT_SIZE)
        .set("text-anchor", "left");
        legend_label_75.append(Tekst::new(tick_label(legend_75)))
        ;
    let mut legend_label_50 = Text::new()
        .set("x", WIDTH + LEGEND_GRADIENT_WIDTH + LEGEND_GRADIENT_MARGIN*2.0)
        .set("y", TICK_FIRST_Y + TICK_HEIGHT_DELTA*2.0)
        .set("font-family", "serif")
        .set("font-size", TICK_FONT_SIZE)
        .set("text-anchor", "left");
        legend_label_50.append(Tekst::new(tick_label(legend_50)))
        ;
    let mut legend_label_25 = Text::new()
        .set("x", WIDTH + LEGEND_GRADIENT_WIDTH + LEGEND_GRADIENT_MARGIN*2.0)
        .set("y", TICK_FIRST_Y + TICK_HEIGHT_DELTA*3.0)
        .set("font-family", "serif")
        .set("font-size", TICK_FONT_SIZE)
        .set("text-anchor", "left");
        legend_label_25.append(Tekst::new(tick_label(legend_25)))
        ;
    let mut legend_label_0 = Text::new()
        .set("x", WIDTH + LEGEND_GRADIENT_WIDTH + LEGEND_GRADIENT_MARGIN*2.0)
        .set("y", TICK_FIRST_Y + TICK_HEIGHT_DELTA*4.0)
        .set("font-family", "serif")
        .set("font-size", TICK_FONT_SIZE)
        .set("text-anchor", "left");
        legend_label_0.append(Tekst::new(tick_label(legend_0)))
        ;

    let mut legend_g = Group::new();
    legend_g.append(legend);
    legend_g.append(legend_label_100);
    legend_g.append(legend_label_75);
    legend_g.append(legend_label_50);
    legend_g.append(legend_label_25);
    legend_g.append(legend_label_0);

    let dp_desc_text = match plot_info.colour_mode {
            ColourMode::DpAvg   => format!("mean({})",  plot_info.dp_desc),
            ColourMode::DpMedian   => format!("median({})",  plot_info.dp_desc),
            ColourMode::DpVar   => format!("var({})",   plot_info.dp_desc),
            ColourMode::DpUniq  => format!("uniq({})",  plot_info.dp_desc),
            ColourMode::DpSum   => format!("sum({})",   plot_info.dp_desc),
            _   =>  plot_info.dp_desc.to_string(), //"Responses".to_string(), //colour_mode
        };

    // TODO: more precise way of determining actual maximum width
    // the second or fourth tick could still be longer than the middle one
    let ticks_max_width = (tick_label(legend_50).len() * 5) as f64 + 1.0;

    let mut legend_dp_desc = Text::new()
        //.set("x", WIDTH + LEGEND_GRADIENT_MARGIN*1.0)
        //.set("y", LABEL_DP_DESC_HEIGHT - 2.0)
        .set("font-family", "serif")
        .set("font-size", TICK_FONT_SIZE)
        // vertical:
        .set("writing-mode", "tb-rl")
        .set("x", TICK_X + ticks_max_width)
        .set("y", HEIGHT / 2.0)
        .set("text-anchor", "middle")
        ;
        

        //.set("alignment-baseline", "hanging"); // this does not work in firefox
        legend_dp_desc.append(Tekst::new(dp_desc_text));
    
    legend_g.append(legend_dp_desc);

    (defs, legend_g)
}

pub fn legend_discrete(_plot_info: &PlotInfo) -> (Definitions, Group) {

    // NB hardcoded for now, should match the HashMap in treemap.rs
    let scale = vec!["#ff0000",
                     "#ffff00",
                     "#00ff00",
                     "#ff00ff",
                     "#00ffff",
                     "#0000ff",
                    ];


    let defs = Definitions::new();
    let mut group = Group::new();
    //for (i, (id, colour)) in scale.iter().enumerate() {
    let legend_box_width = 5.0;
    for (i, colour) in scale.iter().enumerate() {
        let r = Rectangle::new()
                    .set("x", WIDTH + LEGEND_GRADIENT_MARGIN)
                    .set("y", i as f64 * (HEIGHT / scale.len() as f64))
                    .set("width", legend_box_width)
                    .set("height", legend_box_width)
                    .set("fill", colour.to_string())
                    .set("opacity", 1.0)
                ;
        let mut tick_label = Text::new()
                    .set("x", WIDTH + LEGEND_GRADIENT_WIDTH + LEGEND_GRADIENT_MARGIN*2.0 + 1.0)
                    .set("y", 5.0 + i as f64 * (HEIGHT / scale.len() as f64))
                    .set("font-family", "serif")
                    .set("font-size", TICK_FONT_SIZE)
                ;
        tick_label.append(Tekst::new(format!("{}", i + 1))); // as i is the id
        group.append(r);
        group.append(tick_label);
    }


    let mut legend_dp_desc = Text::new()
        //.set("x", WIDTH + LEGEND_GRADIENT_MARGIN*1.0)
        //.set("y", LABEL_DP_DESC_HEIGHT - 2.0)
        .set("font-family", "serif")
        .set("font-size", TICK_FONT_SIZE)
        // vertical:
        .set("writing-mode", "tb-rl")
        .set("x", WIDTH + LEGEND_GRADIENT_MARGIN*2.0 + legend_box_width + 6.0)
        .set("y", HEIGHT / 2.0)
        .set("text-anchor", "middle")
        ;

        //.set("alignment-baseline", "hanging"); // this does not work in firefox
        legend_dp_desc.append(Tekst::new("Cluster ID"));

    group.append(legend_dp_desc);
    (defs, group)
}

pub fn tick_label(v: f64) -> String {

    match v as u64 {
        i if i > 1_000_000 => format!("{:.0}M", (i/1_000_000)), //.to_string()
        1000...999_999 => format!("{:.0}K", (v/1000.0)), //.to_string()
        _ => format!("{:.0}", v)
    }
}
*/

#[cfg(test)]
mod tests{
    use super::*;

    #[test]
    fn colour_scale_log() {
        let cs = ColourScale::new(1.0, 10.0, 2048.0);
        let (h,s,l) = cs.get(1.0);
        assert_eq!(h.round(), 240.0);
        let (h,s,l) = cs.get(2048.0);
        assert_eq!(h.round(), 0.0);

        let (h,s,l) = cs.get(45.0);
        assert_eq!(h.round(), 120.0);


        //let cs = ColourScale::new(10.0, 50.0, 100.0);
        //let (h,s,l) = cs.get(10.0);
        //assert_eq!(h, 240.0);
        //let (h,s,l) = cs.get(100.0);
        //assert_eq!(h, 0.0);

    }

    #[test]
    fn colour_scale_boxplot() {
        let cs = ColourScale::new(1.0, 10.0, 100.0);
        let (h,s,l) = cs.get_boxplot(1.0);
        assert_eq!(h, 240.0);
        let (h,s,l) = cs.get_boxplot(100.0);
        assert_eq!(h, 0.0);


        let cs = ColourScale::new(10.0, 50.0, 100.0);
        let (h,s,l) = cs.get_boxplot(10.0);
        assert_eq!(h, 240.0);
        let (h,s,l) = cs.get_boxplot(100.0);
        assert_eq!(h, 0.0);

    }

}
