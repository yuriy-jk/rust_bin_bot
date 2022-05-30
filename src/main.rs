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

const ACCEL: f64 = 0.021;
const MAX: f64 = 0.088;
const PERIOD: u32 = 9;

fn get_start_position(client: &reqwest::blocking::Client, pos: &mut &str, 
    tiker: &str,
    interval: &str,
    limit: &u32,) {
    let klines = api::get_klines_struct(&client, &tiker, &interval, &limit).unwrap();
    let (sar_values, _begin) = indic_compute::sar(ACCEL, MAX, &klines.high, &klines.low);
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
    dotenv::dotenv().ok();
    println!("key: {}", env::var("BINANCE_API_KEY").unwrap());
    let tiker = env::var("tiker").unwrap();
    let interval = env::var("timeframe").unwrap();
    let klines_count = env::var("klines").unwrap().parse::<u32>().unwrap();
    let client = api::get_client();

    // // api::get_balance(&client);
    // api::get_server_time(&client);
    // TRADE VARS
    let mut pos: &str = " ";
    let mut size: f64 = 0.0;
    let mut profit: f64 = 0.0;
    get_start_position(&client, &mut pos, &tiker, &interval, &klines_count);
    println!("Start_position {}", pos);

    let mut sched = JobScheduler::new();
    sched.add(Job::new("10 0,5/5 * * * *".parse().unwrap(), || {
        start_bot(&client, &mut pos, &mut size, &mut profit, &tiker, &interval, &klines_count);
    }));
    loop {
        sched.tick();

        std::thread::sleep(Duration::from_millis(500));
    }
}

fn start_bot(client: &reqwest::blocking::Client, pos: &mut &str, size: &mut f64, profit: &mut f64, tiker: &str,
    interval: &str,
    limit: &u32,) {
    let klines = match api::get_klines_struct(&client, &tiker, &interval, &limit){
        Some(klines) => klines,
        None => return println!("Got error from get_klines function")
    };
    let (sar_values, sar_begin) = indic_compute::sar(ACCEL, MAX, &klines.high, &klines.low);
    let (wma_values, wma_begin) = indic_compute::wma(PERIOD, &klines.close);
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
    println!(
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
                println!("{}", pos);
                *size = summ / curr_price;
                println!(
                    "{} - {} - {:.5} - {:.5} - {}",
                    date.format("%Y-%m-%d %H:%M:%S"),
                    &open,
                    &position_profit,
                    &profit,
                    &size
                );
            } else if low < profit_line {
                position_profit = summ - (curr_price * *size);
                *profit += position_profit;
                *size = 0.0;
                println!(
                    "{} - {} - {:.5} - {:.5} close by WMA signal",
                    date.format("%Y-%m-%d %H:%M:%S"),
                    &open,
                    &position_profit,
                    &profit
                );
            }
        } else if *size == 0.0 && open > sar {
            *pos = "buy";
            println!("{}", pos);
            *size = summ / curr_price;
            println!(
                "{} - {} - {}",
                date.format("%Y-%m-%d %H:%M:%S"),
                &open,
                &size
            )
        }
    } else if *pos == "buy" {
        if *size > 0.0 {
            if open < sar {
                position_profit = (curr_price * *size) - 100.0;
                *profit += position_profit;
                *pos = "sell";
                println!("{}", pos);
                *size = summ / curr_price;
                println!(
                    "{} - {} - {:.5} - {:.5} - {}",
                    date.format("%Y-%m-%d %H:%M:%S"),
                    &open,
                    &position_profit,
                    &profit,
                    &size
                );
            } else if high > profit_line {
                position_profit = (curr_price * *size) - 100.0;
                *profit += position_profit;
                *size = 0.0;
            }
        } else if *size == 0.0 && open < sar {
            *pos = "sell";
            println!("{}", pos);
            *size = summ / curr_price;
            println!(
                "{} - {} - {}",
                date.format("%Y-%m-%d %H:%M:%S"),
                &open,
                &size
            );
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

