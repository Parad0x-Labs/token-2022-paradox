/**
 * Token Configuration State
 * 
 * Made by LabsX402 for Solana
 * https://x.com/LabsX402
 */

use anchor_lang::prelude::*;

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
    pub fn calculate_distribution(&self, fee_amount: u64) -> (u64, u64, u64) {
        let to_lp = fee_amount
            .checked_mul(self.lp_share_bps as u64)
            .unwrap()
            .checked_div(10_000)
            .unwrap();
        
        let to_burn = fee_amount
            .checked_mul(self.burn_share_bps as u64)
            .unwrap()
            .checked_div(10_000)
            .unwrap();
        
        let to_treasury = fee_amount
            .checked_sub(to_lp)
            .unwrap()
            .checked_sub(to_burn)
            .unwrap();
        
        (to_lp, to_burn, to_treasury)
    }
}

