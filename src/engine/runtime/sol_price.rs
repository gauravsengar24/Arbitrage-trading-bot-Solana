use std::sync::Arc;
use tokio::sync::Mutex;
use once_cell::sync::Lazy;
use serde::Deserialize;
use chrono::Utc;

/// Shared state for SOL price (fetched from CoinGecko)
pub static SOL_PRICE: Lazy<Arc<Mutex<Option<f64>>>> = Lazy::new(|| Arc::new(Mutex::new(None)));

/// CoinGecko API response structure
#[derive(Debug, Deserialize)]
struct CoinGeckoResponse {
    solana: CoinGeckoPrice,
}

#[derive(Debug, Deserialize)]
struct CoinGeckoPrice {
    usd: f64,
}

/// Fetch SOL price from CoinGecko API with retry logic
pub async fn fetch_sol_price_from_coingecko() -> Result<f64, anyhow::Error> {
    const MAX_RETRIES: u32 = 3;
    const INITIAL_DELAY_MS: u64 = 1000; // 1 second
    
    let url = "https://api.coingecko.com/api/v3/simple/price?ids=solana&vs_currencies=usd";
    
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent("Jupiter-Arbitrage-Bot/1.0") // CoinGecko requires User-Agent header
        .build()?;
    
    let mut last_error = None;
    
    for attempt in 0..=MAX_RETRIES {
        let response = match client
            .get(url)
            .header("Accept", "application/json")
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                last_error = Some(format!("Network error: {}", e));
                if attempt < MAX_RETRIES {
                    let delay_ms = INITIAL_DELAY_MS * (1 << attempt); // Exponential backoff
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                    continue;
                }
                return Err(anyhow::anyhow!("Failed after {} retries: {}", MAX_RETRIES + 1, last_error.unwrap()));
            }
        };
        
        if response.status().is_success() {
            match response.json::<CoinGeckoResponse>().await {
                Ok(data) => return Ok(data.solana.usd),
                Err(e) => {
                    last_error = Some(format!("JSON parse error: {}", e));
                    if attempt < MAX_RETRIES {
                        let delay_ms = INITIAL_DELAY_MS * (1 << attempt);
                        tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                        continue;
                    }
                }
            }
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            last_error = Some(format!("HTTP {}: {}", status, error_text));
            
            // Don't retry on 403/401 (authentication errors) or 404
            if status == 403 || status == 401 || status == 404 {
                return Err(anyhow::anyhow!("CoinGecko API error: {}", last_error.unwrap()));
            }
            
            // Retry on 429 (rate limit) or 5xx (server errors)
            if attempt < MAX_RETRIES && (status == 429 || status.as_u16() >= 500) {
                let delay_ms = INITIAL_DELAY_MS * (1 << attempt);
                tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                continue;
            }
        }
    }
    
    Err(anyhow::anyhow!("Failed after {} retries: {}", MAX_RETRIES + 1, last_error.unwrap_or_else(|| "Unknown error".to_string())))
}

/// Update the shared SOL price state
pub async fn update_sol_price(price: f64) {
    let mut price_guard = SOL_PRICE.lock().await;
    *price_guard = Some(price);
}

/// Get the current SOL price (with fallback to config value)
pub async fn get_sol_price_usdc(fallback_price: f64) -> f64 {
    let price_guard = SOL_PRICE.lock().await;
    price_guard.unwrap_or(fallback_price)
}

/// Background task to fetch SOL price every 1 hour
pub async fn start_sol_price_fetcher(fallback_price: f64) {
    println!("[SOL Price] Starting price fetcher (updates every 1 hour)");
    
    // Fetch immediately on startup
    match fetch_sol_price_from_coingecko().await {
        Ok(price) => {
            update_sol_price(price).await;
            println!(
                "[{}] [SOL Price] ✅ Fetched SOL price: ${:.2} USDC",
                Utc::now().format("%Y-%m-%d %H:%M:%S"),
                price
            );
        }
        Err(e) => {
            eprintln!(
                "[{}] [SOL Price] ⚠️ Failed to fetch SOL price: {}. Using fallback: ${:.2}",
                Utc::now().format("%Y-%m-%d %H:%M:%S"),
                e,
                fallback_price
            );
        }
    }
    
    // Then fetch every 1 hour
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(3600)); // 1 hour = 3600 seconds
    
    loop {
        interval.tick().await;
        
        match fetch_sol_price_from_coingecko().await {
            Ok(price) => {
                update_sol_price(price).await;
                println!(
                    "[{}] [SOL Price] ✅ Updated SOL price: ${:.2} USDC",
                    Utc::now().format("%Y-%m-%d %H:%M:%S"),
                    price
                );
            }
            Err(e) => {
                eprintln!(
                    "[{}] [SOL Price] ⚠️ Failed to fetch SOL price: {}. Keeping previous value or using fallback: ${:.2}",
                    Utc::now().format("%Y-%m-%d %H:%M:%S"),
                    e,
                    fallback_price
                );
            }
        }
    }
}
