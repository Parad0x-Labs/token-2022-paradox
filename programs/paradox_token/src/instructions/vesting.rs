/**
 * Vesting Instructions
 * 
 * Made by LabsX402 for Solana
 * https://x.com/LabsX402
 */

use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount, Mint, Transfer, transfer};

use crate::{
    state::DevVestingVault,
    ParadoxError,
    DEV_VESTING_SEED,
    DevVestingInitialized,
    DevUnlockRequested,
    DevUnlockExecuted,
    DEFAULT_COOLDOWN_SECONDS,
    DEFAULT_TIMELOCK_SECONDS,
    YEAR1_UNLOCK_RATE_BPS,
};

// =============================================================================
// INIT DEV VESTING
// =============================================================================

#[derive(Accounts)]
pub struct InitDevVesting<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    
    pub dev: Signer<'info>,
    
    pub mint: Account<'info, Mint>,
    
    #[account(
        init,
        payer = admin,
        space = DevVestingVault::LEN,
        seeds = [DEV_VESTING_SEED, dev.key().as_ref(), mint.key().as_ref()],
        bump,
    )]
    pub vault: Account<'info, DevVestingVault>,
    
    #[account(mut)]
    pub vault_token_account: Account<'info, TokenAccount>,
    
    #[account(mut)]
    pub source_token_account: Account<'info, TokenAccount>,
    
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn init_dev_handler(
    ctx: Context<InitDevVesting>,
    total_allocation: u64,
    liquid_at_tge: u64,
    cliff_seconds: i64,
    vesting_seconds: i64,
) -> Result<()> {
    let vault = &mut ctx.accounts.vault;
    let clock = Clock::get()?;
    
    let locked_amount = total_allocation.checked_sub(liquid_at_tge).unwrap();
    
    vault.dev = ctx.accounts.dev.key();
    vault.mint = ctx.accounts.mint.key();
    vault.token_account = ctx.accounts.vault_token_account.key();
    vault.total_allocation = total_allocation;
    vault.liquid_at_tge = liquid_at_tge;
    vault.total_locked = locked_amount;
    vault.locked_amount = locked_amount;
    vault.pending_amount = 0;
    vault.initialized_at = clock.unix_timestamp;
    vault.cliff_seconds = cliff_seconds;
    vault.vesting_seconds = vesting_seconds;
    vault.last_request_time = 0;
    vault.unlock_time = 0;
    vault.cooldown_seconds = DEFAULT_COOLDOWN_SECONDS;
    vault.timelock_seconds = DEFAULT_TIMELOCK_SECONDS;
    vault.unlock_rate_bps = YEAR1_UNLOCK_RATE_BPS;
    vault.total_unlocked = 0;
    vault.bump = ctx.bumps.vault;
    
    // Transfer locked tokens to vault
    transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.source_token_account.to_account_info(),
                to: ctx.accounts.vault_token_account.to_account_info(),
                authority: ctx.accounts.admin.to_account_info(),
            },
        ),
        locked_amount,
    )?;
    
    emit!(DevVestingInitialized {
        dev: vault.dev,
        mint: vault.mint,
        total_allocation,
        liquid_at_tge,
        cliff_seconds,
        vesting_seconds,
    });
    
    Ok(())
}

// =============================================================================
// REQUEST DEV UNLOCK
// =============================================================================

#[derive(Accounts)]
pub struct RequestDevUnlock<'info> {
    #[account(mut)]
    pub dev: Signer<'info>,
    
    #[account(
        mut,
        seeds = [DEV_VESTING_SEED, dev.key().as_ref(), vault.mint.as_ref()],
        bump = vault.bump,
        has_one = dev @ ParadoxError::Unauthorized,
    )]
    pub vault: Account<'info, DevVestingVault>,
}

pub fn request_unlock_handler(ctx: Context<RequestDevUnlock>, amount: u64) -> Result<()> {
    let vault = &mut ctx.accounts.vault;
    let clock = Clock::get()?;
    
    // Check cliff
    require!(vault.cliff_passed(clock.unix_timestamp), ParadoxError::CliffNotPassed);
    
    // Check cooldown
    require!(vault.cooldown_passed(clock.unix_timestamp), ParadoxError::CooldownNotPassed);
    
    // Update unlock rate based on time
    vault.update_unlock_rate(clock.unix_timestamp);
    
    // Check amount doesn't exceed rate
    let max_unlockable = vault.max_unlockable();
    require!(amount <= max_unlockable, ParadoxError::UnlockRateExceeded);
    
    // Set pending unlock
    vault.pending_amount = amount;
    vault.last_request_time = clock.unix_timestamp;
    vault.unlock_time = clock.unix_timestamp + vault.timelock_seconds;
    
    emit!(DevUnlockRequested {
        dev: vault.dev,
        amount,
        unlock_time: vault.unlock_time,
    });
    
    Ok(())
}

// =============================================================================
// EXECUTE DEV UNLOCK
// =============================================================================

#[derive(Accounts)]
pub struct ExecuteDevUnlock<'info> {
    #[account(mut)]
    pub dev: Signer<'info>,
    
    #[account(
        mut,
        seeds = [DEV_VESTING_SEED, dev.key().as_ref(), vault.mint.as_ref()],
        bump = vault.bump,
        has_one = dev @ ParadoxError::Unauthorized,
    )]
    pub vault: Account<'info, DevVestingVault>,
    
    #[account(mut)]
    pub vault_token_account: Account<'info, TokenAccount>,
    
    #[account(mut)]
    pub dev_token_account: Account<'info, TokenAccount>,
    
    pub token_program: Program<'info, Token>,
}

pub fn execute_unlock_handler(ctx: Context<ExecuteDevUnlock>) -> Result<()> {
    let vault = &mut ctx.accounts.vault;
    let clock = Clock::get()?;
    
    // Check timelock expired
    require!(vault.timelock_expired(clock.unix_timestamp), ParadoxError::TimelockNotExpired);
    require!(vault.pending_amount > 0, ParadoxError::InsufficientFees);
    
    let amount = vault.pending_amount;
    
    // Transfer tokens
    let seeds = &[
        DEV_VESTING_SEED,
        vault.dev.as_ref(),
        vault.mint.as_ref(),
        &[vault.bump],
    ];
    
    transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.vault_token_account.to_account_info(),
                to: ctx.accounts.dev_token_account.to_account_info(),
                authority: vault.to_account_info(),
            },
            &[seeds],
        ),
        amount,
    )?;
    
    // Update state
    vault.locked_amount -= amount;
    vault.pending_amount = 0;
    vault.total_unlocked += amount;
    
    emit!(DevUnlockExecuted {
        dev: vault.dev,
        amount,
        remaining_locked: vault.locked_amount,
    });
    
    Ok(())
}

