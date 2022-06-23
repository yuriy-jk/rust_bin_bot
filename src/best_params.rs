use crate::api::{get_klines_struct, Kline};
use crate::indic_compute::{sar, wma};
use crate::pnl::count_pnl_wma;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

#[derive(Debug)]
pub struct BestParams {
    pub profit: f64,
    pub profit_trade: i32,
    pub loss_trade: i32,
    pub accel: f64,
    pub max: f64,
    pub period: i32,
    pub pnl: String,
}

pub fn count_best_params(
    client: &reqwest::blocking::Client,
    tiker: &str,
    interval: &str,
    klines_count: &u32,
    best_profit: &Arc<Mutex<f64>>,
    best_params: &Arc<Mutex<BestParams>>,
) {
    log::info!("start_count_best_params");
    let start = Instant::now();
    let klines = get_klines_struct(&client, &tiker, &interval, &klines_count).unwrap();
    let mut handles: Vec<thread::JoinHandle<()>> = vec![];
    for period in 5..22 {
        let klines_copy = klines.clone();
        let best_thread_profit = Arc::clone(&best_profit);
        let best_thread_params = Arc::clone(&best_params);
        let handle = thread::spawn(move || {
            count_profit(&klines_copy, period, best_thread_profit, best_thread_params)
        });
        handles.push(handle)
    }

    for handle in handles {
        handle.join().unwrap();
    }
    let duration = start.elapsed();
    log::info!("duration time {:?}", duration);
    log::info!("best_profit={:?}", &best_profit);
    log::info!("best_params={:?}", &best_params)
}

fn count_profit(
    klines: &Kline,
    period: i32,
    best_thread_profit: Arc<Mutex<f64>>,
    best_thread_params: Arc<Mutex<BestParams>>,
) {
    let mut best_profit: f64 = 0.0;
    let mut best_profit_trade: i32 = 0;
    let mut best_loss_trade: i32 = 0;
    let mut best_accel: f64 = 0.0;
    let mut best_max: f64 = 0.0;
    let mut best_pnl = "best";
    for accel in (1..300).map(|x| x as f64 * 0.001) {
        for max in (1..300).map(|x| x as f64 * 0.001) {
            let (sar_values, _) = sar(&accel, &max, &klines.high, &klines.low);
            let (wma_values, wma_begin) = wma(&period, &klines.close);
            // println!("sar={}-{}, wma={}-{}, open={}", sar_values.len(), sar_begin,  wma_values.len(), wma_begin, klines.open.len());
            let (profit_trade, loss_trade, profit) = count_pnl_wma(
                &klines.open[wma_begin as usize..klines.open.len()].to_vec(),
                &klines.high[wma_begin as usize..klines.high.len()].to_vec(),
                &klines.low[wma_begin as usize..klines.low.len()].to_vec(),
                &sar_values[wma_begin as usize - 1..sar_values.len()].to_vec(),
                &wma_values,
            );
            if profit > best_profit {
                best_profit = profit;
                best_profit_trade = profit_trade;
                best_loss_trade = loss_trade;
                best_accel = accel;
                best_max = max;
                best_pnl = "wma";
            }
            // let (profit_trade, loss_trade, profit) = pnl::count_pnl_base(
            //     &klines.open[wma_begin as usize..klines.open.len()].to_vec(),
            //     &sar_values[wma_begin as usize - 1..sar_values.len()].to_vec(),
            // );
            // if profit > best_profit {
            //     best_profit = profit;
            //     best_profit_trade = profit_trade;
            //     best_loss_trade = loss_trade;
            //     best_accel = accel;
            //     best_max = max;
            //     best_pnl = "base";
            // }
        }
    }
    let mut profit = best_thread_profit.lock().unwrap();
    let mut params = best_thread_params.lock().unwrap();
    if best_profit > *profit {
        *profit = best_profit;
        params.profit = best_profit;
        params.profit_trade = best_profit_trade;
        params.loss_trade = best_loss_trade;
        params.accel = best_accel;
        params.max = best_max;
        params.period = period;
        params.pnl = best_pnl.to_string();
    }
    log::debug!(
        "{} - {} - {} - {} - {} - {} - {}",
        best_pnl,
        best_profit_trade,
        best_loss_trade,
        best_profit,
        period,
        best_accel,
        best_max
    )
}
