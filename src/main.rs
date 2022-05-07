use hmac::{Hmac, Mac, NewMac};
use reqwest::header;
use sha2::Sha256;
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};

static APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    println!("key: {}", env::var("TEST_BINANCE_API_KEY").unwrap());
    let timestamp = get_timestamp(SystemTime::now());
    let params = format!("timestamp={}", timestamp.to_string());
    println!("Request:{}", params);
    let signature = get_signature(params.clone());

    let request = format!(
        "https://testnet.binancefuture.com/fapi/v2/account?{}&signature={}",
        params.clone(),
        signature
    );
    let client = get_client();

    let result = client
        .get(request)
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();

    let balances = result["assets"].as_array().unwrap();
    println!("{}", balances.len());
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

fn get_signature(request: String) -> String {
    let secret_key = env::var("TEST_BINANCE_SECRET_KEY").unwrap();
    let mut signed_key = Hmac::<Sha256>::new_from_slice(secret_key.as_bytes()).unwrap();
    signed_key.update(request.as_bytes());
    let signature = hex::encode(signed_key.finalize().into_bytes());
    format!("{}", signature)
}

fn get_timestamp(time: SystemTime) -> u128 {
    let since_epoch = time.duration_since(UNIX_EPOCH).unwrap();
    since_epoch.as_millis()
}

fn get_client() -> reqwest::Client {
    let mut headers = header::HeaderMap::new();
    headers.insert(
        header::HeaderName::from_static("x-mbx-apikey"),
        header::HeaderValue::from_str(&env::var("TEST_BINANCE_API_KEY").unwrap()).unwrap(),
    );
    let client = reqwest::Client::builder()
        .default_headers(headers)
        .user_agent(APP_USER_AGENT)
        .build()
        .unwrap();

    client
}
