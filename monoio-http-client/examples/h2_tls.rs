use std::time::Instant;

use http::{request::Builder, Method};
use monoio_http::common::body::{Body, FixedBody, HttpBody};

#[monoio::main( enable_timer = true)]
async fn main() {
    let h2_client = monoio_http_client::Builder::new()
        .http2_client()
        .http2_max_concurrent_streams(200)
        .build();

    let base_url = "https://api.binance.com";
    let endpoint = "/api/v3/ticker/price";
    let symbol = "BTCUSDT"; 

    let url = format!("{}?symbol={}", endpoint, symbol);
    let mut latencies = Vec::new();

    for _ in 0..1000 {
        let url = url.clone();

        let request = Builder::new()
            .method(Method::GET)
            .uri(format!("{}{}", base_url, url))
            .version(http::Version::HTTP_2)
            .body(HttpBody::fixed_body(None))
            .unwrap();

        let start_time = Instant::now();

        let resp = h2_client
            .send_request(request)
            .await
            .expect("Failed to send request");

        let (_parts, mut body) = resp.into_parts();
        let mut collected_data = Vec::new();

        while let Some(Ok(data)) = body.next_data().await {
            collected_data.extend_from_slice(&data);
        }
        let s = String::from_utf8(collected_data).unwrap();
        println!("{}", s);

        let duration = start_time.elapsed();
        latencies.push(duration.as_millis() as u64);
    }

    let mut latencies = latencies.clone();

    latencies.sort();

    let p999 = calculate_percentile(&latencies, 99.9);
    let p99 = calculate_percentile(&latencies, 99.0);
    let p90 = calculate_percentile(&latencies, 90.0);
    let p60 = calculate_percentile(&latencies, 60.0);
    let avg = latencies.iter().sum::<u64>() as f64 / latencies.len() as f64;

    println!("P999 latency: {} ms", p999);
    println!("P99 latency: {} ms", p99);
    println!("P90 latency: {} ms", p90);
    println!("P60 latency: {} ms", p60);
    println!("Average latency: {} ms", avg);
}

fn calculate_percentile(latencies: &[u64], percentile: f64) -> u64 {
    let index = (percentile / 100.0 * latencies.len() as f64).ceil() as usize - 1;
    latencies[index]
}
