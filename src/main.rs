extern crate chrono;
extern crate job_scheduler;
extern crate ta_lib_wrapper;
mod best_params;
use chrono::prelude::*;
use job_scheduler::{Job, JobScheduler};
use std::env;
use std::sync::{Arc, Mutex};
use std::thread;
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
    best_thread_params: &Arc<Mutex<best_params::BestParams>>,
) {
    let params = best_thread_params.lock().unwrap();
    let klines = api::get_klines_struct(&client, &tiker, &interval, &limit).unwrap();
    let (sar_values, _begin) =
        indic_compute::sar(&params.accel, &params.max, &klines.high, &klines.low);
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
                                                                  // INIT ENV VARS
    dotenv::dotenv().ok();
    let tiker = env::var("TIKER").unwrap();
    let interval = env::var("TIMEFRAME").unwrap();
    let klines_count = env::var("KLINES").unwrap().parse::<u32>().unwrap();
    // let sar_accel = env::var("ACCEL").unwrap().parse::<f64>().unwrap();
    // let sar_max = env::var("MAX").unwrap().parse::<f64>().unwrap();
    // let wma_period = env::var("PERIOD").unwrap().parse::<i32>().unwrap();
    // INIT CLIENT API
    let client = api::get_client();
    let curr_price: f64 = api::get_curr_price(&client, &tiker);
    log::info!("{}", curr_price);
    // INIT BEST PARAMS
    let best_thread_profit = Arc::new(Mutex::new(0.0));
    let best_thread_params = Arc::new(Mutex::new(best_params::BestParams {
        profit: 0.0,
        profit_trade: 0,
        loss_trade: 0,
        accel: 0.0,
        max: 0.0,
        period: 0,
        pnl: "best".to_string(),
    }));

    best_params::count_best_params(
        &client,
        &tiker,
        &interval,
        &klines_count,
        &best_thread_profit,
        &best_thread_params,
    );

    // TRADE VARS
    let mut summ: f64 = 100.0;
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
        &best_thread_params,
    );
    log::debug!("Start_position {}", pos);

    // SCHEDULER init
    let mut sched = JobScheduler::new();
    sched.add(Job::new("10 0,5/5 * * * *".parse().unwrap(), || {
        start_bot(
            &client,
            &mut summ,
            &mut pos,
            &mut size,
            &mut profit,
            &tiker,
            &interval,
            &klines_count,
            &best_thread_params,
            &mut offset,
        );
    }));
    sched.add(Job::new("0 2 1/4 * * *".parse().unwrap(), || {
        best_params::count_best_params(
            &client,
            &tiker,
            &interval,
            &klines_count,
            &best_thread_profit,
            &best_thread_params,
        );
    }));
    loop {
        sched.tick();

        thread::sleep(Duration::from_millis(500));
    }
}

fn start_bot(
    client: &reqwest::blocking::Client,
    summ: &mut f64,
    pos: &mut &str,
    size: &mut f64,
    profit: &mut f64,
    tiker: &str,
    interval: &str,
    limit: &u32,
    best_thread_params: &Arc<Mutex<best_params::BestParams>>,
    offset: &mut i32,
) {
    let klines = match api::get_klines_struct(&client, &tiker, &interval, &limit) {
        Some(klines) => klines,
        None => return log::error!("Got error from get_klines function"),
    };
    let params = best_thread_params.lock().unwrap();
    log::info!("Best Params acc={}, max={}", &params.accel, &params.max);
    let (sar_values, _) = indic_compute::sar(&params.accel, &params.max, &klines.high, &klines.low);
    let (wma_values, _) = indic_compute::wma(&params.period, &klines.close);
    // LAST VALUES OF KLINES AND INDICS
    let open: f64 = klines.open[klines.open.len() - 1];
    let sar: f64 = sar_values[sar_values.len() - 1];

    let low: f64 = klines.low[klines.low.len() - 2];
    let high: f64 = klines.high[klines.high.len() - 2];
    let wma: f64 = wma_values[wma_values.len() - 2];
    // TRADE CONST

    let mut position_profit: f64 = 0.0;
    let date = Utc.timestamp_millis(
        klines.timestamp[klines.timestamp.len() - 1]
            .try_into()
            .unwrap(),
    );

    let profit_line: f64 = wma + (wma - sar_values[sar_values.len() - 2]);
    log::info!(
        "{} - open={} - sar={:.5} - pos={} - summ={} - {} - LastStep - p_wma={:.5} - high={} - low={}",
        date.format("%Y-%m-%d %H:%M:%S"),
        &open,
        &sar,
        pos,
        &summ,
        Utc::now(),
        &profit_line,
        &high,
        &low
    );

    let curr_price: f64 = api::get_curr_price(&client, &tiker);
    if *pos == "sell" {
        if *size > 0.0 {
            if open > sar {
                position_profit = *summ - (curr_price * *size);
                *summ += position_profit;
                *profit += position_profit;
                *pos = "buy";
                log::info!("{}", pos);
                *size = *summ / curr_price;
                log::info!(
                    "{} - price={} - pos_profit={:.5} - profit={:.5} - size={} - summ={}",
                    date.format("%Y-%m-%d %H:%M:%S"),
                    &curr_price,
                    &position_profit,
                    &profit,
                    &size,
                    &summ
                );
                *offset = 1;
            } else if low < profit_line && *offset == 0 {
                position_profit = *summ - (curr_price * *size);
                *summ += position_profit;
                *profit += position_profit;
                *size = 0.0;
                log::info!(
                    "{} - price={} - pos_profit={:.5} - profit={:.5} - summ={} close by WMA signal",
                    date.format("%Y-%m-%d %H:%M:%S"),
                    &curr_price,
                    &position_profit,
                    &profit,
                    &summ
                );
            } else {
                *offset = 0;
            }
        } else if *size == 0.0 && open > sar {
            *pos = "buy";
            log::info!("{}", pos);
            *size = *summ / curr_price;
            log::info!(
                "{} - price={} - size={} - summ={}",
                date.format("%Y-%m-%d %H:%M:%S"),
                &curr_price,
                &size,
                &summ
            );
            *offset = 1;
        }
    } else if *pos == "buy" {
        if *size > 0.0 {
            if open < sar {
                position_profit = (curr_price * *size) - *summ;
                *summ += position_profit;
                *profit += position_profit;
                *pos = "sell";
                log::info!("{}", pos);
                *size = *summ / curr_price;
                log::info!(
                    "{} - price={} - pos_profit={:.5} - profit={:.5} - size={} - summ={}",
                    date.format("%Y-%m-%d %H:%M:%S"),
                    &curr_price,
                    &position_profit,
                    &profit,
                    &size,
                    &summ
                );
                *offset = 1;
            } else if high > profit_line && *offset == 0 {
                position_profit = (curr_price * *size) - *summ;
                *summ += position_profit;
                *profit += position_profit;
                *size = 0.0;
                log::info!(
                    "{} - price={} - pos_profit={:.5} - profit={:.5} - summ={} close by WMA signal",
                    date.format("%Y-%m-%d %H:%M:%S"),
                    &curr_price,
                    &position_profit,
                    &profit,
                    &summ
                );
            } else {
                *offset = 0;
            }
        } else if *size == 0.0 && open < sar {
            *pos = "sell";
            log::info!("{}", pos);
            *size = *summ / curr_price;
            log::info!(
                "{} - price={} - size={} - summ={}",
                date.format("%Y-%m-%d %H:%M:%S"),
                &curr_price,
                &size,
                &summ
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
