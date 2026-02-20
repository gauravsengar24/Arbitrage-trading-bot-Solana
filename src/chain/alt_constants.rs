use once_cell::sync::Lazy;
use solana_sdk::pubkey::Pubkey;

/// Jupiter external address lookup table (resolved at runtime to avoid const eval limits)
pub static ALT_EXTERNAL: Lazy<Vec<Pubkey>> = Lazy::new(|| {
    vec![
        "3pqmFC8JcBNoZqojvaUqTi7ydxa3EdVvbFGb7PZMqMY"
            .parse()
            .expect("invalid ALT_EXTERNAL pubkey"),
    ]
});
