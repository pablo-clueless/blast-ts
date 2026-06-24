// src/runner.rs
use reqwest::Client;
use serde_json::Value;
use std::collections::HashMap;
use std::time::Instant;

use crate::config::Endpoint;
use crate::template;

#[derive(Debug, Clone)]
pub struct RequestResult {
    pub endpoint_name: String,
    pub method: String,
    pub path: String,
    pub expected_status: Option<u16>,
    pub actual_status: u16,
    pub latency_ms: u128,
    pub passed: bool,
    pub error: Option<String>, // response body on failure or network error
    pub body: Option<Value>,
}

pub async fn execute(
    client: &Client,
    endpoint: &Endpoint,
    base_url: &str,
    ctx: &HashMap<String, String>,
) -> RequestResult {
    let start = Instant::now();

    let url = format!("{}{}", base_url.trim_end_matches('/'), endpoint.path);

    //resolving the headers
    let resolved_headers = match &endpoint.headers {
        Some(headers) => template::resolve_map(headers, ctx),
        None => HashMap::new(),
    };

    //body
    let resolved_body = endpoint.body.as_ref().map(|b| template::resolve(b, ctx));

    let method = match endpoint.method.to_uppercase().as_str() {
        "GET" => reqwest::Method::GET,
        "POST" => reqwest::Method::POST,
        "PUT" => reqwest::Method::PUT,
        "PATCH" => reqwest::Method::PATCH,
        "DELETE" => reqwest::Method::DELETE,
        "HEAD" => reqwest::Method::HEAD,
        _ => reqwest::Method::GET, // validate() already caught this
    };

    let mut request = client.request(method, &url);

    for (key, value) in &resolved_headers {
        request = request.header(key, value)
    }

    if let Some(body) = resolved_body {
        request = request.json(&body)
    };

    let response = match request.send().await {
        Ok(r) => r,
        Err(e) => {
            let latency_ms = start.elapsed().as_millis();
            return RequestResult {
                endpoint_name: endpoint.name.clone(),
                method: endpoint.method.clone(),
                path: endpoint.path.clone(),
                expected_status: endpoint.expect_status,
                actual_status: 0, // no status — never reached the server
                latency_ms,
                passed: false,
                error: Some(format!("network error: {e}")),
                body: None,
            };
        }
    };

    let latency_ms = start.elapsed().as_millis();
    let actual_status = response.status().as_u16();

    let passed = match endpoint.expect_status {
        Some(expected) => actual_status == expected,
        None => actual_status < 500,
    };

    // body from the http response
    let body_text = response.text().await.unwrap_or_default();

    let body: Option<Value> = serde_json::from_str(&body_text).ok();

    // checking for error
    let error = if !passed {
        Some(body_text.clone())
    } else {
        None
    };

    //return the result
    RequestResult {
        endpoint_name: endpoint.name.clone(),
        method: endpoint.method.clone(),
        path: endpoint.path.clone(),
        expected_status: endpoint.expect_status,
        actual_status,
        latency_ms,
        passed,
        error,
        body,
    }
}
