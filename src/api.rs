use hmac::{Hmac, Mac, NewMac};
use reqwest::header;
use serde_json::json;
use sha2::Sha256;
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};

static APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

pub fn get_signature(request: String) -> String {
    let secret_key = env::var("BINANCE_SECRET_KEY").unwrap();
    let mut signed_key = Hmac::<Sha256>::new_from_slice(secret_key.as_bytes()).unwrap();
    signed_key.update(request.as_bytes());
    let signature = hex::encode(signed_key.finalize().into_bytes());
    format!("{}", signature)
}

pub fn get_timestamp(time: SystemTime) -> u128 {
    let since_epoch = time.duration_since(UNIX_EPOCH).unwrap();
    since_epoch.as_millis()
}

pub fn get_client() -> reqwest::Client {
    let mut headers = header::HeaderMap::new();
    headers.insert(
        header::HeaderName::from_static("x-mbx-apikey"),
        header::HeaderValue::from_str(&env::var("BINANCE_API_KEY").unwrap()).unwrap(),
    );
    let client = reqwest::Client::builder()
        .default_headers(headers)
        .user_agent(APP_USER_AGENT)
        .build()
        .unwrap();

    client
}

pub async fn get_balance(client: &reqwest::Client) {
    let timestamp = get_timestamp(SystemTime::now());
    let params = format!("timestamp={}", timestamp.to_string());
    let signature = get_signature(params.clone());
    let request = format!(
        "https://fapi.binance.com/fapi/v2/account?{}&signature={}",
        params.clone(),
        signature
    );
    let result = client
        .get(request)
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();

    let balances = result["assets"].as_array().unwrap();
    for i in 0..balances.len() {
        println!(
            "{} - {}",
            balances[i]["asset"],
            balances[i]["walletBalance"]
                .as_str()
                .unwrap()
                .parse::<f32>()
                .unwrap()
        );
    }
}

pub struct Kline {
    pub open: Vec<f64>,
    pub high: Vec<f64>,
    pub low: Vec<f64>,
    pub close: Vec<f64>,
}

pub async fn get_klines_struct(
    client: &reqwest::Client,
    ticker: &str,
    interval: &str,
    limit: u32,
) -> Kline {
    let timestamp = get_timestamp(SystemTime::now());
    let params = format!("timestamp={}", timestamp.to_string());
    let signature = get_signature(params.clone());
    let request_body = format!(
        "https://fapi.binance.com/fapi/v1/klines?{}&symbol={}&interval={}&limit={}&signature={}",
        params, ticker, interval, limit, signature
    );
    let result = client
        .get(request_body)
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();

    println!("{:?}", &result[0]);

    let kline_struct = Kline {
        open: result
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x[1].as_str().unwrap().parse::<f64>().unwrap())
            .collect(),
        high: result
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x[1].as_str().unwrap().parse::<f64>().unwrap())
            .collect(),
        low: result
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x[3].as_str().unwrap().parse::<f64>().unwrap())
            .collect(),
        close: result
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x[4].as_str().unwrap().parse::<f64>().unwrap())
            .collect(),
    };
    kline_struct
}
