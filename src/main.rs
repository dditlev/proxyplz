use std::{collections::HashMap, net::SocketAddr, time::Duration};

use bytes::Bytes;
use reqwest::header::HeaderMap;
use thiserror::Error;
use warp::{
    http::{self, StatusCode},
    Filter, Rejection, Reply,
};

#[derive(Debug, Error)]
enum ProxyError {
    #[error("Missing URL parameter")]
    MissingUrl,
    #[error("Malformed URL")]
    MalformedUrl,
    #[error("Upstream request failed: {0}")]
    ReqwestError(#[from] reqwest::Error),
    #[error("HTTP error: {0}")]
    HttpError(#[from] http::Error),
}
impl warp::reject::Reject for ProxyError {}

async fn handle_rejection(err: Rejection) -> Result<impl Reply, std::convert::Infallible> {
    let (code, message) = if let Some(e) = err.find::<ProxyError>() {
        match e {
            ProxyError::MissingUrl => (StatusCode::BAD_REQUEST, "Missing URL parameter"),
            ProxyError::MalformedUrl => (StatusCode::BAD_REQUEST, "Malformed URL"),
            ProxyError::ReqwestError(_) => (StatusCode::BAD_GATEWAY, "Upstream request failed"),
            ProxyError::HttpError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error"),
        }
    } else {
        (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
    };

    Ok(warp::reply::with_status(
        message,
        code,
    ))
}

async fn proxy_handler(
    method: http::Method,
    headers: HeaderMap,
    query: HashMap<String, String>,
    body: Bytes,
    client: reqwest::Client,
) -> Result<impl Reply, Rejection> {
    let target_url = query
        .get("url")
        .ok_or_else(|| warp::reject::custom(ProxyError::MissingUrl))?;

    let target_url = reqwest::Url::parse(target_url)
        .map_err(|_| warp::reject::custom(ProxyError::MalformedUrl))?;

    let mut req_builder = client.request(method, target_url);

    // Filter hop-by-hop headers
    let filtered_headers = headers
        .iter()
        .filter(|(name, _)| {
            let name = name.as_str().to_lowercase();
            !matches!(
                name.as_str(),
                "host"
                    | "connection"
                    | "keep-alive"
                    | "proxy-authenticate"
                    | "proxy-authorization"
                    | "te"
                    | "trailer"
                    | "transfer-encoding"
                    | "upgrade"
            )
        })
        .fold(HeaderMap::new(), |mut acc, (name, value)| {
            acc.insert(name, value.clone());
            acc
        });

    req_builder = req_builder.headers(filtered_headers).body(body);

    let upstream_resp = req_builder
        .send()
        .await
        .map_err(ProxyError::ReqwestError)
        .map_err(warp::reject::custom)?;

    let mut response_builder = http::Response::builder().status(upstream_resp.status());

    // Filter response headers and add CORS
    for (name, value) in upstream_resp.headers().iter() {
        if name.as_str().eq_ignore_ascii_case("access-control-allow-origin") {
            continue;
        }
        response_builder = response_builder.header(name, value);
    }

    let response_builder = response_builder
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Allow-Methods", "*")
        .header("Access-Control-Allow-Headers", "*");

    let resp_bytes = upstream_resp
        .bytes()
        .await
        .map_err(ProxyError::ReqwestError)
        .map_err(warp::reject::custom)?;

    response_builder
        .body(resp_bytes)
        .map_err(ProxyError::HttpError)
        .map_err(warp::reject::custom)
}

async fn listen(addr: SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::none())
        .build()?;

    let cors_route = warp::options().map(|| {
        warp::reply::with_header(
            warp::reply::with_status("", StatusCode::NO_CONTENT),
            "Access-Control-Allow-Origin",
            "*",
        )
    });

    let proxy_route = warp::any()
        .and(warp::method())
        .and(warp::header::headers_cloned())
        .and(warp::query::<HashMap<String, String>>())
        .and(warp::body::bytes())
        .and(warp::any().map(move || client.clone()))
        .and_then(proxy_handler);

    println!("Proxy server running at\nhttp://{}?url=...", addr);

    warp::serve(cors_route.or(proxy_route).recover(handle_rejection))
        .run(addr)
        .await;

    Ok(())
}

#[tokio::main]
async fn main() {
    println!("ProxyPlz");

    let listen_addr = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!(
            "Usage: {} <listen_addr>",
            std::env::args().next().unwrap()
        );
        std::process::exit(1);
    });

    let addr: SocketAddr = match listen_addr.parse() {
        Ok(addr) => addr,
        Err(_) => {
            eprintln!("Invalid listen address: {}", listen_addr);
            std::process::exit(1);
        }
    };

    if let Err(e) = listen(addr).await {
        eprintln!("Error: {:?}", e);
    }
}