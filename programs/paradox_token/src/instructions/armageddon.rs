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
    ArmageddonTriggered,
    ArmageddonRecovered,
};

#[derive(Accounts)]
pub struct TriggerArmageddon<'info> {
    pub admin: Signer<'info>,
    
    #[account(mut)]
    pub token_config: Account<'info, TokenConfig>,
    
    #[account(mut)]
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
            config.transfer_fee_bps = 300; // 3%
            state.emergency_lp_share_bps = 9000; // 90%
        },
        2 => {
            // DEFCON 2: Above + Treasury injection
            config.transfer_fee_bps = 300;
            state.emergency_lp_share_bps = 9000;
            // DEV: Trigger treasury injection here
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

#[derive(Accounts)]
pub struct RecoverArmageddon<'info> {
    pub admin: Signer<'info>,
    
    #[account(mut)]
    pub token_config: Account<'info, TokenConfig>,
    
    #[account(mut)]
    pub armageddon_state: Account<'info, ArmageddonState>,
}

pub fn recover_handler(ctx: Context<RecoverArmageddon>) -> Result<()> {
    let config = &mut ctx.accounts.token_config;
    let state = &mut ctx.accounts.armageddon_state;
    
    require!(state.level > 0, ParadoxError::NotInArmageddon);
    
    // DEV: Add LP recovery check here
    // require!(state.can_recover(current_lp_value), ParadoxError::LpNotRecovered);
    
    let previous_level = state.level;
    
    // Reset to normal
    state.level = 0;
    state.trading_paused = false;
    config.armageddon_level = 0;
    
    // DEV: Restore normal fee rate based on schedule
    // config.transfer_fee_bps = calculate_scheduled_fee()?;
    
    emit!(ArmageddonRecovered {
        previous_level,
        lp_recovery_percent: 120, // Example: recovered to 120% of trigger
    });
    
    Ok(())
}

