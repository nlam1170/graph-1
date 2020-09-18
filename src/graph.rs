use reqwest::Client;
use std::error::Error;
use serde_json::Value;
use futures::future;
use rayon::prelude::*;
use plotly::common::{Side, Title, Mode, Marker,};
use plotly::layout::{Axis, Layout};
use plotly::{Plot, Rgb, Scatter};


const SYMBOLS: &'static [&'static str] = &["BTCUSDT", "ETHUSDT", "BCHUSDT", "XRPUSDT", "EOSUSDT", "LTCUSDT", "TRXUSDT", "ETCUSDT", "LINKUSDT", "XLMUSDT", "ADAUSDT", "XMRUSDT", "DASHUSDT", "ZECUSDT", 
"XTZUSDT", "BNBUSDT", "ATOMUSDT", "ONTUSDT", "IOTAUSDT", "BATUSDT", "VETUSDT", "NEOUSDT", "QTUMUSDT", "IOSTUSDT", "THETAUSDT", "ALGOUSDT", "ZILUSDT", "BALUSDT", "SUSHIUSDT", "CRVUSDT", 
"KNCUSDT", "ZRXUSDT", "COMPUSDT", "OMGUSDT", "DOGEUSDT", "SXPUSDT", "LENDUSDT", "KAVAUSDT", "BANDUSDT", "RLCUSDT", "WAVESUSDT", "MKRUSDT", "SNXUSDT", "DOTUSDT"];

#[derive(Debug)]
struct Liquid<'a>{
    symbol: &'a str,
    oi: f64,
}

#[derive(Debug)]
struct Volatility<'a> {
    symbol: &'a str,
    vol: f64,
}

fn get_urls_for_oi() -> Vec<String> {
    let results = SYMBOLS.par_iter().map(|s| {
        let url = format!("https://fapi.binance.com/fapi/v1/openInterest?symbol={}", &s);
        url
    }).collect();
    results
}

fn get_urls_for_kline() -> Vec<String> {
    let results = SYMBOLS.par_iter().map(|s| {
        let url = format!("https://fapi.binance.com/fapi/v1/klines?symbol={}&interval=2h&limit=168", &s);
        url
    }).collect();
    results
}

fn get_urls_for_price() -> Vec<String> {
    let results = SYMBOLS.par_iter().map(|s| {
        let url = format!("https://api.binance.com/api/v3/ticker/price?symbol={}", &s);
        url
    }).collect();
    results
}

async fn make_grp_req(urls: &Vec<String>) -> Result<Vec<Value>, Box<dyn Error>> {
    let mut result = vec![];
    let client = Client::new();

    let bodies = future::join_all(urls.iter().map(|url| {
        let client_ref = &client;
        async move {
            let resp = client_ref.get(url).send().await?;
            resp.text().await
        }
    }))
    .await;

    for b in bodies {
        let s = b.unwrap();
        let v: Value = serde_json::from_str(&s).unwrap();
        result.push(v);
    }
    Ok(result)
}

async fn calculate_vol_for_value(v: &Value) -> Result<f64, Box<dyn Error>> {
    let num: f64 = v.as_array().unwrap().par_iter().map(|x| {
        let open: f64 = x[1].as_str().unwrap().parse().unwrap();
        let close: f64 = x[4].as_str().unwrap().parse().unwrap();
        ((open - close) / close * 100.0).abs() 
    }).sum();
    Ok(num / 168.0)
}

async fn fill_vol<'a>() -> Result<Vec<Volatility<'a>>, Box<dyn Error>> {
    let urls = get_urls_for_kline();
    let resp = make_grp_req(&urls).await?;
    let mut result = vec![];

    for i in resp.iter() {
        let n = calculate_vol_for_value(i).await?;
        result.push(n);
    }

    let most_vol = result.iter().cloned().fold(0./0., f64::max);

    let mut j = 0;
    let vol_vec: Vec<Volatility> = result.iter().map(|x| {
        let n = most_vol / x;
        j += 1;
        Volatility {symbol: SYMBOLS[j - 1], vol: n}
    }).collect();
    
    Ok(vol_vec)
}

async fn fill_oi<'a>() -> Result<Vec<Liquid<'a>>, Box<dyn Error>> {
    let urls = get_urls_for_oi();
    let prices = get_usdt_prices().await?;
    let resp = make_grp_req(&urls).await?;
    let btc_oi: f64 = &resp[0]["openInterest"].as_str().unwrap().parse().unwrap() * &prices[0] / 1000000.0;

    let mut i = 0;
    let oi_vec: Vec<Liquid> = resp.iter().map(|x| {
        let oi: f64 = x["openInterest"].as_str().unwrap().parse().unwrap();
        let x = btc_oi / ((oi * prices[i]) / 1000000.0);
        i += 1;
        Liquid {symbol: SYMBOLS[i - 1], oi: x}
    }).collect();
    Ok(oi_vec)
}

async fn get_usdt_prices() -> Result<Vec<f64>, Box<dyn Error>> {
    let urls = get_urls_for_price();
    let resp = make_grp_req(&urls).await?;
 
    let prices: Vec<f64> = resp.par_iter().map(|x| {
        let price: f64 = x["price"].as_str().unwrap().parse().unwrap();
        price
    }).collect();
    Ok(prices)
}

pub async fn graph_results() -> Result<(), Box<dyn Error>> {
    let mut oi_info = fill_oi().await?;
    let mut vol_info = fill_vol().await?;
    oi_info.sort_by(|a, b| a.oi.partial_cmp(&b.oi).unwrap());
    vol_info.sort_by(|a, b| a.vol.partial_cmp(&b.vol).unwrap());


    let oi_vec: Vec<f64> = oi_info.par_iter().map(|x| x.oi).collect();
    let oi_names: Vec<&str> = oi_info.par_iter().map(|x| x.symbol).collect();
    
    let vol_vec: Vec<f64> = vol_info.par_iter().map(|x| x.vol).collect();
    let vol_names: Vec<&str> = vol_info.par_iter().map(|x| x.symbol).collect();


    let trace1 = Scatter::new(oi_names, oi_vec).name("LIQUIDITY").mode(Mode::Lines).marker(Marker::new().color(Rgb::new(236, 38, 37)));
    let trace2 = Scatter::new(vol_names, vol_vec).name("VOLATILITY").y_axis("y2").mode(Mode::Markers).marker(Marker::new().size(20).color(Rgb::new(0, 0, 0)));

    let mut plot = Plot::new();

    plot.add_trace(trace1);
    plot.add_trace(trace2);


    let layout = Layout::new()
    .x_axis(Axis::new().title(Title::new("USDT PAIRINGS")))
    .y_axis(Axis::new().title(Title::new("LOW LIQUIDITY SCALE")))
    .y_axis2(
        Axis::new().title(Title::new("LOW VOLATILITY SCALE"))
        .overlaying("y")
        .side(Side::Right)
    );

    //plot.set_layout(layout1);
    plot.set_layout(layout);
    plot.show();

    Ok(())
}