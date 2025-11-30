/**
 * LP Lock State - Progressive Timelock with Snapshot/Restore
 * 
 * TIMELINE:
 * - Days 0-3:   12h notice (emergency fixes)
 * - Days 3-15:  15 days notice (careful changes)
 * - Days 15+:   30 days notice (permanent mode)
 * 
 * SAFETY:
 * - Snapshot taken before any withdrawal
 * - Holder balances + LP state preserved
 * - Restore capability for relaunch
 * - All actions visible on-chain
 * 
 * Made by LabsX402 for Solana
 * https://x.com/LabsX402
 */

use anchor_lang::prelude::*;

// =============================================================================
// TIMELOCK CONSTANTS
// =============================================================================

/// Phase 1: First 3 days - 12h notice for emergency fixes
pub const PHASE1_DURATION_SECONDS: i64 = 3 * 24 * 60 * 60; // 3 days
pub const PHASE1_TIMELOCK_SECONDS: i64 = 12 * 60 * 60; // 12 hours

/// Phase 2: Days 3-15 - 15 day notice
pub const PHASE2_DURATION_SECONDS: i64 = 15 * 24 * 60 * 60; // 15 days total
pub const PHASE2_TIMELOCK_SECONDS: i64 = 15 * 24 * 60 * 60; // 15 days

/// Phase 3: After 15 days - 30 day notice (permanent)
pub const PHASE3_TIMELOCK_SECONDS: i64 = 30 * 24 * 60 * 60; // 30 days

/// Maximum withdrawal per request: 100% (full pull allowed with proper notice)
pub const MAX_WITHDRAWAL_BPS: u16 = 10000;

/// Maximum pending withdrawals
pub const MAX_PENDING_WITHDRAWALS: usize = 3;

/// Maximum snapshots stored
pub const MAX_SNAPSHOTS: usize = 5;

// =============================================================================
// ENUMS
// =============================================================================

/// LP Lock phase
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq)]
pub enum LpLockPhase {
    /// Phase 1: Emergency period (0-3 days)
    Emergency = 0,
    /// Phase 2: Stabilization period (3-15 days)
    Stabilization = 1,
    /// Phase 3: Permanent mode (15+ days)
    Permanent = 2,
}

impl Default for LpLockPhase {
    fn default() -> Self {
        Self::Emergency
    }
}

/// LP Lock status
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq)]
pub enum LpLockStatus {
    NotInitialized,
    Active,
    WithdrawalPending,
    Withdrawn,
    Restored,
}

impl Default for LpLockStatus {
    fn default() -> Self {
        Self::NotInitialized
    }
}

// =============================================================================
// SNAPSHOT STRUCTURES
// =============================================================================

/// Holder balance snapshot entry
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Default)]
pub struct HolderSnapshot {
    /// Holder wallet address
    pub wallet: Pubkey,
    /// Token balance at snapshot
    pub balance: u64,
}

/// LP State snapshot
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Default)]
pub struct LpSnapshot {
    /// Snapshot ID
    pub id: u64,
    /// Timestamp when snapshot was taken
    pub timestamp: i64,
    /// Reason for snapshot
    pub reason: [u8; 32],
    /// LP token amount at snapshot
    pub lp_tokens: u64,
    /// SOL reserve at snapshot
    pub sol_reserve: u64,
    /// Token reserve at snapshot
    pub token_reserve: u64,
    /// Total supply at snapshot
    pub total_supply: u64,
    /// Number of holders at snapshot
    pub holder_count: u32,
    /// Is this snapshot valid for restore
    pub is_valid: bool,
    /// Has this been restored
    pub was_restored: bool,
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
    /// Snapshot ID taken before this withdrawal
    pub snapshot_id: u64,
    /// Is this slot active
    pub is_active: bool,
}

// =============================================================================
// MAIN LP LOCK ACCOUNT
// =============================================================================

/// LP Lock account with progressive timelock
#[account]
pub struct LpLock {
    // ─────────────────────────────────────────────────────────────────────────
    // IDENTIFIERS
    // ─────────────────────────────────────────────────────────────────────────
    
    /// Token mint this lock belongs to
    pub mint: Pubkey,
    /// LP pool address (Raydium/Orca/Meteora)
    pub lp_pool: Pubkey,
    /// LP token mint
    pub lp_token_mint: Pubkey,
    /// Vault holding locked LP tokens (PDA owned)
    pub lp_vault: Pubkey,
    
    // ─────────────────────────────────────────────────────────────────────────
    // AUTHORITIES
    // ─────────────────────────────────────────────────────────────────────────
    
    /// Admin who can propose withdrawals (transferable to DAO)
    pub admin: Pubkey,
    /// Governance address for major changes
    pub governance: Pubkey,
    /// Emergency multisig (requires 2/3 for phase 1)
    pub emergency_multisig: Pubkey,
    
    // ─────────────────────────────────────────────────────────────────────────
    // TIMESTAMPS & PHASE
    // ─────────────────────────────────────────────────────────────────────────
    
    /// Timestamp when lock was created
    pub created_at: i64,
    /// Current lock phase
    pub phase: LpLockPhase,
    /// Current lock status
    pub status: LpLockStatus,
    
    // ─────────────────────────────────────────────────────────────────────────
    // LP STATE
    // ─────────────────────────────────────────────────────────────────────────
    
    /// Total LP tokens locked
    pub lp_tokens_locked: u64,
    /// Total LP tokens withdrawn (lifetime)
    pub total_withdrawn: u64,
    /// Initial LP tokens (for restore reference)
    pub initial_lp_tokens: u64,
    
    // ─────────────────────────────────────────────────────────────────────────
    // SNAPSHOTS
    // ─────────────────────────────────────────────────────────────────────────
    
    /// Snapshot counter
    pub snapshot_counter: u64,
    /// Last 5 snapshots
    pub snapshots: [LpSnapshot; 5],
    /// Most recent valid snapshot ID for restore
    pub latest_restorable_snapshot: u64,
    
    // ─────────────────────────────────────────────────────────────────────────
    // PENDING WITHDRAWALS
    // ─────────────────────────────────────────────────────────────────────────
    
    /// Pending withdrawal requests (max 3)
    pub pending_withdrawals: [PendingWithdrawal; 3],
    /// Number of active pending withdrawals
    pub pending_count: u8,
    
    // ─────────────────────────────────────────────────────────────────────────
    // METADATA
    // ─────────────────────────────────────────────────────────────────────────
    
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
        32 + // emergency_multisig
        8 +  // created_at
        1 +  // phase
        1 +  // status
        8 +  // lp_tokens_locked
        8 +  // total_withdrawn
        8 +  // initial_lp_tokens
        8 +  // snapshot_counter
        (8 + 8 + 32 + 8 + 8 + 8 + 8 + 4 + 1 + 1) * 5 + // snapshots (5x ~86 bytes)
        8 +  // latest_restorable_snapshot
        (8 + 32 + 8 + 8 + 64 + 8 + 1) * 3 + // pending_withdrawals (3x ~129 bytes)
        1 +  // pending_count
        1 +  // bump
        64;  // reserved
    
    // =========================================================================
    // PHASE CALCULATION
    // =========================================================================
    
    /// Get current phase based on time since creation
    pub fn get_current_phase(&self) -> LpLockPhase {
        let now = match Clock::get() {
            Ok(clock) => clock.unix_timestamp,
            Err(_) => return self.phase,
        };
        
        let age = now - self.created_at;
        
        if age < PHASE1_DURATION_SECONDS {
            LpLockPhase::Emergency
        } else if age < PHASE2_DURATION_SECONDS {
            LpLockPhase::Stabilization
        } else {
            LpLockPhase::Permanent
        }
    }
    
    /// Get required timelock for current phase
    pub fn get_required_timelock(&self) -> i64 {
        match self.get_current_phase() {
            LpLockPhase::Emergency => PHASE1_TIMELOCK_SECONDS,
            LpLockPhase::Stabilization => PHASE2_TIMELOCK_SECONDS,
            LpLockPhase::Permanent => PHASE3_TIMELOCK_SECONDS,
        }
    }
    
    /// Get phase name for display
    pub fn get_phase_name(&self) -> &'static str {
        match self.get_current_phase() {
            LpLockPhase::Emergency => "EMERGENCY (12h notice)",
            LpLockPhase::Stabilization => "STABILIZATION (15d notice)",
            LpLockPhase::Permanent => "PERMANENT (30d notice)",
        }
    }
    
    /// Get days until next phase
    pub fn days_until_next_phase(&self) -> Option<i64> {
        let now = match Clock::get() {
            Ok(clock) => clock.unix_timestamp,
            Err(_) => return None,
        };
        
        let age = now - self.created_at;
        
        match self.get_current_phase() {
            LpLockPhase::Emergency => Some((PHASE1_DURATION_SECONDS - age) / (24 * 60 * 60)),
            LpLockPhase::Stabilization => Some((PHASE2_DURATION_SECONDS - age) / (24 * 60 * 60)),
            LpLockPhase::Permanent => None, // No next phase
        }
    }
    
    // =========================================================================
    // INITIALIZATION
    // =========================================================================
    
    /// Initialize a new LP lock
    pub fn initialize(
        &mut self,
        mint: Pubkey,
        lp_pool: Pubkey,
        lp_token_mint: Pubkey,
        lp_vault: Pubkey,
        admin: Pubkey,
        emergency_multisig: Pubkey,
        lp_amount: u64,
        bump: u8,
    ) {
        let clock = Clock::get().expect("Clock required");
        
        self.mint = mint;
        self.lp_pool = lp_pool;
        self.lp_token_mint = lp_token_mint;
        self.lp_vault = lp_vault;
        self.admin = admin;
        self.governance = admin;
        self.emergency_multisig = emergency_multisig;
        self.created_at = clock.unix_timestamp;
        self.phase = LpLockPhase::Emergency;
        self.status = LpLockStatus::Active;
        self.lp_tokens_locked = lp_amount;
        self.total_withdrawn = 0;
        self.initial_lp_tokens = lp_amount;
        self.snapshot_counter = 0;
        self.latest_restorable_snapshot = 0;
        self.pending_count = 0;
        self.bump = bump;
        
        // Clear arrays
        for s in &mut self.snapshots {
            *s = LpSnapshot::default();
        }
        for pw in &mut self.pending_withdrawals {
            *pw = PendingWithdrawal::default();
        }
    }
    
    // =========================================================================
    // SNAPSHOT MANAGEMENT
    // =========================================================================
    
    /// Take a snapshot of current state
    pub fn take_snapshot(
        &mut self,
        reason: [u8; 32],
        sol_reserve: u64,
        token_reserve: u64,
        total_supply: u64,
        holder_count: u32,
    ) -> u64 {
        let clock = Clock::get().expect("Clock required");
        
        self.snapshot_counter += 1;
        let snapshot_id = self.snapshot_counter;
        
        // Rotate snapshots (keep last 5)
        let idx = ((snapshot_id - 1) % 5) as usize;
        
        self.snapshots[idx] = LpSnapshot {
            id: snapshot_id,
            timestamp: clock.unix_timestamp,
            reason,
            lp_tokens: self.lp_tokens_locked,
            sol_reserve,
            token_reserve,
            total_supply,
            holder_count,
            is_valid: true,
            was_restored: false,
        };
        
        self.latest_restorable_snapshot = snapshot_id;
        
        snapshot_id
    }
    
    /// Get snapshot by ID
    pub fn get_snapshot(&self, id: u64) -> Option<&LpSnapshot> {
        for s in &self.snapshots {
            if s.id == id && s.is_valid {
                return Some(s);
            }
        }
        None
    }
    
    /// Mark snapshot as restored
    pub fn mark_snapshot_restored(&mut self, id: u64) {
        for s in &mut self.snapshots {
            if s.id == id {
                s.was_restored = true;
            }
        }
    }
    
    // =========================================================================
    // WITHDRAWAL MANAGEMENT
    // =========================================================================
    
    /// Announce a new withdrawal (starts timelock)
    pub fn announce_withdrawal(
        &mut self,
        amount: u64,
        recipient: Pubkey,
        reason: [u8; 64],
        snapshot_id: u64,
    ) -> Result<usize> {
        let clock = Clock::get()?;
        
        // Find empty slot
        let slot = self.pending_withdrawals
            .iter()
            .position(|pw| !pw.is_active)
            .ok_or(error!(crate::ParadoxError::TooManyPendingWithdrawals))?;
        
        let timelock = self.get_required_timelock();
        
        self.pending_withdrawals[slot] = PendingWithdrawal {
            amount,
            recipient,
            announced_at: clock.unix_timestamp,
            execute_after: clock.unix_timestamp + timelock,
            reason,
            snapshot_id,
            is_active: true,
        };
        
        self.pending_count += 1;
        self.status = LpLockStatus::WithdrawalPending;
        
        Ok(slot)
    }
    
    /// Check if withdrawal can be executed
    pub fn can_execute_withdrawal(&self, slot: usize) -> bool {
        if slot >= MAX_PENDING_WITHDRAWALS {
            return false;
        }
        
        let pw = &self.pending_withdrawals[slot];
        if !pw.is_active {
            return false;
        }
        
        match Clock::get() {
            Ok(clock) => clock.unix_timestamp >= pw.execute_after,
            Err(_) => false,
        }
    }
    
    /// Get time remaining until withdrawal executable
    pub fn time_until_executable(&self, slot: usize) -> i64 {
        if slot >= MAX_PENDING_WITHDRAWALS {
            return i64::MAX;
        }
        
        let pw = &self.pending_withdrawals[slot];
        if !pw.is_active {
            return i64::MAX;
        }
        
        match Clock::get() {
            Ok(clock) => (pw.execute_after - clock.unix_timestamp).max(0),
            Err(_) => i64::MAX,
        }
    }
    
    /// Execute withdrawal
    pub fn execute_withdrawal(&mut self, slot: usize) -> Result<(u64, Pubkey)> {
        require!(slot < MAX_PENDING_WITHDRAWALS, crate::ParadoxError::InvalidWithdrawalSlot);
        require!(self.pending_withdrawals[slot].is_active, crate::ParadoxError::NoActiveWithdrawal);
        require!(self.can_execute_withdrawal(slot), crate::ParadoxError::TimelockNotExpired);
        
        let pw = &self.pending_withdrawals[slot];
        let amount = pw.amount;
        let recipient = pw.recipient;
        
        // Update state
        self.lp_tokens_locked = self.lp_tokens_locked.saturating_sub(amount);
        self.total_withdrawn = self.total_withdrawn.saturating_add(amount);
        
        // Clear slot
        self.pending_withdrawals[slot] = PendingWithdrawal::default();
        self.pending_count = self.pending_count.saturating_sub(1);
        
        // Update status
        if self.pending_count == 0 {
            self.status = if self.lp_tokens_locked == 0 {
                LpLockStatus::Withdrawn
            } else {
                LpLockStatus::Active
            };
        }
        
        Ok((amount, recipient))
    }
    
    /// Cancel withdrawal
    pub fn cancel_withdrawal(&mut self, slot: usize) -> Result<()> {
        require!(slot < MAX_PENDING_WITHDRAWALS, crate::ParadoxError::InvalidWithdrawalSlot);
        require!(self.pending_withdrawals[slot].is_active, crate::ParadoxError::NoActiveWithdrawal);
        
        self.pending_withdrawals[slot] = PendingWithdrawal::default();
        self.pending_count = self.pending_count.saturating_sub(1);
        
        if self.pending_count == 0 {
            self.status = LpLockStatus::Active;
        }
        
        Ok(())
    }
    
    // =========================================================================
    // RESTORE
    // =========================================================================
    
    /// Restore LP from snapshot (for relaunch)
    pub fn restore_from_snapshot(&mut self, lp_amount: u64) {
        self.lp_tokens_locked = lp_amount;
        self.status = LpLockStatus::Restored;
        
        // Update phase to current (may have advanced during restore)
        self.phase = self.get_current_phase();
    }
}

// =============================================================================
// HOLDER BALANCES ACCOUNT (Separate for scalability)
// =============================================================================

/// Holder balances snapshot account
/// Stored separately due to size constraints
#[account]
pub struct HolderBalancesSnapshot {
    /// Snapshot ID this belongs to
    pub snapshot_id: u64,
    /// LP Lock this belongs to
    pub lp_lock: Pubkey,
    /// Timestamp
    pub timestamp: i64,
    /// Number of holders
    pub holder_count: u32,
    /// Holder balances (up to 100 per account, chain for more)
    pub holders: Vec<HolderSnapshot>,
    /// Next account in chain (if more than 100 holders)
    pub next_account: Option<Pubkey>,
    /// Bump seed
    pub bump: u8,
}

impl HolderBalancesSnapshot {
    pub const BASE_LEN: usize = 8 + // discriminator
        8 +  // snapshot_id
        32 + // lp_lock
        8 +  // timestamp
        4 +  // holder_count
        4 +  // vec length
        33 + // next_account (Option<Pubkey>)
        1;   // bump
    
    /// Calculate size for N holders
    pub fn size_for_holders(n: usize) -> usize {
        Self::BASE_LEN + (n * (32 + 8)) // wallet + balance per holder
    }
}
