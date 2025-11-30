/**
 * DAO Treasury State
 * 
 * Governance-controlled treasury with spending limits and timelocks.
 * 
 * Made by LabsX402 for Solana
 * https://x.com/LabsX402
 */

use anchor_lang::prelude::*;

/// DAO Treasury Vault account
#[account]
pub struct DaoTreasuryVault {
    /// Governance address (multisig or DAO program)
    pub governance: Pubkey,
    
    /// Token mint
    pub mint: Pubkey,
    
    /// Token account holding treasury tokens
    pub token_account: Pubkey,
    
    /// Total tokens held
    pub balance: u64,
    
    /// Maximum spend per period (in bps of balance)
    pub max_spend_bps_per_period: u16,
    
    /// Period length in seconds
    pub period_seconds: i64,
    
    /// Start of current period
    pub period_start: i64,
    
    /// Amount spent in current period
    pub spent_this_period: u64,
    
    /// Pending withdrawal amount (in timelock)
    pub pending_amount: u64,
    
    /// Pending withdrawal recipient
    pub pending_recipient: Pubkey,
    
    /// Pending withdrawal reason
    pub pending_reason: [u8; 128],
    
    /// Timestamp when pending withdrawal can be executed
    pub pending_execute_after: i64,
    
    /// Timelock duration for withdrawals (seconds)
    pub timelock_seconds: i64,
    
    /// Total withdrawn (lifetime)
    pub total_withdrawn: u64,
    
    /// Bump seed for PDA
    pub bump: u8,
    
    /// Reserved for future use
    pub reserved: [u8; 32],
}

impl DaoTreasuryVault {
    pub const LEN: usize = 8 + // discriminator
        32 + // governance
        32 + // mint
        32 + // token_account
        8 +  // balance
        2 +  // max_spend_bps_per_period
        8 +  // period_seconds
        8 +  // period_start
        8 +  // spent_this_period
        8 +  // pending_amount
        32 + // pending_recipient
        128 + // pending_reason
        8 +  // pending_execute_after
        8 +  // timelock_seconds
        8 +  // total_withdrawn
        1 +  // bump
        32;  // reserved
    
    /// Get maximum spendable amount in current period
    /// Uses u128 intermediate calculations to prevent overflow
    pub fn max_spendable(&self) -> u64 {
        let max_spend = ((self.balance as u128)
            .saturating_mul(self.max_spend_bps_per_period as u128)
            .checked_div(10_000)
            .unwrap_or(0)) as u64;
        
        max_spend.saturating_sub(self.spent_this_period)
    }
    
    /// Check if period has reset
    pub fn should_reset_period(&self, current_time: i64) -> bool {
        current_time >= self.period_start + self.period_seconds
    }
    
    /// Reset period tracking
    pub fn reset_period(&mut self, current_time: i64) {
        self.period_start = current_time;
        self.spent_this_period = 0;
    }
    
    /// Check if withdrawal can be executed
    pub fn can_execute_withdrawal(&self, current_time: i64) -> bool {
        self.pending_amount > 0 && current_time >= self.pending_execute_after
    }
}

