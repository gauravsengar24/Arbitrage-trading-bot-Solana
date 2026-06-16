//! Runtime statistics, circuit breaker, and P&L tracking.
//!
//! The circuit breaker opens after THRESHOLD consecutive submission failures
//! and holds for PAUSE_SECS, then automatically resets. Discovery continues
//! while the breaker is open so no opportunities are missed.

use std::sync::atomic::{AtomicI64, AtomicU32, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use once_cell::sync::Lazy;

const CIRCUIT_BREAKER_THRESHOLD: u32 = 5;
const CIRCUIT_BREAKER_PAUSE_SECS: u64 = 30;

pub struct ArbStats {
    pub trades_attempted:   AtomicU64,
    pub trades_successful:  AtomicU64,
    /// Cumulative net profit in lamports (convert token units before recording for non-SOL tokens).
    pub total_profit_lamports: AtomicI64,
    pub consecutive_failures: AtomicU32,
    paused_until: Mutex<Option<Instant>>,
}

pub static STATS: Lazy<ArbStats> = Lazy::new(|| ArbStats {
    trades_attempted:      AtomicU64::new(0),
    trades_successful:     AtomicU64::new(0),
    total_profit_lamports: AtomicI64::new(0),
    consecutive_failures:  AtomicU32::new(0),
    paused_until:          Mutex::new(None),
});

impl ArbStats {
    pub fn record_attempt(&self) {
        self.trades_attempted.fetch_add(1, Ordering::Relaxed);
    }

    /// Call after a trade is confirmed accepted by the RPC node.
    /// `net_profit_lamports` is the expected net profit expressed in lamports.
    pub fn record_success(&self, net_profit_lamports: i64) {
        self.trades_successful.fetch_add(1, Ordering::Relaxed);
        self.total_profit_lamports.fetch_add(net_profit_lamports, Ordering::Relaxed);
        self.consecutive_failures.store(0, Ordering::Relaxed);
    }

    /// Call when a submission fails (RPC error, signing error, etc.).
    pub fn record_failure(&self) {
        let failures = self.consecutive_failures.fetch_add(1, Ordering::Relaxed) + 1;
        if failures >= CIRCUIT_BREAKER_THRESHOLD {
            let mut guard = self.paused_until.lock().unwrap();
            if guard.is_none() {
                *guard = Some(Instant::now() + Duration::from_secs(CIRCUIT_BREAKER_PAUSE_SECS));
                tracing::warn!(
                    consecutive_failures = failures,
                    pause_secs = CIRCUIT_BREAKER_PAUSE_SECS,
                    "Circuit breaker open — pausing submissions"
                );
            }
        }
    }

    /// Returns true when the circuit breaker is open. Auto-resets once the pause window expires.
    pub fn is_circuit_open(&self) -> bool {
        let mut guard = self.paused_until.lock().unwrap();
        if let Some(until) = *guard {
            if Instant::now() < until {
                return true;
            }
            *guard = None;
            self.consecutive_failures.store(0, Ordering::Relaxed);
            tracing::info!("Circuit breaker closed — resuming submissions");
        }
        false
    }

    /// Win rate as a percentage (0.0–100.0).
    pub fn win_rate(&self) -> f64 {
        let attempted  = self.trades_attempted.load(Ordering::Relaxed);
        let successful = self.trades_successful.load(Ordering::Relaxed);
        if attempted == 0 { 0.0 } else { successful as f64 / attempted as f64 * 100.0 }
    }

    pub fn total_profit_sol(&self) -> f64 {
        self.total_profit_lamports.load(Ordering::Relaxed) as f64 / 1_000_000_000.0
    }
}
