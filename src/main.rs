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

fn get_start_position(client: &reqwest::blocking::Client, pos: &mut &str, 
    tiker: &str,
    interval: &str,
    limit: &u32,) {
    let klines = api::get_klines_struct(&client, &tiker, &interval, &limit).unwrap();
    let (sar_values, _begin) = indic_compute::sar(0.019, 0.096, &klines.high, &klines.low);
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
    let interval = env::var("interval").unwrap();
    let klines_count = env::var("klines").unwrap().parse::<u32>().unwrap();
    // let client = api::get_client();

    // // api::get_balance(&client);
    // api::get_server_time(&client);

    // // TRADE VARS
    // let mut pos: &str = " ";
    // let mut size: f64 = 0.0;
    // let mut profit: f64 = 0.0;
    // get_start_position(&client, &mut pos, &tiker, &interval, &klines_count);
    // println!("Start_position {}", pos);

    // let mut sched = JobScheduler::new();
    // sched.add(Job::new("10 0,5/5 * * * *".parse().unwrap(), || {
    //     start_bot(&client, &mut pos, &mut size, &mut profit, &tiker, &interval, &klines_count);
    // }));
    // loop {
    //     sched.tick();

    //     std::thread::sleep(Duration::from_millis(500));
    // }

    let client = api::get_client();
    let klines = match api::get_klines_struct(&client, &tiker, &interval, &klines_count){
        Some(klines) => klines,
        None => return println!("Got error from get_klines function")
    };
    // let (sar_values, _begin) = indic_compute::sar(0.006, 0.023, &klines.high, &klines.low);
    // let (wma_values, wma_begin) = indic_compute::wma(17, &klines.close);
    // let (profit_trade, loss_trade, profit) =
    //                 pnl::count_pnl(
    //                 &klines.timestamp[wma_begin as usize..klines.timestamp.len()].to_vec(),
    //                 &klines.open[wma_begin as usize..klines.open.len()].to_vec(), 
    //                 &klines.high[wma_begin as usize..klines.high.len()].to_vec(), 
    //                 &klines.low[wma_begin as usize..klines.low.len()].to_vec(), 
    //                 &sar_values[wma_begin as usize-1..sar_values.len()].to_vec(), 
    //                 &wma_values);
    // println!(
    //     "{}-{} - {}",
    //     profit_trade, loss_trade, profit
    // )
    count_bestprofit(&klines)
}

fn start_bot(client: &reqwest::blocking::Client, pos: &mut &str, size: &mut f64, profit: &mut f64, tiker: &str,
    interval: &str,
    limit: &u32,) {
    let klines = match api::get_klines_struct(&client, &tiker, &interval, &limit){
        Some(klines) => klines,
        None => return println!("Got error from get_klines function")
    };
    // let (rsi_values, _begin) = indic_compute::rsi(12, &klines.close);
    let (sar_values, _begin) = indic_compute::sar(0.019, 0.096, &klines.high, &klines.low);

    // LAST VALUES OF OPEN_PRICE AND SAR_INDICATOR
    let open: f64 = klines.open[klines.open.len() - 1];
    let sar: f64 = sar_values[sar_values.len() - 1];

    // TRADE CONST
    let summ: f64 = 100.0;
    let mut position_profit: f64 = 0.0;
    let date = Utc.timestamp_millis(
        klines.timestamp[klines.timestamp.len() - 1]
            .try_into()
            .unwrap(),
    );
    println!(
        "{} - {} - {} - {} - {}",
        date.format("%Y-%m-%d %H:%M:%S"),
        &open,
        &sar,
        pos,
        Utc::now()
    );
    if open > sar && *pos == "sell" {
        if *size > 0.0 {
            position_profit = summ - (open * *size);
            *profit += position_profit;
        }
        *pos = "buy";
        println!("{}", pos);
        *size = summ / open;

        println!(
            "{} - {} - {} - {} - {}",
            date.format("%Y-%m-%d %H:%M:%S"),
            &open,
            &position_profit,
            &profit,
            &size
        );
    } else if open < sar && *pos == "buy" {
        if *size > 0.0 {
            position_profit = (open * *size) - 100.0;
            *profit += position_profit;
        }
        *pos = "sell";
        println!("{}", pos);
        *size = summ / open;
        println!(
            "{} - {} - {} - {} - {}",
            date.format("%Y-%m-%d %H:%M:%S"),
            &open,
            &position_profit,
            &profit,
            &size
        );
    }
}

fn count_bestprofit(klines: &api::Kline) {
    let mut best_profit: f64 = 0.0;
    for period in 7..22 {
        for accel in (1..900).map(|x| x as f64 * 0.001) {
            for max in (1..900).map(|x| x as f64 * 0.001) {
                let (sar_values, sar_begin) = indic_compute::sar(accel, max, &klines.high, &klines.low);
                let (wma_values, wma_begin) = indic_compute::wma(period, &klines.close);
                // println!("sar={}-{}, wma={}-{}, open={}", sar_values.len(), sar_begin,  wma_values.len(), wma_begin, klines.open.len());
                let (profit_trade, loss_trade, profit) =
                    pnl::count_pnl(
                    &klines.timestamp[wma_begin as usize..klines.timestamp.len()].to_vec(),
                    &klines.open[wma_begin as usize..klines.open.len()].to_vec(), 
                    &klines.high[wma_begin as usize..klines.high.len()].to_vec(), 
                    &klines.low[wma_begin as usize..klines.low.len()].to_vec(), 
                    &sar_values[wma_begin as usize-1..sar_values.len()].to_vec(), 
                    &wma_values);
                if profit > best_profit {
                    best_profit = profit;
                    println!(
                        "{}-{} - {} - {} - {} - {}",
                        profit_trade, loss_trade, best_profit, period, &accel, &max
                    )
                }
            }
        }
    }
}
