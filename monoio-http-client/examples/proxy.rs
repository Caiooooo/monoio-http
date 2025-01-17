use core::fmt;
use std::time::Duration;
use http::Method;
use monoio_http::common::body::{Body, FixedBody, HttpBody};
use monoio_http_client::Client;
// use percent_encoding::utf8_percent_encode;
// use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use thiserror::Error;

#[derive(Default)]
pub struct RestClient {
    pub domain: &'static str,
    pub port: u16,
    pub timeout: u64,  // ms
    pub defualt_headers: Vec<(String, String)>,
    pub client: Client,
}

impl RestClient {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        domain: &'static str, // binance.com
        port: u16,            // 443
        timeout: u64,
    ) -> Self {
        RestClient {
            domain,
            port,
            timeout,
            defualt_headers: vec![],
            client: monoio_http_client::Builder::new().http2_client().build(),
        }
    }

    pub fn add_headers(&mut self, headers: Vec<(String, String)>) {
        self.defualt_headers.extend(headers);
    }

    // pub fn replace_headers(&mut self, headers: Vec<(String, String)>) {
    //     self.defualt_headers.extend(headers);
    // }

    pub fn request_builder(
        &self,
        path: &str,
        params: Option<Vec<(String, String)>>,
        method: Option<Method>,
        headers: Option<Vec<(String, String)>>,
        body: Option<Vec<(String, String)>>,
    ) -> http::Request<HttpBody> {
        let mut builder = http::Request::builder()
            .version(http::Version::HTTP_2)
            .header(http::header::HOST, self.domain);

        builder = match method {
            Some(method) => builder.method(method),
            None => builder.method(Method::GET),
        };
        if let Some(headers) = headers {
            for (key, value) in headers {
                builder = builder.header(key, value);
            }
        }
        for (key, value) in &self.defualt_headers {
            builder = builder.header(key, value);
        }
        // target url
        let url = format!(
            "{}://{}{}{}",
            http::uri::Scheme::HTTPS,
            self.domain,
            path,
            params
            .as_ref()
            .map(|params| {
                let query_string = serde_urlencoded::to_string(params).unwrap_or_default();
                if !query_string.is_empty() {
                    format!("?{}", query_string)
                } else {
                    String::new()
                }
            })
            .unwrap_or_default()
        );
        // println!("url:{}", url);
        // let url = utf8_percent_encode(&url, NON_ALPHANUMERIC).to_string();
        builder.uri(url).body(match body{
            Some(body) => {
                let body_string = serde_urlencoded::to_string(&body)
                    .unwrap_or_else(|_| String::new()); 
    
                HttpBody::fixed_body(Some(bytes::Bytes::from(body_string)))
            }
            None => HttpBody::fixed_body(None)
        }).unwrap()
    }


    pub async fn get_response(&self, request: http::Request<HttpBody>) -> Result<String, PlutusError> {
        let timeout_duration = Duration::from_millis(self.timeout);
        let response_result = monoio::time::timeout(timeout_duration, self.client.send_request(request)).await;

        let response = match response_result {
            Ok(Ok(response)) => response,   
            Ok(Err(e)) => return Err(PlutusError::RequestFailed(e.to_string())), 
            Err(_) => return Err(PlutusError::Timeout),      
        };

        let (_parts, mut body) = response.into_parts();
        
        let mut collected_data = Vec::new();

        // 读取响应体
        while let Some(Ok(data)) = body.next_data().await {
            collected_data.extend_from_slice(&data);
        }
        String::from_utf8(collected_data).map_err(|e| PlutusError::ParseError(e.to_string()))
    }
}

impl fmt::Debug for RestClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RestClient")
            .field("domain", &self.domain)
            .field("port", &self.port)
            .field("timeout", &self.timeout)
            .finish()
    }
}
#[monoio::main(timer_enabled = true)]
async fn main () {
    let client = RestClient::new("api.bitget.com", 443, 3000);

    let req = client.request_builder(
        "/api/v2/spot/market/tickers",
        Some(vec![("symbol".to_string(), "BTCUSDT".to_string())]),
        None,
        None,
        None
    );
    println!("{:?}", client);
    println!("{:?}", client.get_response(req).await);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[monoio::test(timer_enabled = true)]
    async fn test_http() {
        let client = RestClient::new("api.bitget.com", 443, 1000);

        let req = client.request_builder(
            "/api/v2/spot/market/tickers",
            Some(vec![("symbol".to_string(), "BTCUSDT".to_string())]),
            None,
            None,
            None
        );
        println!("{:?}", client);
        println!("{:?}", client.get_response(req).await);
    }
}

#[derive(Error, Debug)]
pub enum PlutusError {
    #[error("network error")]
    NetworkError,
    #[error("balance not enough")]
    BalanceNotEnough,
    #[error("too many request")]
    TooManyRequest,
    #[error("order not found")]
    OrderNotFound,
    #[error("bad precision")]
    BadPrecision,
    #[error("cookie expired")]
    CookieExpired,
    #[error("unknown error")]
    UnknownError,
    #[error("unknown symbol")]
    UnknownSymbol,
    #[error("unknown symbol")]
    InvalidPrecision,
    #[error("used private api")]
    UsedPrivateAPI,
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),
    #[error("HTTP request failed: {0}")]
    RequestFailed(String),
    #[error("Response parsing failed: {0}")]
    ParseError(String),
    #[error("Timeout")]
    Timeout,
}
