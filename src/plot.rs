extern crate svg;
use svg::{Document, Node};
use svg::node::element::{Rectangle, Text, Group, Definitions, LinearGradient, Stop};
use svg::node::Text as Tekst;

use clap::ArgMatches;
use treemap::{PlotInfo,ColourMode,Row};

pub const WIDTH: f64 = 160.0;
pub const HEIGHT: f64 = 100.0;
pub const PLOT_LIMIT: u64 = 2000;
pub const COLOUR_INPUT: &str = "hits";

const LEGEND_GRADIENT_WIDTH: f64 = 3.0;     // width of the gradient itself
const LEGEND_GRADIENT_MARGIN: f64 = 2.0;    // margin between gradient and the plot and the ticks
const LEGEND_GRADIENT_HEIGHT: f64 = HEIGHT; // - LABEL_DP_DESC_HEIGHT;     // width of the gradient itself

const TICK_FIRST_Y: f64 = 0.0; //LABEL_DP_DESC_HEIGHT * 1.5; 
const TICK_HEIGHT_DELTA: f64 = LEGEND_GRADIENT_HEIGHT / 4.0; // 4.0 because we have 5 ticks, so 4 spaces in between
const TICK_X: f64 = WIDTH + LEGEND_GRADIENT_WIDTH + 2.0*LEGEND_GRADIENT_MARGIN ; 
const TICK_FONT_SIZE: &str = "40%";
pub const LEGEND_MARGIN_W: f64 = LEGEND_GRADIENT_WIDTH + 2.0*LEGEND_GRADIENT_MARGIN + 20.0;



pub fn draw_svg(matches: &ArgMatches, rows: Vec<Row>, plot_info: &PlotInfo) -> svg::Document {
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

            let sub_rects = area.specific.all_rects(&area, &plot_info);
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

    let (defs, legend_g) = if matches.is_present("asn-colours") {
        legend_discrete(&plot_info)
    } else {
        legend(&plot_info)
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
    document

}

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
