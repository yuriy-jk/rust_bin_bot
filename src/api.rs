use chrono::prelude::*;
use hmac::{Hmac, Mac, NewMac};
use reqwest::{header, Response};
use sha2::Sha256;
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};
use serde_json::json;
use serde::{Deserialize, Serialize};
use std::{thread, time};

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

pub fn get_client() -> reqwest::blocking::Client {
    let mut headers = header::HeaderMap::new();
    headers.insert(
        header::HeaderName::from_static("x-mbx-apikey"),
        header::HeaderValue::from_str(&env::var("BINANCE_API_KEY").unwrap()).unwrap(),
    );
    let client = reqwest::blocking::Client::builder()
        .default_headers(headers)
        .user_agent(APP_USER_AGENT)
        .build()
        .unwrap();

    client
}

pub fn get_balance(client: &reqwest::blocking::Client) {
    let mut result = json!({});
    let mut retries: i32 = 0;
    while retries != 3 {
        let timestamp = get_timestamp(SystemTime::now());
        let params = format!("timestamp={}", timestamp.to_string());
        let signature = get_signature(params.clone());
        let request = format!(
            "https://fapi.binance.com/fapi/v2/account?{}&signature={}",
            params.clone(),
            signature
        );
        let response = match client
            .get(request)
            .send()
            {
                Ok(response) => {
                    if response.status().is_success() {
                        result = response.json::<serde_json::Value>()
                                                    .unwrap();
                        break
                    } else {
                        println!("{:?}", response.error_for_status());
                        break
                    }
                },
                Err(err) => {
                    println!("{}", err);
                    retries += 1;
                    continue
                },
            };
        } 

    if retries == 3 {
        panic!("Max retries exceeded")
    }

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
    pub timestamp: Vec<i64>,
}


pub fn get_klines_struct(
    client: &reqwest::blocking::Client,
    ticker: &str,
    interval: &str,
    limit: &u32,
) -> Option<Kline> {
    let mut result = json!({});
    let timestamp = get_timestamp(SystemTime::now());
    let params = format!("timestamp={}", timestamp.to_string());
    let signature = get_signature(params.clone());
    let request = format!(
        "https://fapi.binance.com/fapi/v1//klines?{}&symbol={}&interval={}&limit={}&signature={}",
        params, ticker, interval, limit, signature
    );
    let response = match client
            .get(request)
            .send()
            {
                Ok(response) => {
                    if response.status().is_success() {
                        result = response.json::<serde_json::Value>()
                                                    .unwrap();
                        let kline_struct = Kline {
                            open: get_kline_price_field(&result, 1),
                            high: get_kline_price_field(&result, 2),
                            low: get_kline_price_field(&result, 3),
                            close: get_kline_price_field(&result, 4),
                            timestamp: get_kline_timestamp_field(&result, 0)
                        };
                        return Some(kline_struct)
                    } else {
                        println!("{:?}", response.error_for_status());
                        return None
                    }
            },
                Err(err) => {
                    println!("Get Error - {}", err);
                    return None
                    }
            };
    }
// println!("{:?}", &result[result[0].as_array().unwrap().len() - 1]);

fn get_kline_price_field(res: &serde_json::Value, index: usize) -> Vec<f64> {
    res.as_array().unwrap()
    .iter()
    .map(|x| x[index]
    .as_str().unwrap()
    .parse::<f64>().unwrap())
    .collect()
}

fn get_kline_timestamp_field(res: &serde_json::Value, index: usize) -> Vec<i64> {
    res.as_array().unwrap()
    .iter()
    .map(|x| x[index].as_i64().unwrap())
    .collect()
}


pub fn get_server_time(client: &reqwest::blocking::Client) {
    let request_body = format!("https://fapi.binance.com/fapi/v1/time",);
    let response = client
        .get(request_body)
        .send()
        .unwrap();
    
    if response.status().is_success(){
        let result = response
            .json::<serde_json::Value>()
            .unwrap();
            let timestamp = result["serverTime"].as_i64().unwrap();
            let date = Utc.timestamp_millis(timestamp.try_into().unwrap());
            println!("{}  -  {}", date.format("%Y-%m-%d %H:%M:%S"), Utc::now())
    } else {
        println!("{}", response.status())
    }
}

#[derive(Serialize, Deserialize)]
pub struct Price {
    pub symbol: String,
    pub price: String,
    pub time: u64
}

pub fn get_curr_price(client: &reqwest::blocking::Client, tiker: &str,) -> f64 {
    let price: Price = loop {
        let timestamp = get_timestamp(SystemTime::now());
        let params = format!("timestamp={}", timestamp.to_string());
        let signature = get_signature(params.clone());
        let request = format!(
            "https://fapi.binance.com/fapi/v1/ticker/price?{}&symbol={}&signature={}",
            params, tiker, signature
        );
        let _ = match client.get(request).send() {
            Ok(response) => {
                if response.status().is_success() {
                    break response.json().unwrap();
                } else {
                    println!("{:?}", response.error_for_status());
                    thread::sleep(time::Duration::from_secs(1));
                }
            }
            Err(err) => {
                println!("Get Error - {}", err);
                thread::sleep(time::Duration::from_secs(1));
            }
        };
    };
    price.price.parse::<f64>().unwrap()
}