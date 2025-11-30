/**
 * Dev Vesting Vault State
 * 
 * Implements cliff + linear vesting with rate limiting.
 * Prevents dev dumps while allowing gradual access to tokens.
 * 
 * Made by LabsX402 for Solana
 * https://x.com/LabsX402
 */

use anchor_lang::prelude::*;

/// Dev Vesting Vault account
/// Locks dev tokens with cliff period and progressive unlocks
#[account]
pub struct DevVestingVault {
    /// Dev wallet that owns this vault
    pub dev: Pubkey,
    
    /// Token mint being vested
    pub mint: Pubkey,
    
    /// Token account holding vested tokens
    pub token_account: Pubkey,
    
    /// Total amount originally allocated
    pub total_allocation: u64,
    
    /// Amount liquid at TGE (can be 0 for full cliff)
    pub liquid_at_tge: u64,
    
    /// Total amount locked (total_allocation - liquid_at_tge)
    pub total_locked: u64,
    
    /// Currently locked amount (decreases as unlocks happen)
    pub locked_amount: u64,
    
    /// Amount pending unlock (in timelock)
    pub pending_amount: u64,
    
    /// Timestamp when vault was initialized
    pub initialized_at: i64,
    
    /// Cliff period in seconds (no unlocks before this passes)
    pub cliff_seconds: i64,
    
    /// Total vesting period in seconds
    pub vesting_seconds: i64,
    
    /// Timestamp when last unlock request was made
    pub last_request_time: i64,
    
    /// Timestamp when pending unlock becomes available
    pub unlock_time: i64,
    
    /// Cooldown between unlock requests (seconds)
    pub cooldown_seconds: i64,
    
    /// Timelock from request to availability (seconds)
    pub timelock_seconds: i64,
    
    /// Current unlock rate in bps (500 = 5% year 1, 1000 = 10% year 2+)
    pub unlock_rate_bps: u16,
    
    /// Total amount unlocked (lifetime)
    pub total_unlocked: u64,
    
    /// Bump seed for PDA
    pub bump: u8,
    
    /// Reserved for future use
    pub reserved: [u8; 32],
}

impl DevVestingVault {
    pub const LEN: usize = 8 + // discriminator
        32 + // dev
        32 + // mint
        32 + // token_account
        8 +  // total_allocation
        8 +  // liquid_at_tge
        8 +  // total_locked
        8 +  // locked_amount
        8 +  // pending_amount
        8 +  // initialized_at
        8 +  // cliff_seconds
        8 +  // vesting_seconds
        8 +  // last_request_time
        8 +  // unlock_time
        8 +  // cooldown_seconds
        8 +  // timelock_seconds
        2 +  // unlock_rate_bps
        8 +  // total_unlocked
        1 +  // bump
        32;  // reserved
    
    /// Check if cliff period has passed
    pub fn cliff_passed(&self, current_time: i64) -> bool {
        let cliff_end = self.initialized_at + self.cliff_seconds;
        current_time >= cliff_end
    }
    
    /// Check if cooldown has passed
    pub fn cooldown_passed(&self, current_time: i64) -> bool {
        let cooldown_end = self.last_request_time + self.cooldown_seconds;
        current_time >= cooldown_end
    }
    
    /// Check if timelock has expired
    pub fn timelock_expired(&self, current_time: i64) -> bool {
        current_time >= self.unlock_time
    }
    
    /// Calculate maximum unlockable amount based on rate
    /// Uses saturating arithmetic - safe for all inputs
    pub fn max_unlockable(&self) -> u64 {
        // Rate is in bps (e.g., 500 = 5%)
        self.locked_amount
            .saturating_mul(self.unlock_rate_bps as u64)
            / 10_000
    }
    
    /// Calculate vested amount based on time
    /// Uses saturating arithmetic - safe for all inputs
    pub fn vested_amount(&self, current_time: i64) -> u64 {
        if !self.cliff_passed(current_time) {
            return 0;
        }
        
        let time_since_cliff = current_time - (self.initialized_at + self.cliff_seconds);
        let vesting_time = self.vesting_seconds - self.cliff_seconds;
        
        if time_since_cliff >= vesting_time || vesting_time <= 0 {
            // Fully vested
            return self.total_locked;
        }
        
        // Linear vesting - safe division
        self.total_locked
            .saturating_mul(time_since_cliff as u64)
            / (vesting_time as u64).max(1)
    }
    
    /// Update unlock rate based on time since TGE
    /// Year 1: 5% per request
    /// Year 2+: 10% per request
    pub fn update_unlock_rate(&mut self, current_time: i64) {
        let months_since_tge = (current_time - self.initialized_at) / (30 * 24 * 60 * 60);
        
        if months_since_tge >= 18 {
            // Year 2+ (after month 18)
            self.unlock_rate_bps = 1000; // 10%
        } else {
            // Year 1 (months 7-18)
            self.unlock_rate_bps = 500; // 5%
        }
    }
}

