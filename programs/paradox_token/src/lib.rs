/**
 * Token-2022 Paradox Edition
 * 
 * SPL Token-2022 with transfer fees, LP growth, and vesting mechanics.
 * 
 * Made by LabsX402 for Solana
 * https://x.com/LabsX402
 * 
 * License: BSL 1.1 (converts to MIT after Dec 2028)
 */

use anchor_lang::prelude::*;

pub mod state;
pub mod instructions;

use state::*;
use instructions::*;

declare_id!("PARADOX111111111111111111111111111111111111");

// =============================================================================
// SEEDS
// =============================================================================

pub const TOKEN_CONFIG_SEED: &[u8] = b"token_config";
pub const LP_GROWTH_SEED: &[u8] = b"lp_growth";
pub const LP_LOCK_SEED: &[u8] = b"lp_lock";
pub const DEV_VESTING_SEED: &[u8] = b"dev_vesting";
pub const DAO_TREASURY_SEED: &[u8] = b"dao_treasury";
pub const FEE_VAULT_SEED: &[u8] = b"fee_vault";

/// Emergency window for LP lock: 15 minutes
pub const LP_EMERGENCY_WINDOW_SECONDS: i64 = 15 * 60;

// =============================================================================
// CONSTANTS
// =============================================================================

/// Basis points denominator (10000 = 100%)
pub const BPS_DENOMINATOR: u64 = 10_000;

/// Default transfer fee: 3% (300 bps)
pub const DEFAULT_TRANSFER_FEE_BPS: u16 = 300;

/// Minimum transfer fee: 1% (100 bps)
pub const MIN_TRANSFER_FEE_BPS: u16 = 100;

/// Maximum transfer fee: 3% (300 bps)  
pub const MAX_TRANSFER_FEE_BPS: u16 = 300;

/// Minimum transfer amount to prevent dust attack (fee must be >= 1 raw unit)
/// At 300 bps (3%), amounts below 34 result in 0 fee
pub const MIN_TRANSFER_AMOUNT: u64 = 34;

/// Fee change timelock: 24 hours (prevents front-running)
pub const FEE_CHANGE_TIMELOCK_SECONDS: i64 = 24 * 60 * 60;

/// Default LP share: 70%
pub const DEFAULT_LP_SHARE_BPS: u16 = 7000;

/// Default burn share: 15%
pub const DEFAULT_BURN_SHARE_BPS: u16 = 1500;

/// Default treasury share: 15%
pub const DEFAULT_TREASURY_SHARE_BPS: u16 = 1500;

/// Cliff period: 6 months in seconds
pub const DEFAULT_CLIFF_SECONDS: i64 = 6 * 30 * 24 * 60 * 60; // ~6 months

/// Vesting period: 36 months in seconds
pub const DEFAULT_VESTING_SECONDS: i64 = 36 * 30 * 24 * 60 * 60; // ~36 months

/// Cooldown between unlock requests: 30 days
pub const DEFAULT_COOLDOWN_SECONDS: i64 = 30 * 24 * 60 * 60;

/// Timelock from request to withdrawal: 30 days
pub const DEFAULT_TIMELOCK_SECONDS: i64 = 30 * 24 * 60 * 60;

/// Year 1 unlock rate: 5% per request
pub const YEAR1_UNLOCK_RATE_BPS: u16 = 500;

/// Year 2+ unlock rate: 10% per request
pub const YEAR2_UNLOCK_RATE_BPS: u16 = 1000;

// =============================================================================
// PROGRAM
// =============================================================================

#[program]
pub mod paradox_token {
    use super::*;

    // =========================================================================
    // TOKEN CONFIGURATION
    // =========================================================================

    /// Initialize token configuration
    /// Called once after token mint is created
    pub fn init_token_config(
        ctx: Context<InitTokenConfig>,
        transfer_fee_bps: u16,
        lp_share_bps: u16,
        burn_share_bps: u16,
        treasury_share_bps: u16,
    ) -> Result<()> {
        instructions::init_token_config::handler(
            ctx,
            transfer_fee_bps,
            lp_share_bps,
            burn_share_bps,
            treasury_share_bps,
        )
    }

    /// Announce fee change (starts 24h timelock)
    pub fn announce_fee_change(
        ctx: Context<AnnounceFeeChange>,
        new_fee_bps: u16,
    ) -> Result<()> {
        instructions::update_token_config::announce_fee_change_handler(ctx, new_fee_bps)
    }
    
    /// Execute fee change (after 24h timelock)
    pub fn execute_fee_change(
        ctx: Context<ExecuteFeeChange>,
    ) -> Result<()> {
        instructions::update_token_config::execute_fee_change_handler(ctx)
    }
    
    /// Cancel pending fee change
    pub fn cancel_fee_change(
        ctx: Context<CancelFeeChange>,
    ) -> Result<()> {
        instructions::update_token_config::cancel_fee_change_handler(ctx)
    }

    // =========================================================================
    // LP GROWTH MANAGER
    // =========================================================================

    /// Initialize LP Growth Manager
    /// Creates the PDA that controls automatic LP growth from fees
    pub fn init_lp_growth(
        ctx: Context<InitLpGrowth>,
        min_fee_threshold: u64,
        cooldown_seconds: i64,
    ) -> Result<()> {
        instructions::lp_growth::init_handler(ctx, min_fee_threshold, cooldown_seconds)
    }

    /// Execute LP Growth
    /// Uses accumulated fees to add liquidity to the pool
    pub fn execute_lp_growth(ctx: Context<ExecuteLpGrowth>) -> Result<()> {
        instructions::lp_growth::execute_handler(ctx)
    }

    /// Lock LP Growth (emergency)
    pub fn lock_lp_growth(ctx: Context<LockLpGrowth>) -> Result<()> {
        instructions::lp_growth::lock_handler(ctx)
    }

    /// Unlock LP Growth
    pub fn unlock_lp_growth(ctx: Context<UnlockLpGrowth>) -> Result<()> {
        instructions::lp_growth::unlock_handler(ctx)
    }

    // =========================================================================
    // DEV VESTING
    // =========================================================================

    /// Initialize dev vesting vault
    /// Locks dev tokens with cliff + linear vesting
    pub fn init_dev_vesting(
        ctx: Context<InitDevVesting>,
        total_allocation: u64,
        liquid_at_tge: u64,
        cliff_seconds: i64,
        vesting_seconds: i64,
    ) -> Result<()> {
        instructions::vesting::init_dev_handler(
            ctx,
            total_allocation,
            liquid_at_tge,
            cliff_seconds,
            vesting_seconds,
        )
    }

    /// Request dev unlock
    /// Starts timelock for withdrawal
    pub fn request_dev_unlock(
        ctx: Context<RequestDevUnlock>,
        amount: u64,
    ) -> Result<()> {
        instructions::vesting::request_unlock_handler(ctx, amount)
    }

    /// Execute dev unlock
    /// Withdraws after timelock expires
    pub fn execute_dev_unlock(ctx: Context<ExecuteDevUnlock>) -> Result<()> {
        instructions::vesting::execute_unlock_handler(ctx)
    }

    // =========================================================================
    // DAO TREASURY
    // =========================================================================

    /// Initialize DAO treasury
    pub fn init_dao_treasury(
        ctx: Context<InitDaoTreasury>,
        governance: Pubkey,
        max_spend_bps_per_period: u16,
        period_seconds: i64,
    ) -> Result<()> {
        instructions::treasury::init_handler(
            ctx,
            governance,
            max_spend_bps_per_period,
            period_seconds,
        )
    }

    /// Propose DAO withdrawal
    pub fn propose_dao_withdrawal(
        ctx: Context<ProposeDaoWithdrawal>,
        amount: u64,
        recipient: Pubkey,
        reason: String,
    ) -> Result<()> {
        instructions::treasury::propose_handler(ctx, amount, recipient, reason)
    }

    /// Execute DAO withdrawal (after timelock)
    pub fn execute_dao_withdrawal(ctx: Context<ExecuteDaoWithdrawal>) -> Result<()> {
        instructions::treasury::execute_handler(ctx)
    }

    // =========================================================================
    // ARMAGEDDON MODE (Emergency)
    // =========================================================================

    /// Initialize Armageddon state account
    pub fn init_armageddon(ctx: Context<InitArmageddon>) -> Result<()> {
        instructions::armageddon::init_armageddon_handler(ctx)
    }

    /// Trigger Armageddon mode
    /// Emergency response when LP drops significantly
    pub fn trigger_armageddon(
        ctx: Context<TriggerArmageddon>,
        level: u8, // 1 = DEFCON 3, 2 = DEFCON 2, 3 = DEFCON 1
    ) -> Result<()> {
        instructions::armageddon::trigger_handler(ctx, level)
    }

    /// Recover from Armageddon
    pub fn recover_from_armageddon(ctx: Context<RecoverArmageddon>) -> Result<()> {
        instructions::armageddon::recover_handler(ctx)
    }

    // =========================================================================
    // FEE DISTRIBUTION
    // =========================================================================

    /// Distribute collected fees
    /// Splits fees between LP, burn, and treasury
    pub fn distribute_fees(ctx: Context<DistributeFees>) -> Result<()> {
        instructions::fees::distribute_handler(ctx)
    }

    // =========================================================================
    // LP LOCK (Progressive Timelock with Snapshot/Restore)
    // =========================================================================
    // 
    // TIMELINE:
    //   Days 0-3:   12h notice (emergency fixes)
    //   Days 3-15:  15 days notice (stabilization)
    //   Days 15+:   30 days notice (permanent)
    //
    // SAFETY:
    //   - Snapshot taken before any withdrawal
    //   - Holder balances + LP state preserved
    //   - Restore capability for relaunch
    // =========================================================================

    /// Create pool and lock LP atomically
    pub fn create_pool_and_lock(
        ctx: Context<CreatePoolAndLock>,
        sol_amount: u64,
        token_amount: u64,
        timelock_seconds: Option<i64>,
        max_withdrawal_bps: Option<u16>,
    ) -> Result<()> {
        instructions::lp_lock::create_pool_and_lock_handler(
            ctx, sol_amount, token_amount, timelock_seconds, max_withdrawal_bps
        )
    }

    /// Take manual snapshot of LP state
    pub fn take_lp_snapshot(
        ctx: Context<TakeSnapshot>,
        reason: [u8; 32],
        sol_reserve: u64,
        token_reserve: u64,
        total_supply: u64,
        holder_count: u32,
    ) -> Result<u64> {
        instructions::lp_lock::take_snapshot_handler(
            ctx, reason, sol_reserve, token_reserve, total_supply, holder_count
        )
    }

    /// Announce LP withdrawal (auto-takes snapshot, starts timelock)
    /// Timelock depends on current phase:
    ///   - Days 0-3: 12h
    ///   - Days 3-15: 15 days  
    ///   - Days 15+: 30 days
    pub fn announce_lp_withdrawal(
        ctx: Context<AnnounceWithdrawal>,
        amount: u64,
        recipient: Pubkey,
        reason: [u8; 64],
    ) -> Result<()> {
        instructions::lp_lock::announce_withdrawal_handler(ctx, amount, recipient, reason)
    }

    /// Execute LP withdrawal (after timelock passes)
    pub fn execute_lp_withdrawal(
        ctx: Context<ExecuteWithdrawal>,
        slot: u8,
    ) -> Result<()> {
        instructions::lp_lock::execute_withdrawal_handler(ctx, slot)
    }

    /// Cancel pending LP withdrawal
    pub fn cancel_lp_withdrawal(
        ctx: Context<CancelWithdrawal>,
        slot: u8,
    ) -> Result<()> {
        instructions::lp_lock::cancel_withdrawal_handler(ctx, slot)
    }

    /// Restore LP from snapshot (for relaunch)
    /// Restores LP to vault and marks snapshot as used
    pub fn restore_from_snapshot(
        ctx: Context<RestoreFromSnapshot>,
        snapshot_id: u64,
        lp_amount: u64,
    ) -> Result<()> {
        instructions::lp_lock::restore_from_snapshot_handler(ctx, snapshot_id, lp_amount)
    }

    /// Transfer LP lock admin (to DAO)
    pub fn transfer_lp_lock_admin(ctx: Context<TransferAdmin>) -> Result<()> {
        instructions::lp_lock::transfer_admin_handler(ctx)
    }

    /// Get LP lock status
    pub fn get_lp_lock_status(ctx: Context<GetLockStatus>) -> Result<()> {
        instructions::lp_lock::get_lock_status_handler(ctx)
    }
}

// =============================================================================
// ERRORS
// =============================================================================

#[error_code]
pub enum ParadoxError {
    #[msg("Transfer fee out of allowed range (100-300 bps)")]
    InvalidTransferFee,

    #[msg("Fee shares must sum to 10000 bps (100%)")]
    InvalidFeeShares,

    #[msg("Cliff period not yet passed")]
    CliffNotPassed,

    #[msg("Cooldown period not yet passed")]
    CooldownNotPassed,

    #[msg("Timelock not yet expired")]
    TimelockNotExpired,

    #[msg("Unlock amount exceeds allowed rate")]
    UnlockRateExceeded,

    #[msg("Insufficient accumulated fees")]
    InsufficientFees,

    #[msg("LP Growth is locked")]
    LpGrowthLocked,

    #[msg("Armageddon mode is active")]
    ArmageddonActive,

    #[msg("Not in Armageddon mode")]
    NotInArmageddon,

    #[msg("LP not sufficiently recovered")]
    LpNotRecovered,

    #[msg("Unauthorized")]
    Unauthorized,

    #[msg("DAO spending limit exceeded")]
    DaoSpendingLimitExceeded,

    #[msg("Invalid Armageddon level")]
    InvalidArmageddonLevel,

    #[msg("Math overflow")]
    MathOverflow,

    #[msg("Timelock too short (minimum 24 hours)")]
    TimelockTooShort,

    #[msg("Withdrawal amount exceeds maximum allowed")]
    WithdrawalAmountExceeded,

    #[msg("Insufficient LP tokens locked")]
    InsufficientLpTokens,

    #[msg("Too many pending withdrawals (max 3)")]
    TooManyPendingWithdrawals,

    #[msg("Invalid withdrawal slot")]
    InvalidWithdrawalSlot,

    #[msg("No active withdrawal in this slot")]
    NoActiveWithdrawal,

    #[msg("Invalid vault account")]
    InvalidVault,

    #[msg("Emergency window still open")]
    EmergencyWindowStillOpen,

    #[msg("Emergency withdrawal already used")]
    EmergencyAlreadyUsed,

    #[msg("Emergency window closed")]
    EmergencyWindowClosed,

    #[msg("Already finalized")]
    AlreadyFinalized,

    #[msg("Amount is below minimum transfer threshold")]
    AmountBelowMinimum,

    #[msg("Fee change timelock not expired")]
    FeeChangeTimelockNotExpired,

    #[msg("No pending fee change")]
    NoPendingFeeChange,

    #[msg("Fee change not yet announced")]
    FeeChangeNotAnnounced,

    #[msg("Snapshot data required (reserves cannot all be zero)")]
    SnapshotDataRequired,

    #[msg("No fees to harvest")]
    NoFeesToHarvest,

    #[msg("Pool not initialized")]
    PoolNotInitialized,
}

// =============================================================================
// EVENTS
// =============================================================================

#[event]
pub struct TokenConfigInitialized {
    pub mint: Pubkey,
    pub transfer_fee_bps: u16,
    pub lp_share_bps: u16,
    pub burn_share_bps: u16,
    pub treasury_share_bps: u16,
}

#[event]
pub struct FeeChangeAnnounced {
    pub mint: Pubkey,
    pub old_fee_bps: u16,
    pub new_fee_bps: u16,
    pub activate_time: i64,
}

#[event]
pub struct TransferFeeUpdated {
    pub mint: Pubkey,
    pub old_fee_bps: u16,
    pub new_fee_bps: u16,
}

#[event]
pub struct FeeChangeCancelled {
    pub mint: Pubkey,
    pub cancelled_fee_bps: u16,
}

#[event]
pub struct LpGrowthInitialized {
    pub mint: Pubkey,
    pub lp_pool: Pubkey,
    pub min_fee_threshold: u64,
}

#[event]
pub struct LpGrowthExecuted {
    pub mint: Pubkey,
    pub sol_added: u64,
    pub tokens_minted: u64,
    pub new_lp_value: u64,
}

#[event]
pub struct LpGrowthLocked {
    pub mint: Pubkey,
    pub locked_by: Pubkey,
    pub reason: String,
}

#[event]
pub struct LpGrowthUnlocked {
    pub mint: Pubkey,
    pub unlocked_by: Pubkey,
}

#[event]
pub struct DevVestingInitialized {
    pub dev: Pubkey,
    pub mint: Pubkey,
    pub total_allocation: u64,
    pub liquid_at_tge: u64,
    pub cliff_seconds: i64,
    pub vesting_seconds: i64,
}

#[event]
pub struct DevUnlockRequested {
    pub dev: Pubkey,
    pub amount: u64,
    pub unlock_time: i64,
}

#[event]
pub struct DevUnlockExecuted {
    pub dev: Pubkey,
    pub amount: u64,
    pub remaining_locked: u64,
}

#[event]
pub struct DaoWithdrawalProposed {
    pub proposer: Pubkey,
    pub amount: u64,
    pub recipient: Pubkey,
    pub reason: String,
    pub execute_after: i64,
}

#[event]
pub struct DaoWithdrawalExecuted {
    pub recipient: Pubkey,
    pub amount: u64,
}

#[event]
pub struct ArmageddonTriggered {
    pub level: u8,
    pub lp_drop_percent: u8,
    pub response: String,
}

#[event]
pub struct ArmageddonRecovered {
    pub previous_level: u8,
    pub lp_recovery_percent: u8,
}

#[event]
pub struct FeesDistributed {
    pub total_fees: u64,
    pub to_lp: u64,
    pub burned: u64,
    pub to_treasury: u64,
}

// LP Lock Events

#[event]
pub struct LpLockCreated {
    pub mint: Pubkey,
    pub lp_pool: Pubkey,
    pub lp_tokens_locked: u64,
    pub timelock_seconds: i64,
    pub max_withdrawal_bps: u16,
    pub admin: Pubkey,
}

#[event]
pub struct LpWithdrawalAnnounced {
    pub mint: Pubkey,
    pub amount: u64,
    pub recipient: Pubkey,
    pub reason: String,
    pub announced_at: i64,
    pub execute_after: i64,
    pub slot: u8,
}

#[event]
pub struct LpWithdrawalExecuted {
    pub mint: Pubkey,
    pub amount: u64,
    pub recipient: Pubkey,
    pub executed_by: Pubkey,
    pub time_waited: i64,
    pub remaining_locked: u64,
}

#[event]
pub struct LpWithdrawalCancelled {
    pub mint: Pubkey,
    pub amount: u64,
    pub recipient: Pubkey,
    pub cancelled_by: Pubkey,
    pub slot: u8,
}

#[event]
pub struct LpLockFinalized {
    pub mint: Pubkey,
    pub lp_pool: Pubkey,
    pub lp_tokens_locked: u64,
    pub finalized_at: i64,
    pub finalized_by: Pubkey,
}

#[event]
pub struct LpEmergencyWithdrawal {
    pub mint: Pubkey,
    pub creator: Pubkey,
    pub lp_amount: u64,
    pub reason: String,
    pub timestamp: i64,
}

#[event]
pub struct FeesHarvested {
    pub mint: Pubkey,
    pub amount: u64,
    pub harvested_by: Pubkey,
    pub destination: Pubkey,
}

