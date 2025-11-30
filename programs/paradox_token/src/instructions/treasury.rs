/**
 * DAO Treasury Instructions
 * 
 * Made by LabsX402 for Solana
 * https://x.com/LabsX402
 */

use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount, Transfer, transfer};

use crate::{
    state::DaoTreasuryVault,
    ParadoxError,
    DaoWithdrawalProposed,
    DaoWithdrawalExecuted,
};

/// Seed for DAO Treasury PDA
pub const DAO_TREASURY_SEED: &[u8] = b"dao_treasury";

// =============================================================================
// INIT DAO TREASURY
// =============================================================================

#[derive(Accounts)]
pub struct InitDaoTreasury<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    
    pub mint: anchor_spl::token::Mint,
    
    #[account(
        init,
        payer = admin,
        space = DaoTreasuryVault::LEN,
        seeds = [DAO_TREASURY_SEED, mint.key().as_ref()],
        bump,
    )]
    pub treasury: Account<'info, DaoTreasuryVault>,
    
    /// CHECK: Token account for treasury (created separately)
    pub token_account: UncheckedAccount<'info>,
    
    pub system_program: Program<'info, System>,
}

pub fn init_handler(
    ctx: Context<InitDaoTreasury>,
    governance: Pubkey,
    max_spend_bps_per_period: u16,
    period_seconds: i64,
) -> Result<()> {
    let treasury = &mut ctx.accounts.treasury;
    let clock = Clock::get()?;
    
    treasury.governance = governance;
    treasury.mint = ctx.accounts.mint.key();
    treasury.token_account = ctx.accounts.token_account.key();
    treasury.balance = 0;
    treasury.max_spend_bps_per_period = max_spend_bps_per_period;
    treasury.period_seconds = period_seconds;
    treasury.period_start = clock.unix_timestamp;
    treasury.spent_this_period = 0;
    treasury.pending_amount = 0;
    treasury.pending_recipient = Pubkey::default();
    treasury.pending_reason = [0u8; 128];
    treasury.pending_execute_after = 0;
    treasury.timelock_seconds = 48 * 60 * 60; // 48h default
    treasury.total_withdrawn = 0;
    treasury.bump = ctx.bumps.treasury;
    
    msg!("DAO Treasury initialized with governance: {}", governance);
    Ok(())
}

// =============================================================================
// PROPOSE DAO WITHDRAWAL
// =============================================================================

#[derive(Accounts)]
pub struct ProposeDaoWithdrawal<'info> {
    #[account(
        constraint = governance.key() == treasury.governance @ ParadoxError::Unauthorized
    )]
    pub governance: Signer<'info>,
    
    #[account(
        mut,
        seeds = [DAO_TREASURY_SEED, treasury.mint.as_ref()],
        bump = treasury.bump,
    )]
    pub treasury: Account<'info, DaoTreasuryVault>,
}

pub fn propose_handler(
    ctx: Context<ProposeDaoWithdrawal>,
    amount: u64,
    recipient: Pubkey,
    reason: String,
) -> Result<()> {
    let treasury = &mut ctx.accounts.treasury;
    let clock = Clock::get()?;
    
    // Reset period if needed
    if treasury.should_reset_period(clock.unix_timestamp) {
        treasury.reset_period(clock.unix_timestamp);
    }
    
    // Check spending limit
    require!(amount <= treasury.max_spendable(), ParadoxError::DaoSpendingLimitExceeded);
    
    // Set pending withdrawal
    treasury.pending_amount = amount;
    treasury.pending_recipient = recipient;
    
    // Copy reason (truncate if needed)
    let reason_bytes = reason.as_bytes();
    let copy_len = reason_bytes.len().min(128);
    treasury.pending_reason[..copy_len].copy_from_slice(&reason_bytes[..copy_len]);
    
    treasury.pending_execute_after = clock.unix_timestamp + treasury.timelock_seconds;
    
    emit!(DaoWithdrawalProposed {
        proposer: ctx.accounts.governance.key(),
        amount,
        recipient,
        reason,
        execute_after: treasury.pending_execute_after,
    });
    
    Ok(())
}

// =============================================================================
// EXECUTE DAO WITHDRAWAL
// =============================================================================

#[derive(Accounts)]
pub struct ExecuteDaoWithdrawal<'info> {
    pub executor: Signer<'info>,
    
    #[account(
        mut,
        seeds = [DAO_TREASURY_SEED, treasury.mint.as_ref()],
        bump = treasury.bump,
    )]
    pub treasury: Account<'info, DaoTreasuryVault>,
    
    #[account(
        mut,
        constraint = treasury_token_account.key() == treasury.token_account @ ParadoxError::InvalidVault,
    )]
    pub treasury_token_account: Account<'info, TokenAccount>,
    
    /// CHECK: Must match pending_recipient
    #[account(
        mut,
        constraint = recipient_token_account.key() == treasury.pending_recipient @ ParadoxError::Unauthorized,
    )]
    pub recipient_token_account: UncheckedAccount<'info>,
    
    pub token_program: Program<'info, Token>,
}

pub fn execute_handler(ctx: Context<ExecuteDaoWithdrawal>) -> Result<()> {
    let treasury = &mut ctx.accounts.treasury;
    let clock = Clock::get()?;
    
    // Check timelock
    require!(treasury.can_execute_withdrawal(clock.unix_timestamp), ParadoxError::TimelockNotExpired);
    
    let amount = treasury.pending_amount;
    let recipient = treasury.pending_recipient;
    
    // Transfer tokens
    let mint_key = treasury.mint;
    let seeds = &[
        DAO_TREASURY_SEED,
        mint_key.as_ref(),
        &[treasury.bump],
    ];
    
    transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.treasury_token_account.to_account_info(),
                to: ctx.accounts.recipient_token_account.to_account_info(),
                authority: treasury.to_account_info(),
            },
            &[seeds],
        ),
        amount,
    )?;
    
    // Update state
    treasury.spent_this_period = treasury.spent_this_period
        .checked_add(amount)
        .ok_or(ParadoxError::MathOverflow)?;
    treasury.total_withdrawn = treasury.total_withdrawn
        .checked_add(amount)
        .ok_or(ParadoxError::MathOverflow)?;
    treasury.balance = treasury.balance.saturating_sub(amount);
    
    // Clear pending
    treasury.pending_amount = 0;
    treasury.pending_recipient = Pubkey::default();
    treasury.pending_reason = [0u8; 128];
    treasury.pending_execute_after = 0;
    
    emit!(DaoWithdrawalExecuted {
        recipient,
        amount,
    });
    
    Ok(())
}
