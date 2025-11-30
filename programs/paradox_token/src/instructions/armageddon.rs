/**
 * Armageddon Mode Instructions
 * 
 * Emergency response system for extreme LP drops.
 * 
 * Made by LabsX402 for Solana
 * https://x.com/LabsX402
 */

use anchor_lang::prelude::*;

use crate::{
    state::{ArmageddonState, TokenConfig},
    ParadoxError,
    TOKEN_CONFIG_SEED,
    ArmageddonTriggered,
    ArmageddonRecovered,
};

/// Seed for ArmageddonState PDA
pub const ARMAGEDDON_SEED: &[u8] = b"armageddon";

// =============================================================================
// INIT ARMAGEDDON STATE
// =============================================================================

#[derive(Accounts)]
pub struct InitArmageddon<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    
    #[account(
        seeds = [TOKEN_CONFIG_SEED, token_config.mint.as_ref()],
        bump = token_config.bump,
        has_one = admin @ ParadoxError::Unauthorized,
    )]
    pub token_config: Account<'info, TokenConfig>,
    
    #[account(
        init,
        payer = admin,
        space = ArmageddonState::LEN,
        seeds = [ARMAGEDDON_SEED, token_config.key().as_ref()],
        bump,
    )]
    pub armageddon_state: Account<'info, ArmageddonState>,
    
    pub system_program: Program<'info, System>,
}

pub fn init_armageddon_handler(ctx: Context<InitArmageddon>) -> Result<()> {
    let state = &mut ctx.accounts.armageddon_state;
    
    state.token_config = ctx.accounts.token_config.key();
    state.level = 0;
    state.triggered_at = 0;
    state.lp_value_at_trigger = 0;
    state.baseline_lp_value = 0;
    state.trigger_authority = ctx.accounts.admin.key();
    state.recovery_authority = ctx.accounts.admin.key();
    state.recovery_threshold_bps = 12000; // 120%
    state.emergency_fee_bps = 300; // 3%
    state.emergency_lp_share_bps = 9000; // 90%
    state.trading_paused = false;
    state.max_pause_duration = 24 * 60 * 60; // 24h max
    state.bump = ctx.bumps.armageddon_state;
    
    msg!("Armageddon state initialized");
    Ok(())
}

// =============================================================================
// TRIGGER ARMAGEDDON
// =============================================================================

#[derive(Accounts)]
pub struct TriggerArmageddon<'info> {
    #[account(
        constraint = admin.key() == token_config.admin @ ParadoxError::Unauthorized
    )]
    pub admin: Signer<'info>,
    
    #[account(
        mut,
        seeds = [TOKEN_CONFIG_SEED, token_config.mint.as_ref()],
        bump = token_config.bump,
    )]
    pub token_config: Account<'info, TokenConfig>,
    
    #[account(
        mut,
        seeds = [ARMAGEDDON_SEED, token_config.key().as_ref()],
        bump = armageddon_state.bump,
        constraint = armageddon_state.token_config == token_config.key() @ ParadoxError::Unauthorized,
    )]
    pub armageddon_state: Account<'info, ArmageddonState>,
}

pub fn trigger_handler(ctx: Context<TriggerArmageddon>, level: u8) -> Result<()> {
    require!(level >= 1 && level <= 3, ParadoxError::InvalidArmageddonLevel);
    
    let config = &mut ctx.accounts.token_config;
    let state = &mut ctx.accounts.armageddon_state;
    let clock = Clock::get()?;
    
    // Set Armageddon level
    state.level = level;
    state.triggered_at = clock.unix_timestamp;
    config.armageddon_level = level;
    
    // Apply emergency measures based on level
    match level {
        1 => {
            // DEFCON 3: Max fees, high LP share
            config.transfer_fee_bps = 300;
            state.emergency_lp_share_bps = 9000;
        },
        2 => {
            // DEFCON 2: Above + Treasury injection prep
            config.transfer_fee_bps = 300;
            state.emergency_lp_share_bps = 9000;
        },
        3 => {
            // DEFCON 1: Above + Trading slowdown
            config.transfer_fee_bps = 300;
            state.emergency_lp_share_bps = 9000;
            state.trading_paused = true;
        },
        _ => {}
    }
    
    emit!(ArmageddonTriggered {
        level,
        lp_drop_percent: ArmageddonState::get_threshold(level),
        response: ArmageddonState::get_response(level).to_string(),
    });
    
    Ok(())
}

// =============================================================================
// RECOVER FROM ARMAGEDDON
// =============================================================================

#[derive(Accounts)]
pub struct RecoverArmageddon<'info> {
    #[account(
        constraint = admin.key() == token_config.admin @ ParadoxError::Unauthorized
    )]
    pub admin: Signer<'info>,
    
    #[account(
        mut,
        seeds = [TOKEN_CONFIG_SEED, token_config.mint.as_ref()],
        bump = token_config.bump,
    )]
    pub token_config: Account<'info, TokenConfig>,
    
    #[account(
        mut,
        seeds = [ARMAGEDDON_SEED, token_config.key().as_ref()],
        bump = armageddon_state.bump,
        constraint = armageddon_state.token_config == token_config.key() @ ParadoxError::Unauthorized,
    )]
    pub armageddon_state: Account<'info, ArmageddonState>,
}

pub fn recover_handler(ctx: Context<RecoverArmageddon>) -> Result<()> {
    let config = &mut ctx.accounts.token_config;
    let state = &mut ctx.accounts.armageddon_state;
    
    require!(state.level > 0, ParadoxError::NotInArmageddon);
    
    let previous_level = state.level;
    
    // Reset to normal
    state.level = 0;
    state.trading_paused = false;
    config.armageddon_level = 0;
    
    emit!(ArmageddonRecovered {
        previous_level,
        lp_recovery_percent: 120,
    });
    
    Ok(())
}
