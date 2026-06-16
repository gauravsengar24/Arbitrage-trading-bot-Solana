//! Transaction submission via Solana RPC only.

use solana_sdk::{
    compute_budget::ComputeBudgetInstruction,
    hash::Hash,
    instruction::Instruction,
    message::{AddressLookupTableAccount, VersionedMessage, v0},
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    transaction::VersionedTransaction,
};

/// Parameters for a trade submission.
pub struct Tips {
    pub tip_sol_amount: f64,
    pub tip_addr_idx: u8,
    pub cu: Option<u32>,
    pub priority_fee_micro_lamport: Option<u64>,
    pub payer: Pubkey,
    pub pure_ix: Vec<Instruction>,
}

/// Always returns false — low-latency third-party services are not used.
pub fn use_low_latency_submission() -> bool {
    false
}

/// Submit a signed versioned transaction via the configured submit RPC endpoint.
/// Returns `true` when the RPC node accepted the transaction (signature received),
/// `false` on any signing or network error after all retries.
pub async fn submit_with_services(
    tx_info: Tips,
    signers: &'static Vec<&'static Keypair>,
    recent_blockhash: Hash,
    nonce_ix: Instruction,
    alt: Vec<AddressLookupTableAccount>,
    retry_count: u32,
) -> bool {
    let mut instructions: Vec<Instruction> = Vec::new();

    if let Some(cu) = tx_info.cu {
        instructions.push(ComputeBudgetInstruction::set_compute_unit_limit(cu));
    }
    if let Some(fee) = tx_info.priority_fee_micro_lamport {
        instructions.push(ComputeBudgetInstruction::set_compute_unit_price(fee));
    }

    // Nonce advance must come before swap instructions.
    instructions.push(nonce_ix);
    instructions.extend(tx_info.pure_ix);

    let message = match v0::Message::try_compile(
        &tx_info.payer,
        &instructions,
        &alt,
        recent_blockhash,
    ) {
        Ok(m) => m,
        Err(e) => {
            tracing::error!(error = %e, "Failed to compile transaction message");
            return false;
        }
    };

    let signer_refs: Vec<&dyn Signer> = signers.iter().copied().map(|k| k as &dyn Signer).collect();
    let tx = match VersionedTransaction::try_new(VersionedMessage::V0(message), &signer_refs) {
        Ok(t) => t,
        Err(e) => {
            tracing::error!(error = %e, "Failed to sign transaction");
            return false;
        }
    };

    for attempt in 0..=retry_count {
        match crate::SUBMIT_CLIENT.send_transaction(&tx).await {
            Ok(sig) => {
                tracing::info!(signature = %sig, "Transaction sent via RPC");
                return true;
            }
            Err(e) if attempt < retry_count => {
                tracing::warn!(error = %e, attempt, "RPC send failed, retrying");
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            }
            Err(e) => {
                tracing::error!(error = %e, "RPC send failed after all retries");
            }
        }
    }
    false
}
