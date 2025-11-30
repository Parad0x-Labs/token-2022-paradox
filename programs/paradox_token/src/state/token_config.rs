/**
 * Token Configuration State
 * 
 * Made by LabsX402 for Solana
 * https://x.com/LabsX402
 */

use anchor_lang::prelude::*;
use crate::ParadoxError;

/// Token configuration account
/// Stores fee rates, distribution shares, and admin keys
#[account]
pub struct TokenConfig {
    /// Token mint this config belongs to
    pub mint: Pubkey,
    
    /// Admin who can update config (transferable)
    pub admin: Pubkey,
    
    /// Governance address for major changes
    pub governance: Pubkey,
    
    /// Current transfer fee in basis points (100-300)
    pub transfer_fee_bps: u16,
    
    /// Share of fees going to LP growth (default 7000 = 70%)
    pub lp_share_bps: u16,
    
    /// Share of fees to burn (default 1500 = 15%)
    pub burn_share_bps: u16,
    
    /// Share of fees to treasury (default 1500 = 15%)
    pub treasury_share_bps: u16,
    
    /// Fee vault where collected fees accumulate
    pub fee_vault: Pubkey,
    
    /// Total fees collected (lifetime)
    pub total_fees_collected: u64,
    
    /// Total fees distributed (lifetime)
    pub total_fees_distributed: u64,
    
    /// Is token paused (emergency only)
    pub is_paused: bool,
    
    /// Current Armageddon level (0 = normal, 1-3 = DEFCON levels)
    pub armageddon_level: u8,
    
    /// Timestamp of last fee update
    pub last_fee_update: i64,
    
    /// Pending fee change (announced but not executed)
    pub pending_fee_bps: u16,
    
    /// Timestamp when pending fee change can be executed
    pub pending_fee_activate_time: i64,
    
    /// Timestamp when pending fee change can be cancelled (after activate_time)
    pub pending_fee_cancel_time: i64,
    
    /// Bump seed for PDA
    pub bump: u8,
    
    /// Reserved for future use
    pub reserved: [u8; 64],
}

impl TokenConfig {
    pub const LEN: usize = 8 + // discriminator
        32 + // mint
        32 + // admin
        32 + // governance
        2 +  // transfer_fee_bps
        2 +  // lp_share_bps
        2 +  // burn_share_bps
        2 +  // treasury_share_bps
        32 + // fee_vault
        8 +  // total_fees_collected
        8 +  // total_fees_distributed
        1 +  // is_paused
        1 +  // armageddon_level
        8 +  // last_fee_update
        2 +  // pending_fee_bps
        8 +  // pending_fee_activate_time
        8 +  // pending_fee_cancel_time
        1 +  // bump
        64;  // reserved
    
    /// Validate fee shares sum to 100%
    pub fn validate_shares(&self) -> bool {
        let total = self.lp_share_bps as u32 
            + self.burn_share_bps as u32 
            + self.treasury_share_bps as u32;
        total == 10_000
    }
    
    /// Calculate fee distribution for a given amount
    /// Uses u128 intermediate calculations to prevent overflow
    pub fn calculate_distribution(&self, fee_amount: u64) -> Result<(u64, u64, u64)> {
        // Use u128 for intermediate calculations to prevent overflow
        let to_lp = ((fee_amount as u128)
            .checked_mul(self.lp_share_bps as u128)
            .ok_or(error!(crate::ParadoxError::MathOverflow))?
            .checked_div(10_000)
            .ok_or(error!(crate::ParadoxError::MathOverflow))?) as u64;
        
        let to_burn = ((fee_amount as u128)
            .checked_mul(self.burn_share_bps as u128)
            .ok_or(error!(crate::ParadoxError::MathOverflow))?
            .checked_div(10_000)
            .ok_or(error!(crate::ParadoxError::MathOverflow))?) as u64;
        
        // Treasury gets remainder to ensure exact distribution
        let to_treasury = fee_amount
            .checked_sub(to_lp)
            .and_then(|v| v.checked_sub(to_burn))
            .ok_or(error!(crate::ParadoxError::MathOverflow))?;
        
        Ok((to_lp, to_burn, to_treasury))
    }
}

