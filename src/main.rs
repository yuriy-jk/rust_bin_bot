extern crate chrono;
extern crate job_scheduler;
extern crate ta_lib_wrapper;
use chrono::prelude::*;
use job_scheduler::{Job, JobScheduler};
use std::env;
use std::time::Duration;
mod api;
mod indic_compute;
mod pnl;

fn get_start_position(
    client: &reqwest::blocking::Client,
    pos: &mut &str,
    tiker: &str,
    interval: &str,
    limit: &u32,
    sar_accel: &f64,
    sar_max: &f64,
) {
    let klines = api::get_klines_struct(&client, &tiker, &interval, &limit).unwrap();
    let (sar_values, _begin) = indic_compute::sar(sar_accel, sar_max, &klines.high, &klines.low);
    // LAST VALUES OF OPEN_PRICE AND SAR_INDICATOR
    let open = klines.open[klines.open.len() - 1];
    let sar = sar_values[sar_values.len() - 1];
    if open > sar {
        *pos = "buy"
    } else if open < sar {
        *pos = "sell"
    }
}

fn main() {
    log4rs::init_file("log4rs.yml", Default::default()).unwrap(); // init log4rs config from .yml

    // ENV VARS init
    dotenv::dotenv().ok();
    let tiker = env::var("TIKER").unwrap();
    let interval = env::var("TIMEFRAME").unwrap();
    let klines_count = env::var("KLINES").unwrap().parse::<u32>().unwrap();
    let sar_accel = env::var("ACCEL").unwrap().parse::<f64>().unwrap();
    let sar_max = env::var("MAX").unwrap().parse::<f64>().unwrap();
    let wma_period = env::var("PERIOD").unwrap().parse::<i32>().unwrap();
    // CLIENT API init
    let client = api::get_client();
    let curr_price: f64 = api::get_curr_price(&client, &tiker);
    log::info!("{}", curr_price);

    // TRADE VARS
    let mut pos: &str = " ";
    let mut size: f64 = 0.0;
    let mut profit: f64 = 0.0;
    let mut offset: i32 = 0;

    get_start_position(
        &client,
        &mut pos,
        &tiker,
        &interval,
        &klines_count,
        &sar_accel,
        &sar_max,
    );
    log::debug!("Start_position {}", pos);

    // SCHEDULER init
    let mut sched = JobScheduler::new();
    sched.add(Job::new("10 0,5/5 * * * *".parse().unwrap(), || {
        start_bot(
            &client,
            &mut pos,
            &mut size,
            &mut profit,
            &tiker,
            &interval,
            &klines_count,
            &sar_accel,
            &sar_max,
            &wma_period,
            &mut offset,
        );
    }));
    loop {
        sched.tick();

        std::thread::sleep(Duration::from_millis(500));
    }
}

fn start_bot(
    client: &reqwest::blocking::Client,
    pos: &mut &str,
    size: &mut f64,
    profit: &mut f64,
    tiker: &str,
    interval: &str,
    limit: &u32,
    sar_accel: &f64,
    sar_max: &f64,
    wma_period: &i32,
    offset: &mut i32,
) {
    let klines = match api::get_klines_struct(&client, &tiker, &interval, &limit) {
        Some(klines) => klines,
        None => return log::error!("Got error from get_klines function"),
    };
    let (sar_values, _) = indic_compute::sar(sar_accel, sar_max, &klines.high, &klines.low);
    let (wma_values, _) = indic_compute::wma(wma_period, &klines.close);
    // LAST VALUES OF KLINES AND INDICS
    let open: f64 = klines.open[klines.open.len() - 1];
    let sar: f64 = sar_values[sar_values.len() - 1];

    let low: f64 = klines.low[klines.low.len() - 2];
    let high: f64 = klines.high[klines.high.len() - 2];
    let wma: f64 = wma_values[wma_values.len() - 2];
    // TRADE CONST
    let summ: f64 = 100.0;
    let mut position_profit: f64 = 0.0;
    let date = Utc.timestamp_millis(
        klines.timestamp[klines.timestamp.len() - 1]
            .try_into()
            .unwrap(),
    );

    let profit_line: f64 = wma + (wma - sar_values[sar_values.len() - 2]);
    log::info!(
        "{} - open={} - sar={:.5} - pos={} - {} - LastStep - p_wma={:.5} - high={} - low={}",
        date.format("%Y-%m-%d %H:%M:%S"),
        &open,
        &sar,
        pos,
        Utc::now(),
        &profit_line,
        &high,
        &low
    );

    let curr_price: f64 = api::get_curr_price(&client, &tiker);
    if *pos == "sell" {
        if *size > 0.0 {
            if open > sar {
                position_profit = summ - (curr_price * *size);
                *profit += position_profit;
                *pos = "buy";
                log::info!("{}", pos);
                *size = summ / curr_price;
                log::info!(
                    "{} - {} - {:.5} - {:.5} - {}",
                    date.format("%Y-%m-%d %H:%M:%S"),
                    &curr_price,
                    &position_profit,
                    &profit,
                    &size
                );
                *offset = 1;
            } else if low < profit_line && *offset == 0 {
                position_profit = summ - (curr_price * *size);
                *profit += position_profit;
                *size = 0.0;
                log::info!(
                    "{} - {} - {:.5} - {:.5} close by WMA signal",
                    date.format("%Y-%m-%d %H:%M:%S"),
                    &curr_price,
                    &position_profit,
                    &profit
                );
            } else {
                *offset = 0;
            }
        } else if *size == 0.0 && open > sar {
            *pos = "buy";
            log::info!("{}", pos);
            *size = summ / curr_price;
            log::info!(
                "{} - {} - {}",
                date.format("%Y-%m-%d %H:%M:%S"),
                &curr_price,
                &size
            );
            *offset = 1;
        }
    } else if *pos == "buy" {
        if *size > 0.0 {
            if open < sar {
                position_profit = (curr_price * *size) - 100.0;
                *profit += position_profit;
                *pos = "sell";
                log::info!("{}", pos);
                *size = summ / curr_price;
                log::info!(
                    "{} - {} - {:.5} - {:.5} - {}",
                    date.format("%Y-%m-%d %H:%M:%S"),
                    &curr_price,
                    &position_profit,
                    &profit,
                    &size
                );
                *offset = 1;
            } else if high > profit_line && *offset == 0 {
                position_profit = (curr_price * *size) - 100.0;
                *profit += position_profit;
                *size = 0.0;
                log::info!(
                    "{} - {} - {:.5} - {:.5} close by WMA signal",
                    date.format("%Y-%m-%d %H:%M:%S"),
                    &curr_price,
                    &position_profit,
                    &profit
                );
            } else {
                *offset = 0;
            }
        } else if *size == 0.0 && open < sar {
            *pos = "sell";
            log::info!("{}", pos);
            *size = summ / curr_price;
            log::info!(
                "{} - {} - {}",
                date.format("%Y-%m-%d %H:%M:%S"),
                &curr_price,
                &size
            );
            *offset = 1;
        }
    }

    // if open > sar && *pos == "sell" {
    //     if *size > 0.0 {
    //         position_profit = summ - (open * *size);
    //         *profit += position_profit;
    //     }
    //     *pos = "buy";
    //     println!("{}", pos);
    //     *size = summ / open;

    //     println!(
    //         "{} - {} - {} - {} - {}",
    //         date.format("%Y-%m-%d %H:%M:%S"),
    //         &open,
    //         &position_profit,
    //         &profit,
    //         &size
    //     );
    // } else if open < sar && *pos == "buy" {
    //     if *size > 0.0 {
    //         position_profit = (open * *size) - 100.0;
    //         *profit += position_profit;
    //     }
    //     *pos = "sell";
    //     println!("{}", pos);
    //     *size = summ / open;
    //     println!(
    //         "{} - {} - {} - {} - {}",
    //         date.format("%Y-%m-%d %H:%M:%S"),
    //         &open,
    //         &position_profit,
    //         &profit,
    //         &size
    //     );
    // }
}
