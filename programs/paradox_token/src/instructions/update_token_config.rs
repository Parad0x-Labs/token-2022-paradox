/**
 * Update Token Config Instructions
 * 
 * Fee changes require 24h timelock to prevent front-running attacks.
 * 
 * Made by LabsX402 for Solana
 * https://x.com/LabsX402
 */

use anchor_lang::prelude::*;

use crate::{
    state::TokenConfig,
    ParadoxError,
    TOKEN_CONFIG_SEED,
    MIN_TRANSFER_FEE_BPS,
    MAX_TRANSFER_FEE_BPS,
    FEE_CHANGE_TIMELOCK_SECONDS,
    FeeChangeAnnounced,
    TransferFeeUpdated,
    FeeChangeCancelled,
};

// =============================================================================
// ANNOUNCE FEE CHANGE (starts 24h timelock)
// =============================================================================

#[derive(Accounts)]
pub struct AnnounceFeeChange<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    
    #[account(
        mut,
        seeds = [TOKEN_CONFIG_SEED, token_config.mint.as_ref()],
        bump = token_config.bump,
        has_one = admin @ ParadoxError::Unauthorized,
    )]
    pub token_config: Account<'info, TokenConfig>,
}

pub fn announce_fee_change_handler(
    ctx: Context<AnnounceFeeChange>,
    new_fee_bps: u16,
) -> Result<()> {
    // Validate new fee
    require!(
        new_fee_bps >= MIN_TRANSFER_FEE_BPS && new_fee_bps <= MAX_TRANSFER_FEE_BPS,
        ParadoxError::InvalidTransferFee
    );
    
    let config = &mut ctx.accounts.token_config;
    let clock = Clock::get()?;
    
    // Check if there's already a pending change
    require!(
        config.pending_fee_activate_time == 0 || clock.unix_timestamp >= config.pending_fee_cancel_time,
        ParadoxError::FeeChangeTimelockNotExpired
    );
    
    // Set pending fee change
    config.pending_fee_bps = new_fee_bps;
    config.pending_fee_activate_time = clock.unix_timestamp
        .checked_add(FEE_CHANGE_TIMELOCK_SECONDS)
        .ok_or(ParadoxError::MathOverflow)?;
    config.pending_fee_cancel_time = config.pending_fee_activate_time
        .checked_add(FEE_CHANGE_TIMELOCK_SECONDS)
        .ok_or(ParadoxError::MathOverflow)?;
    
    emit!(FeeChangeAnnounced {
        mint: config.mint,
        old_fee_bps: config.transfer_fee_bps,
        new_fee_bps,
        activate_time: config.pending_fee_activate_time,
    });
    
    msg!("Fee change announced: {} bps → {} bps (activates in 24h)", 
         config.transfer_fee_bps, new_fee_bps);
    
    Ok(())
}

// =============================================================================
// EXECUTE FEE CHANGE (after 24h timelock)
// =============================================================================

#[derive(Accounts)]
pub struct ExecuteFeeChange<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    
    #[account(
        mut,
        seeds = [TOKEN_CONFIG_SEED, token_config.mint.as_ref()],
        bump = token_config.bump,
        has_one = admin @ ParadoxError::Unauthorized,
    )]
    pub token_config: Account<'info, TokenConfig>,
}

pub fn execute_fee_change_handler(ctx: Context<ExecuteFeeChange>) -> Result<()> {
    let config = &mut ctx.accounts.token_config;
    let clock = Clock::get()?;
    
    // Check if there's a pending change
    require!(config.pending_fee_bps > 0, ParadoxError::NoPendingFeeChange);
    
    // Check if timelock has expired
    require!(
        clock.unix_timestamp >= config.pending_fee_activate_time,
        ParadoxError::FeeChangeTimelockNotExpired
    );
    
    // Check if cancel window has passed (can't execute after cancel window)
    require!(
        clock.unix_timestamp < config.pending_fee_cancel_time,
        ParadoxError::FeeChangeTimelockNotExpired
    );
    
    let old_fee = config.transfer_fee_bps;
    let new_fee = config.pending_fee_bps;
    
    // Execute the fee change
    config.transfer_fee_bps = new_fee;
    config.last_fee_update = clock.unix_timestamp;
    
    // Clear pending
    config.pending_fee_bps = 0;
    config.pending_fee_activate_time = 0;
    config.pending_fee_cancel_time = 0;
    
    emit!(TransferFeeUpdated {
        mint: config.mint,
        old_fee_bps: old_fee,
        new_fee_bps: new_fee,
    });
    
    msg!("Fee change executed: {} bps → {} bps", old_fee, new_fee);
    
    Ok(())
}

// =============================================================================
// CANCEL FEE CHANGE (before execution)
// =============================================================================

#[derive(Accounts)]
pub struct CancelFeeChange<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    
    #[account(
        mut,
        seeds = [TOKEN_CONFIG_SEED, token_config.mint.as_ref()],
        bump = token_config.bump,
        has_one = admin @ ParadoxError::Unauthorized,
    )]
    pub token_config: Account<'info, TokenConfig>,
}

pub fn cancel_fee_change_handler(ctx: Context<CancelFeeChange>) -> Result<()> {
    let config = &mut ctx.accounts.token_config;
    let clock = Clock::get()?;
    
    // Check if there's a pending change
    require!(config.pending_fee_bps > 0, ParadoxError::NoPendingFeeChange);
    
    // Can cancel before activate_time or after cancel_time
    require!(
        clock.unix_timestamp < config.pending_fee_activate_time || 
        clock.unix_timestamp >= config.pending_fee_cancel_time,
        ParadoxError::FeeChangeTimelockNotExpired
    );
    
    let cancelled_fee = config.pending_fee_bps;
    
    // Clear pending
    config.pending_fee_bps = 0;
    config.pending_fee_activate_time = 0;
    config.pending_fee_cancel_time = 0;
    
    emit!(FeeChangeCancelled {
        mint: config.mint,
        cancelled_fee_bps: cancelled_fee,
    });
    
    msg!("Fee change cancelled");
    
    Ok(())
}
