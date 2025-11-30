/**
 * Initialize Token Config Instruction
 * 
 * Made by LabsX402 for Solana
 * https://x.com/LabsX402
 */

use anchor_lang::prelude::*;
use anchor_spl::token::Mint;

use crate::{
    state::TokenConfig,
    ParadoxError,
    TOKEN_CONFIG_SEED,
    MIN_TRANSFER_FEE_BPS,
    MAX_TRANSFER_FEE_BPS,
    TokenConfigInitialized,
};

#[derive(Accounts)]
pub struct InitTokenConfig<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    
    pub mint: Account<'info, Mint>,
    
    #[account(
        init,
        payer = admin,
        space = TokenConfig::LEN,
        seeds = [TOKEN_CONFIG_SEED, mint.key().as_ref()],
        bump,
    )]
    pub token_config: Account<'info, TokenConfig>,
    
    /// CHECK: Fee vault (created separately)
    pub fee_vault: UncheckedAccount<'info>,
    
    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<InitTokenConfig>,
    transfer_fee_bps: u16,
    lp_share_bps: u16,
    burn_share_bps: u16,
    treasury_share_bps: u16,
) -> Result<()> {
    // Validate transfer fee
    require!(
        transfer_fee_bps >= MIN_TRANSFER_FEE_BPS && transfer_fee_bps <= MAX_TRANSFER_FEE_BPS,
        ParadoxError::InvalidTransferFee
    );
    
    // Validate shares sum to 100%
    let total_shares = lp_share_bps as u32 + burn_share_bps as u32 + treasury_share_bps as u32;
    require!(total_shares == 10_000, ParadoxError::InvalidFeeShares);
    
    let config = &mut ctx.accounts.token_config;
    let clock = Clock::get()?;
    
    config.mint = ctx.accounts.mint.key();
    config.admin = ctx.accounts.admin.key();
    config.governance = ctx.accounts.admin.key(); // Initially same as admin
    config.transfer_fee_bps = transfer_fee_bps;
    config.lp_share_bps = lp_share_bps;
    config.burn_share_bps = burn_share_bps;
    config.treasury_share_bps = treasury_share_bps;
    config.fee_vault = ctx.accounts.fee_vault.key();
    config.total_fees_collected = 0;
    config.total_fees_distributed = 0;
    config.is_paused = false;
    config.armageddon_level = 0;
    config.last_fee_update = clock.unix_timestamp;
    config.bump = ctx.bumps.token_config;
    
    emit!(TokenConfigInitialized {
        mint: config.mint,
        transfer_fee_bps,
        lp_share_bps,
        burn_share_bps,
        treasury_share_bps,
    });
    
    Ok(())
}

