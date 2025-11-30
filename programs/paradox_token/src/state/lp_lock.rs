/**
 * LP Lock State
 * 
 * Manages LP token lock with TIME-LOCKED withdrawals only.
 * - LP locked immediately on pool creation
 * - ANY withdrawal requires advance announcement (24h minimum)
 * - Timelock visible on-chain - everyone sees pending withdrawals
 * - No instant/emergency withdrawals possible
 * 
 * Made by LabsX402 for Solana
 * https://x.com/LabsX402
 */

use anchor_lang::prelude::*;

/// Minimum timelock for LP withdrawal announcement: 24 hours
pub const MIN_WITHDRAWAL_TIMELOCK_SECONDS: i64 = 24 * 60 * 60;

/// Default timelock: 48 hours (recommended)
pub const DEFAULT_WITHDRAWAL_TIMELOCK_SECONDS: i64 = 48 * 60 * 60;

/// Maximum pending withdrawals at once
pub const MAX_PENDING_WITHDRAWALS: usize = 3;

/// LP Lock status
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq)]
pub enum LpLockStatus {
    /// Not initialized
    NotInitialized,
    /// Active and locked (normal state)
    Locked,
    /// Withdrawal pending (announced, waiting for timelock)
    WithdrawalPending,
}

impl Default for LpLockStatus {
    fn default() -> Self {
        LpLockStatus::NotInitialized
    }
}

/// Pending withdrawal request
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Default)]
pub struct PendingWithdrawal {
    /// Amount of LP tokens to withdraw
    pub amount: u64,
    /// Recipient address
    pub recipient: Pubkey,
    /// Timestamp when withdrawal was announced
    pub announced_at: i64,
    /// Timestamp when withdrawal can be executed
    pub execute_after: i64,
    /// Reason for withdrawal (public)
    pub reason: [u8; 64],
    /// Is this slot active?
    pub is_active: bool,
}

/// LP Lock account
/// Controls LP token lock with timelock-only withdrawals
#[account]
pub struct LpLock {
    /// Token mint this lock belongs to
    pub mint: Pubkey,
    
    /// LP pool address (Raydium/Orca/Meteora)
    pub lp_pool: Pubkey,
    
    /// LP token mint
    pub lp_token_mint: Pubkey,
    
    /// Vault holding locked LP tokens (PDA owned)
    pub lp_vault: Pubkey,
    
    /// Admin who can propose withdrawals (transferable to DAO)
    pub admin: Pubkey,
    
    /// Governance address (for admin transfer)
    pub governance: Pubkey,
    
    /// Timestamp when lock was created
    pub created_at: i64,
    
    /// Current lock status
    pub status: LpLockStatus,
    
    /// Total LP tokens locked
    pub lp_tokens_locked: u64,
    
    /// Total LP tokens withdrawn (lifetime)
    pub total_withdrawn: u64,
    
    /// Withdrawal timelock in seconds (minimum 24h)
    pub withdrawal_timelock_seconds: i64,
    
    /// Maximum withdrawal per request (bps of total, e.g., 1000 = 10%)
    pub max_withdrawal_bps: u16,
    
    /// Pending withdrawal requests (max 3)
    pub pending_withdrawals: [PendingWithdrawal; 3],
    
    /// Number of active pending withdrawals
    pub pending_count: u8,
    
    /// Bump seed for PDA
    pub bump: u8,
    
    /// Reserved for future use
    pub reserved: [u8; 64],
}

impl LpLock {
    pub const LEN: usize = 8 + // discriminator
        32 + // mint
        32 + // lp_pool
        32 + // lp_token_mint
        32 + // lp_vault
        32 + // admin
        32 + // governance
        8 +  // created_at
        1 +  // status
        8 +  // lp_tokens_locked
        8 +  // total_withdrawn
        8 +  // withdrawal_timelock_seconds
        2 +  // max_withdrawal_bps
        (8 + 32 + 8 + 8 + 64 + 1) * 3 + // pending_withdrawals (3 slots)
        1 +  // pending_count
        1 +  // bump
        64;  // reserved
    
    /// Initialize a new LP lock
    pub fn initialize(
        &mut self,
        mint: Pubkey,
        lp_pool: Pubkey,
        lp_token_mint: Pubkey,
        lp_vault: Pubkey,
        admin: Pubkey,
        lp_amount: u64,
        timelock_seconds: i64,
        max_withdrawal_bps: u16,
        bump: u8,
    ) {
        let clock = Clock::get().unwrap();
        
        self.mint = mint;
        self.lp_pool = lp_pool;
        self.lp_token_mint = lp_token_mint;
        self.lp_vault = lp_vault;
        self.admin = admin;
        self.governance = admin; // Initially same as admin
        self.created_at = clock.unix_timestamp;
        self.status = LpLockStatus::Locked;
        self.lp_tokens_locked = lp_amount;
        self.total_withdrawn = 0;
        self.withdrawal_timelock_seconds = timelock_seconds.max(MIN_WITHDRAWAL_TIMELOCK_SECONDS);
        self.max_withdrawal_bps = max_withdrawal_bps.min(1000); // Max 10% per withdrawal
        self.pending_count = 0;
        self.bump = bump;
        
        // Clear pending withdrawals
        for pw in &mut self.pending_withdrawals {
            *pw = PendingWithdrawal::default();
        }
    }
    
    /// Check if a withdrawal amount is valid
    pub fn is_valid_withdrawal_amount(&self, amount: u64) -> bool {
        let max_amount = self.lp_tokens_locked
            .checked_mul(self.max_withdrawal_bps as u64)
            .unwrap_or(0)
            .checked_div(10_000)
            .unwrap_or(0);
        
        amount > 0 && amount <= max_amount
    }
    
    /// Announce a new withdrawal (starts timelock)
    pub fn announce_withdrawal(
        &mut self,
        amount: u64,
        recipient: Pubkey,
        reason: [u8; 64],
    ) -> Result<usize> {
        let clock = Clock::get().unwrap();
        
        // Find empty slot
        let slot = self.pending_withdrawals
            .iter()
            .position(|pw| !pw.is_active)
            .ok_or(error!(crate::ParadoxError::TooManyPendingWithdrawals))?;
        
        self.pending_withdrawals[slot] = PendingWithdrawal {
            amount,
            recipient,
            announced_at: clock.unix_timestamp,
            execute_after: clock.unix_timestamp + self.withdrawal_timelock_seconds,
            reason,
            is_active: true,
        };
        
        self.pending_count += 1;
        self.status = LpLockStatus::WithdrawalPending;
        
        Ok(slot)
    }
    
    /// Check if a pending withdrawal can be executed
    pub fn can_execute_withdrawal(&self, slot: usize) -> bool {
        if slot >= MAX_PENDING_WITHDRAWALS {
            return false;
        }
        
        let pw = &self.pending_withdrawals[slot];
        if !pw.is_active {
            return false;
        }
        
        let clock = Clock::get().unwrap();
        clock.unix_timestamp >= pw.execute_after
    }
    
    /// Get time remaining until withdrawal can execute
    pub fn time_until_executable(&self, slot: usize) -> i64 {
        if slot >= MAX_PENDING_WITHDRAWALS {
            return i64::MAX;
        }
        
        let pw = &self.pending_withdrawals[slot];
        if !pw.is_active {
            return i64::MAX;
        }
        
        let clock = Clock::get().unwrap();
        (pw.execute_after - clock.unix_timestamp).max(0)
    }
    
    /// Execute a pending withdrawal
    pub fn execute_withdrawal(&mut self, slot: usize) -> Result<(u64, Pubkey)> {
        require!(slot < MAX_PENDING_WITHDRAWALS, crate::ParadoxError::InvalidWithdrawalSlot);
        require!(self.pending_withdrawals[slot].is_active, crate::ParadoxError::NoActiveWithdrawal);
        require!(self.can_execute_withdrawal(slot), crate::ParadoxError::TimelockNotExpired);
        
        let pw = &self.pending_withdrawals[slot];
        let amount = pw.amount;
        let recipient = pw.recipient;
        
        // Update state
        self.lp_tokens_locked = self.lp_tokens_locked.saturating_sub(amount);
        self.total_withdrawn += amount;
        
        // Clear slot
        self.pending_withdrawals[slot] = PendingWithdrawal::default();
        self.pending_count = self.pending_count.saturating_sub(1);
        
        // Update status
        if self.pending_count == 0 {
            self.status = LpLockStatus::Locked;
        }
        
        Ok((amount, recipient))
    }
    
    /// Cancel a pending withdrawal
    pub fn cancel_withdrawal(&mut self, slot: usize) -> Result<()> {
        require!(slot < MAX_PENDING_WITHDRAWALS, crate::ParadoxError::InvalidWithdrawalSlot);
        require!(self.pending_withdrawals[slot].is_active, crate::ParadoxError::NoActiveWithdrawal);
        
        self.pending_withdrawals[slot] = PendingWithdrawal::default();
        self.pending_count = self.pending_count.saturating_sub(1);
        
        if self.pending_count == 0 {
            self.status = LpLockStatus::Locked;
        }
        
        Ok(())
    }
}
