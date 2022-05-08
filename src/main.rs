use std::env;
extern crate ta_lib_wrapper;
use serde_json::json;

mod api;
mod indic_compute;

struct Kline {
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    println!("key: {}", env::var("BINANCE_API_KEY").unwrap());
    let client = api::get_client();
    api::get_balance(&client).await;
    let klines = api::get_klines(&client, "BTCUSDT", "1h", 100).await;

    let open: Vec<f64> = klines
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x[1].as_str().unwrap().parse::<f64>().unwrap())
        .collect();
    let high: Vec<f64> = klines
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x[2].as_str().unwrap().parse::<f64>().unwrap())
        .collect();
    let low: Vec<f64> = klines
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x[3].as_str().unwrap().parse::<f64>().unwrap())
        .collect();
    let close: Vec<f64> = klines
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x[4].as_str().unwrap().parse::<f64>().unwrap())
        .collect();
    let (sar_values, begin) = indic_compute::sar(0.02, 0.2, &high, &low);
    println!(
        "{} - {:?}",
        &sar_values.len(),
        &sar_values[sar_values.len() - 3..]
    )
}
