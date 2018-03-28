use ipnetwork::Ipv6Network;
use std::net::Ipv6Addr;


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
    pub asn:    u32,
    pub hits: Vec<Ipv6Addr>,
}

impl Route {
    pub fn size(&self) -> u128 {
        //FIXME: this is just a workaround.. is there a better way to do this?
        let mut exp = self.prefix.prefix() as u32;
        if exp < 24 {
            exp = 24;
        }
        //let r = 2_u128.pow(128 - self.prefix.prefix() as u32);
        let r = 2_u128.pow(128 - exp);
        r
    }

    pub fn to_string(&self) -> String {
        format!("AS{}", &self.asn)
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
