/**
 * Armageddon Mode State
 * 
 * Emergency response system for extreme LP drops.
 * 
 * Made by LabsX402 for Solana
 * https://x.com/LabsX402
 */

use anchor_lang::prelude::*;

/// Armageddon Mode levels
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq)]
pub enum ArmageddonLevel {
    /// Normal operation
    Normal = 0,
    /// LP dropped 50% - Defensive measures
    Defcon3 = 1,
    /// LP dropped 75% - Treasury injection
    Defcon2 = 2,
    /// LP dropped 90% - Emergency pause
    Defcon1 = 3,
}

impl Default for ArmageddonLevel {
    fn default() -> Self {
        Self::Normal
    }
}

/// Armageddon State account
#[account]
pub struct ArmageddonState {
    /// Token config this state belongs to
    pub token_config: Pubkey,
    
    /// Current Armageddon level
    pub level: u8,
    
    /// Timestamp when Armageddon was triggered
    pub triggered_at: i64,
    
    /// LP value when triggered
    pub lp_value_at_trigger: u64,
    
    /// Baseline LP value (for recovery calculation)
    pub baseline_lp_value: u64,
    
    /// Authority that can trigger Armageddon (usually token config admin)
    pub trigger_authority: Pubkey,
    
    /// Authority that can recover from Armageddon
    pub recovery_authority: Pubkey,
    
    /// LP recovery threshold (in bps, e.g., 12000 = 120% of trigger value)
    pub recovery_threshold_bps: u16,
    
    /// Fee rate override during Armageddon (max fee)
    pub emergency_fee_bps: u16,
    
    /// LP share override during Armageddon (higher share)
    pub emergency_lp_share_bps: u16,
    
    /// Is trading paused (DEFCON 1 only)
    pub trading_paused: bool,
    
    /// Max pause duration (seconds)
    pub max_pause_duration: i64,
    
    /// Bump seed for PDA
    pub bump: u8,
    
    /// Reserved for future use
    pub reserved: [u8; 32],
}

impl ArmageddonState {
    pub const LEN: usize = 8 + // discriminator
        32 + // token_config
        1 +  // level
        8 +  // triggered_at
        8 +  // lp_value_at_trigger
        8 +  // baseline_lp_value
        32 + // trigger_authority
        32 + // recovery_authority
        2 +  // recovery_threshold_bps
        2 +  // emergency_fee_bps
        2 +  // emergency_lp_share_bps
        1 +  // trading_paused
        8 +  // max_pause_duration
        1 +  // bump
        32;  // reserved
    
    /// Check if LP has recovered enough to exit Armageddon
    pub fn can_recover(&self, current_lp_value: u64) -> bool {
        if self.level == 0 {
            return false; // Not in Armageddon
        }
        
        let recovery_target = self.lp_value_at_trigger
            .checked_mul(self.recovery_threshold_bps as u64)
            .unwrap()
            .checked_div(10_000)
            .unwrap();
        
        current_lp_value >= recovery_target
    }
    
    /// Get DEFCON level thresholds
    pub fn get_threshold(level: u8) -> u8 {
        match level {
            1 => 50,  // DEFCON 3: 50% drop
            2 => 75,  // DEFCON 2: 75% drop
            3 => 90,  // DEFCON 1: 90% drop
            _ => 0,
        }
    }
    
    /// Get responses for each level
    pub fn get_response(level: u8) -> &'static str {
        match level {
            1 => "Fee maxed to 3%, LP share to 90%",
            2 => "Above + Treasury injection initiated",
            3 => "Above + Trading slowdown active",
            _ => "Normal operation",
        }
    }
}

