use log::*;
use std::env;
use structopt::StructOpt;

use anyhow::Error;

use lorikeet::{
    runner::run_steps,
    step::{RunType, SystemVariant},
    yaml::get_steps,
};

use std::path::PathBuf;

use chrono::prelude::*;
use lazy_static::lazy_static;
use std::{collections::HashMap, time::Duration};
use tokio::sync::RwLock;
use tokio::time::delay_for;

use actix_web::{web, App, HttpResponse, HttpServer, HttpRequest};

use conduit_mime_types::Types;

mod chart;

use chart::{Chart, ChartUnits};

#[derive(StructOpt, Debug)]
#[structopt(name = "lorikeet-dash", about = "a web dashboard for lorikeet")]
struct Arguments {
    #[structopt(
        short = "c",
        long = "config",
        help = "Configuration File",
        parse(from_os_str)
    )]
    config: Option<PathBuf>,

    #[structopt(help = "Test Plan", default_value = "test.yml", parse(from_os_str))]
    test_plan: PathBuf,

    #[structopt(
        short = "l",
        long = "listen",
        default_value = "0.0.0.0:3333",
        help = "Listen Address"
    )]
    listen: String,

    #[structopt(
        short = "r",
        long = "refresh_ms",
        default_value = "10000",
        help = "Refresh Interval"
    )]
    refresh_ms: u64,
}

lazy_static! {
    static ref CHARTS: RwLock<Vec<Chart>> = RwLock::new(Vec::new());
    static ref TYPES: Types = Types::new().expect("Could not load mime types");
}

static COLOURS: [&'static str; 7] = [
    "#99c1f1",
    "#8ff0a4",
    "#f9f06b",
    "#ffbe6f",
    "#f66151",
    "#dc8add",
    "#cdab8f"
]; 

#[actix_rt::main]
async fn main() -> Result<(), Error> {
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "actix_web=DEBUG,lorikeet_dash=DEBUG");
    }

    pretty_env_logger::init_timed();

    let args = Arguments::from_args();

    let mut steps = get_steps(&args.test_plan, &args.config)?;

    {
        let mut charts = CHARTS.write().await;

        for (i, step) in steps.iter().enumerate() {
            let units = match step.run {
                RunType::Http(_) => ChartUnits::Seconds,
                RunType::System(ref sys_type) => match sys_type {
                    SystemVariant::LoadAvg1m
                    | SystemVariant::LoadAvg5m
                    | SystemVariant::LoadAvg15m => ChartUnits::Value,
                    _ => ChartUnits::KiloBytes,
                },
                _ => ChartUnits::Value,
            };

            charts.push(Chart {
                name: step.name.clone(),
                points: Vec::new(),
                units,
                smooth: false,
                colour: COLOURS[i % 7].to_string()
            });
        }
    }

    let delay = Duration::from_millis(args.refresh_ms);

    let offset = Local.timestamp(0, 0).offset().fix().local_minus_utc();

    tokio::spawn(async move {
        loop {
            let time = Local::now().timestamp();

            if let Err(err) = run_steps(&mut steps).await {
                error!("Error running steps:{}", err);
            } else {
                for (i, chart) in CHARTS.write().await.iter_mut().enumerate() {
                    if let Some(ref outcome) = steps[i].outcome {
                        if let Some(number) =
                            outcome.output.as_ref().and_then(|val| val.parse().ok())
                        {
                            chart.add_point((time + offset as i64) as f64, number);
                        } else {
                            chart.add_point(
                                (time + offset as i64) as f64,
                                outcome.duration.as_secs() as f64
                                    + (outcome.duration.subsec_millis() as f64) / 1000.0,
                            );
                        }
                    }
                }
            }

            let run_time = Local::now().timestamp() - time;

            if run_time < delay.as_secs() as i64 {
                delay_for(delay - Duration::from_secs(run_time as u64)).await;
            }
        }
    });

    info!("Listening on `{}`", args.listen);

    HttpServer::new(move || {
        App::new()
            // register simple handler
            .service(web::resource("/charts").to(charts))
            .service(web::resource("/charts/{name}").to(get_chart))
            .service(web::resource("{tail:.*}").to(front))
    })
    .bind(&args.listen)?
    .run()
    .await?;

    Ok(())
}

include!(concat!(env!("OUT_DIR"), "/data.rs"));

async fn front(req: HttpRequest) -> HttpResponse {

    let request_path = req.uri().path();

    let mut relative_path = format!("front/dist{}", request_path);

    if request_path == "" || request_path.ends_with("/") {
            relative_path.push_str("index.html");
    }

    match FILES.get(&relative_path) {
            Ok(file) => {
                let mime_type =
                    String::from(TYPES.mime_for_path(&PathBuf::from(&relative_path)));

               return HttpResponse::Ok()
                    .content_type(mime_type)
                    .body(file.into_owned())
            }
            Err(_) => {
                //SPA
                if !request_path.starts_with("/api/") {
                    if let Ok(index_page) = FILES.get("front/dist/index.html") {
                        return HttpResponse::Ok()
                            .content_type("text/html")
                            .body(index_page.into_owned());
                    }
                }

                return HttpResponse::NotFound().finish();
            }
    }


}


async fn charts() -> HttpResponse {
    let response: Vec<String> = CHARTS
        .read()
        .await
        .iter()
        .map(|chart| chart.name.clone())
        .collect();

    HttpResponse::Ok().json(&response) // <- send response
}

async fn get_chart(
    query: web::Query<HashMap<String, String>>,
    path: web::Path<(String,)>,
) -> HttpResponse {
    let name = path.0.replace(".svg", "");

    let width = query
        .get("width")
        .and_then(|val| val.parse().ok())
        .unwrap_or(800);
    let height = query
        .get("height")
        .and_then(|val| val.parse().ok())
        .unwrap_or(500);

    if let Some(svg) = CHARTS
        .read()
        .await
        .iter()
        .find(|chart| chart.name == name)
        .map(|chart| chart.draw_svg(width, height))
    {
        match svg {
            Ok(svg) => {
                return HttpResponse::Ok().content_type("image/svg+xml").body(svg);
                // <- send response
            }
            Err(err) => {
                error!("Error:{:?}", err);

                return HttpResponse::InternalServerError().body(err.to_string());
                // <- send response
            }
        }
    } else {
        return HttpResponse::NotFound().finish();
    }
}
