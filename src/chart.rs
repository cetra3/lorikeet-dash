use anyhow::Error;

use serde::{Deserialize, Serialize};

use std::collections::HashMap;

use tera::{Context, Tera, Value};

use lazy_static::lazy_static;
use std::cmp;

lazy_static! {
    static ref TERA: Tera = {
        let mut tera = Tera::default();
        tera.register_filter("pretty", pretty);
        tera.add_raw_template("chart", include_str!("chart.svg"))
            .expect("Could not add template");

        tera
    };
}

#[derive(Clone, Debug)]
pub struct Chart {
    pub name: String,
    pub points: Vec<Point>,
    pub units: ChartUnits,
    pub smooth: bool,
    pub colour: String
}

impl Default for ChartUnits {
    fn default() -> Self {
        ChartUnits::Value
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Copy)]
#[serde(rename_all = "lowercase")]
pub enum ChartUnits {
    Value,
    KiloBytes,
    Seconds,
}

impl Chart {
    pub fn add_point(&mut self, x: f64, y: f64) {
        self.points.push(Point { x, y });
    }

    pub fn draw_svg(&self, width: usize, height: usize) -> Result<String, Error> {

        if self.points.len() <= 1 {
            return Ok(include_str!("loading.svg").into())
        }


        let mut context = Context::new();

        let min_x = self.points.get(0).map(|val| val.x).unwrap_or(0.0);
        let max_x = self
            .points
            .iter()
            .map(|val| (val.x - min_x))
            .fold(0. / 0., f64::max);
        let min_y = self.points.iter().map(|val| val.y).fold(0. / 0., f64::min);
        let max_y = self
            .points
            .iter()
            .map(|val| (val.y - min_y))
            .fold(0. / 0., f64::max);

        let max_label = pretty_internal(max_y + min_y, self.units);
        
        //sometimes the min_y label is larger
        let min_label = pretty_internal(min_y, self.units);

        //left padding changes based upon how many chars are, using a *really* rough 1 char * 9px;
        let p_left = (max_label.len() as f64 * 9.0 + 5.0).max(min_label.len() as f64 * 9.0 + 5.0).max(50.0);

        let p_other = 50.0;

        let width = width as f64 - p_left - p_other;
        let height = height as f64 - p_other * 2.0;

        let points: Vec<Point> = self
            .points
            .iter()
            .map(|val| Point {
                x: ((val.x - min_x) / max_x * width) + p_left,
                y: ((val.y - min_y) / max_y * (height * -1.0)) + p_other as f64 + height as f64,
            })
            .collect();

        let path = if self.smooth {
            catmull_bezier(points)
                .iter()
                .map(|val| val.to_svg())
                .collect::<Vec<String>>()
                .join("")
        } else {
            points
                .iter()
                .map(|val| val.to_svg())
                .collect::<Vec<String>>()
                .join("")
        };

        context.insert("path", &path);
        context.insert("name", &self.name);
        context.insert("width", &(width));
        context.insert("height", &(height));
        context.insert("p_left", &p_left);
        context.insert("p_other", &p_other);
        context.insert("max_y", &max_y);
        context.insert("min_y", &min_y);
        context.insert("max_x", &max_x);
        context.insert("min_x", &min_x);
        context.insert("units", &self.units);
        context.insert("lines", &5);
        context.insert("colour", &self.colour);

        Ok(TERA.render("chart", &context)?)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Copy)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    fn to_svg(&self) -> String {
        format!("L {} {}", self.x, self.y)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Curve {
    c1: Point,
    c2: Point,
    end: Point,
}

impl Curve {
    fn to_svg(&self) -> String {
        format!(
            "C {:.4} {:.4}, {:.4} {:.4}, {:.4} {:.4}",
            self.c1.x, self.c1.y, self.c2.x, self.c2.y, self.end.x, self.end.y
        )
    }
}

fn catmull_bezier(points: Vec<Point>) -> Vec<Curve> {
    let mut res = Vec::new();

    let last = points.len() - 1;

    for i in 0..last {
        let p0 = if i == 0 { points[0] } else { points[i - 1] };

        let p1 = points[i];

        let p2 = points[i + 1];

        let p3 = if i + 2 > last {
            points[i + 1]
        } else {
            points[i + 2]
        };

        let c1 = Point {
            x: ((-p0.x + 6.0 * p1.x + p2.x) / 6.0),
            y: ((-p0.y + 6.0 * p1.y + p2.y) / 6.0),
        };

        let c2 = Point {
            x: ((p1.x + 6.0 * p2.x - p3.x) / 6.0),
            y: ((p1.y + 6.0 * p2.y - p3.y) / 6.0),
        };

        let end = p2;

        res.push(Curve { c1, c2, end });
    }

    return res;
}

fn pretty_internal(val: f64, unit_type: ChartUnits) -> String {
    match unit_type {
        ChartUnits::Value => format!("{:.2}", val),
        ChartUnits::KiloBytes => pretty_bytes(val * 1024.0),
        ChartUnits::Seconds => format!("{:.0}ms", val * 1000.0),
    }
}

pub fn pretty(value: &Value, args: &HashMap<String, Value>) -> tera::Result<Value> {
    let unit_type: ChartUnits = args
        .get("format")
        .and_then(|val| serde_json::from_value(val.clone()).ok())
        .unwrap_or_default();

    let val = match value.as_f64() {
        None => return Err(tera::Error::msg("Could not convert as number")),
        Some(val) => val,
    };

    return Ok(pretty_internal(val, unit_type).into());
}

pub fn pretty_bytes(num: f64) -> String {
    let negative = if num.is_sign_positive() { "" } else { "-" };
    let num = num.abs();
    let units = ["B", "KB", "MB", "GB", "TB", "PB", "EB", "ZB", "YB"];
    if num < 1_f64 {
        return format!("{}{} {}", negative, num, "B");
    }
    let delimiter = 1000_f64;
    let exponent = cmp::min(
        (num.ln() / delimiter.ln()).floor() as i32,
        (units.len() - 1) as i32,
    );

    let unit = units[exponent as usize];
    return format!("{}{:.2}{}", negative, num / delimiter.powi(exponent), unit);
}
