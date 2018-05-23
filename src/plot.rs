extern crate svg;
use svg::Node;
use svg::node::element::{Rectangle, Circle, Text, Group, Definitions, LinearGradient, Stop};
use svg::node::Text as Tekst;

use treemap::{PlotInfo,ColourMode};

pub const WIDTH: f64 = 160.0;
pub const HEIGHT: f64 = 100.0;
pub const PLOT_LIMIT: u64 = 2000;
pub const COLOUR_INPUT: &str = "hits";

const LABEL_DP_DESC_HEIGHT: f64 = 6.0;      // height of the datapoint description label (e.g. var(TTL))
const LEGEND_GRADIENT_WIDTH: f64 = 3.0;     // width of the gradient itself
const LEGEND_GRADIENT_MARGIN: f64 = 2.0;    // margin between gradient and the plot and the ticks
const LEGEND_GRADIENT_HEIGHT: f64 = HEIGHT - LABEL_DP_DESC_HEIGHT;     // width of the gradient itself

const TICK_FIRST_Y: f64 = LABEL_DP_DESC_HEIGHT * 1.5; 
const TICK_HEIGHT_DELTA: f64 = LEGEND_GRADIENT_HEIGHT / 4.0; // 4.0 because we have 5 ticks, so 4 spaces in between
pub const LEGEND_MARGIN_W: f64 = LEGEND_GRADIENT_WIDTH + 2.0*LEGEND_GRADIENT_MARGIN + 20.0; // FIXME 5.0 for Tekst width?
//pub const LEGEND_MARGIN_Y: f64 = LEGEND_GRADIENT_WIDTH + 2.0*LEGEND_GRADIENT_MARGIN + 25.0; // FIXME 5.0 for Tekst width?



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
                    .set("y", LABEL_DP_DESC_HEIGHT)
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
        ColourMode::DpVar => plot_info.max_dp_var as f64,
        ColourMode::DpUniq =>plot_info.max_dp_uniq as f64,
        ColourMode::DpSum => plot_info.max_dp_sum as f64,
        ColourMode::Asn => 5.0, //FIXME how do we do a scale based on plot_info.asn_colours ?
    };

    let norm = 1024.0 / (legend_100 as f64).log2();
    // round of max
    let legend_75 = 2_f64.powf(786_f64 / norm);
    let legend_50 = 2_f64.powf(512_f64 / norm);
    let legend_25 = 2_f64.powf(256_f64 / norm);
    let legend_0 = 1.0;

    //let tick_font_size = HEIGHT / 20.0; //format!("{}", );
    let tick_font_size = "40%";

    let mut legend_label_100 = Text::new()
        .set("x", WIDTH + LEGEND_GRADIENT_WIDTH + LEGEND_GRADIENT_MARGIN*2.0)
        .set("y", 2.0 + TICK_FIRST_Y + TICK_HEIGHT_DELTA*0.0)
        .set("font-family", "serif")
        .set("font-size", tick_font_size.clone())
        .set("text-anchor", "left");
        //legend_label_100.append(Tekst::new(format!("{:.0}", legend_100)))
        legend_label_100.append(Tekst::new(tick_label(legend_100)))
        ;
    let mut legend_label_75 = Text::new()
        .set("x", WIDTH + LEGEND_GRADIENT_WIDTH + LEGEND_GRADIENT_MARGIN*2.0)
        .set("y", TICK_FIRST_Y + TICK_HEIGHT_DELTA*1.0)
        .set("font-family", "serif")
        .set("font-size", tick_font_size.clone())
        .set("text-anchor", "left");
        legend_label_75.append(Tekst::new(tick_label(legend_75)))
        ;
    let mut legend_label_50 = Text::new()
        .set("x", WIDTH + LEGEND_GRADIENT_WIDTH + LEGEND_GRADIENT_MARGIN*2.0)
        .set("y", TICK_FIRST_Y + TICK_HEIGHT_DELTA*2.0)
        .set("font-family", "serif")
        .set("font-size", tick_font_size.clone())
        .set("text-anchor", "left");
        legend_label_50.append(Tekst::new(tick_label(legend_50)))
        ;
    let mut legend_label_25 = Text::new()
        .set("x", WIDTH + LEGEND_GRADIENT_WIDTH + LEGEND_GRADIENT_MARGIN*2.0)
        .set("y", TICK_FIRST_Y + TICK_HEIGHT_DELTA*3.0)
        .set("font-family", "serif")
        .set("font-size", tick_font_size.clone())
        .set("text-anchor", "left");
        legend_label_25.append(Tekst::new(tick_label(legend_25)))
        ;
    let mut legend_label_0 = Text::new()
        .set("x", WIDTH + LEGEND_GRADIENT_WIDTH + LEGEND_GRADIENT_MARGIN*2.0)
        .set("y", -4.0 + TICK_FIRST_Y + TICK_HEIGHT_DELTA*4.0)
        .set("font-family", "serif")
        .set("font-size", tick_font_size.clone())
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
            ColourMode::DpVar   => format!("var({})",   plot_info.dp_desc),
            ColourMode::DpUniq  => format!("uniq({})",  plot_info.dp_desc),
            ColourMode::DpSum   => format!("sum({})",   plot_info.dp_desc),
            _   =>  "todo".to_string(), //colour_mode
        };

    let mut legend_dp_desc = Text::new()
        .set("x", WIDTH + LEGEND_GRADIENT_MARGIN*1.0)
        .set("y", LABEL_DP_DESC_HEIGHT - 2.0)
        .set("font-family", "serif")
        .set("font-size", tick_font_size.clone());
        //.set("alignment-baseline", "hanging"); // this does not work in firefox
        legend_dp_desc.append(Tekst::new(dp_desc_text))
        ;
    
    legend_g.append(legend_dp_desc);

    (defs, legend_g)
}

pub fn tick_label(v: f64) -> String {

    match v as u64 {
        i if i > 1_000_000 => format!("{:.0}M", (i/1_000_000)), //.to_string()
        1000...999_999 => format!("{:.0}K", (v/1000.0)), //.to_string()
        _ => format!("{:.0}", v)
    }
}
