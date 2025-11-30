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
pub const DEV_VESTING_SEED: &[u8] = b"dev_vesting";
pub const DAO_TREASURY_SEED: &[u8] = b"dao_treasury";
pub const FEE_VAULT_SEED: &[u8] = b"fee_vault";

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

    /// Update transfer fee (governance only)
    pub fn update_transfer_fee(
        ctx: Context<UpdateTokenConfig>,
        new_fee_bps: u16,
    ) -> Result<()> {
        instructions::update_token_config::update_fee(ctx, new_fee_bps)
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
pub struct TransferFeeUpdated {
    pub mint: Pubkey,
    pub old_fee_bps: u16,
    pub new_fee_bps: u16,
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

