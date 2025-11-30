/**
 * Update Token Config Instructions
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
    TransferFeeUpdated,
};

#[derive(Accounts)]
pub struct UpdateTokenConfig<'info> {
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

pub fn update_fee(ctx: Context<UpdateTokenConfig>, new_fee_bps: u16) -> Result<()> {
    // Validate new fee
    require!(
        new_fee_bps >= MIN_TRANSFER_FEE_BPS && new_fee_bps <= MAX_TRANSFER_FEE_BPS,
        ParadoxError::InvalidTransferFee
    );
    
    let config = &mut ctx.accounts.token_config;
    let clock = Clock::get()?;
    
    let old_fee = config.transfer_fee_bps;
    config.transfer_fee_bps = new_fee_bps;
    config.last_fee_update = clock.unix_timestamp;
    
    emit!(TransferFeeUpdated {
        mint: config.mint,
        old_fee_bps: old_fee,
        new_fee_bps,
    });
    
    Ok(())
}

