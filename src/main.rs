use std::env;
extern crate ta_lib_wrapper;
use serde_json::json;

mod api;
mod indic_compute;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    println!("key: {}", env::var("BINANCE_API_KEY").unwrap());
    let client = api::get_client();
    api::get_balance(&client).await;
    let klines = api::get_klines_struct(&client, "BTCUSDT", "1h", 100).await;

    let (sar_values, begin) = indic_compute::sar(0.02, 0.2, &klines.high, &klines.low);
    println!(
        "Last 3 sar values {:?}",
        &sar_values[sar_values.len() - 3..]
    );

    let (rsi_values, begin) = indic_compute::rsi(14, &klines.close);
    println!(
        "Last 3 rsi values {:?}",
        &rsi_values[rsi_values.len() - 3..]
    )
}
