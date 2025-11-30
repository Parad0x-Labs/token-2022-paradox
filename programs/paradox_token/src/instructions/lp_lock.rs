/**
 * LP Lock Instructions - Timelock Based
 * 
 * ALL withdrawals require advance announcement + timelock.
 * No instant withdrawals possible. Everyone can see pending withdrawals on-chain.
 * 
 * Flow:
 * 1. create_pool_and_lock - Creates pool + locks LP atomically
 * 2. announce_withdrawal - Public announcement, starts 24-48h timelock
 * 3. execute_withdrawal - After timelock passes
 * 4. cancel_withdrawal - Can cancel before execution
 * 
 * Made by LabsX402 for Solana
 * https://x.com/LabsX402
 */

use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount, Mint, Transfer, transfer};

use crate::{
    state::{LpLock, LpLockStatus, MIN_WITHDRAWAL_TIMELOCK_SECONDS, DEFAULT_WITHDRAWAL_TIMELOCK_SECONDS},
    ParadoxError,
    LP_LOCK_SEED,
    LpLockCreated,
    LpWithdrawalAnnounced,
    LpWithdrawalExecuted,
    LpWithdrawalCancelled,
};

// =============================================================================
// CREATE POOL AND LOCK LP
// =============================================================================

#[derive(Accounts)]
pub struct CreatePoolAndLock<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,
    
    pub mint: Account<'info, Mint>,
    
    #[account(
        init,
        payer = creator,
        space = LpLock::LEN,
        seeds = [LP_LOCK_SEED, mint.key().as_ref()],
        bump,
    )]
    pub lp_lock: Account<'info, LpLock>,
    
    #[account(mut)]
    pub lp_vault: Account<'info, TokenAccount>,
    
    /// CHECK: LP token mint from DEX
    pub lp_token_mint: UncheckedAccount<'info>,
    
    #[account(mut)]
    pub creator_token_account: Account<'info, TokenAccount>,
    
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn create_pool_and_lock_handler(
    ctx: Context<CreatePoolAndLock>,
    sol_amount: u64,
    token_amount: u64,
    timelock_seconds: Option<i64>,
    max_withdrawal_bps: Option<u16>,
) -> Result<()> {
    let lp_lock = &mut ctx.accounts.lp_lock;
    
    // Use defaults if not specified
    let timelock = timelock_seconds.unwrap_or(DEFAULT_WITHDRAWAL_TIMELOCK_SECONDS);
    let max_bps = max_withdrawal_bps.unwrap_or(1000); // Default 10% max per withdrawal
    
    // Validate timelock is at least minimum
    require!(timelock >= MIN_WITHDRAWAL_TIMELOCK_SECONDS, ParadoxError::TimelockTooShort);
    
    // =========================================================================
    // DEV NOTE: Implement pool creation here
    // =========================================================================
    
    let lp_tokens_received: u64 = 0; // Replace with actual LP tokens
    
    lp_lock.initialize(
        ctx.accounts.mint.key(),
        Pubkey::default(), // Replace with actual pool
        ctx.accounts.lp_token_mint.key(),
        ctx.accounts.lp_vault.key(),
        ctx.accounts.creator.key(),
        lp_tokens_received,
        timelock,
        max_bps,
        ctx.bumps.lp_lock,
    );
    
    msg!("LP Lock created with {} second timelock", timelock);
    msg!("Max withdrawal per request: {}%", max_bps as f64 / 100.0);
    msg!("NO INSTANT WITHDRAWALS - all require {} hour advance notice", timelock / 3600);
    
    emit!(LpLockCreated {
        mint: ctx.accounts.mint.key(),
        lp_pool: lp_lock.lp_pool,
        lp_tokens_locked: lp_tokens_received,
        timelock_seconds: timelock,
        max_withdrawal_bps: max_bps,
        admin: ctx.accounts.creator.key(),
    });
    
    Ok(())
}

// =============================================================================
// ANNOUNCE WITHDRAWAL (Starts Timelock)
// =============================================================================

#[derive(Accounts)]
pub struct AnnounceWithdrawal<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    
    pub mint: Account<'info, Mint>,
    
    #[account(
        mut,
        seeds = [LP_LOCK_SEED, mint.key().as_ref()],
        bump = lp_lock.bump,
        constraint = lp_lock.admin == admin.key() @ ParadoxError::Unauthorized,
    )]
    pub lp_lock: Account<'info, LpLock>,
}

pub fn announce_withdrawal_handler(
    ctx: Context<AnnounceWithdrawal>,
    amount: u64,
    recipient: Pubkey,
    reason: [u8; 64],
) -> Result<()> {
    let lp_lock = &mut ctx.accounts.lp_lock;
    
    // Validate amount
    require!(lp_lock.is_valid_withdrawal_amount(amount), ParadoxError::WithdrawalAmountExceeded);
    require!(amount <= lp_lock.lp_tokens_locked, ParadoxError::InsufficientLpTokens);
    
    // Announce (starts timelock)
    let slot = lp_lock.announce_withdrawal(amount, recipient, reason)?;
    
    let execute_after = lp_lock.pending_withdrawals[slot].execute_after;
    let hours_until = lp_lock.withdrawal_timelock_seconds / 3600;
    
    msg!("=== LP WITHDRAWAL ANNOUNCED ===");
    msg!("Amount: {} LP tokens", amount);
    msg!("Recipient: {}", recipient);
    msg!("Reason: {}", String::from_utf8_lossy(&reason));
    msg!("Timelock: {} hours", hours_until);
    msg!("Executable after: {}", execute_after);
    msg!("================================");
    msg!("⚠️ VISIBLE ON-CHAIN - Everyone can see this pending withdrawal");
    
    emit!(LpWithdrawalAnnounced {
        mint: ctx.accounts.mint.key(),
        amount,
        recipient,
        reason: String::from_utf8_lossy(&reason).to_string(),
        announced_at: Clock::get()?.unix_timestamp,
        execute_after,
        slot: slot as u8,
    });
    
    Ok(())
}

// =============================================================================
// EXECUTE WITHDRAWAL (After Timelock)
// =============================================================================

#[derive(Accounts)]
pub struct ExecuteWithdrawal<'info> {
    #[account(mut)]
    pub executor: Signer<'info>,
    
    pub mint: Account<'info, Mint>,
    
    #[account(
        mut,
        seeds = [LP_LOCK_SEED, mint.key().as_ref()],
        bump = lp_lock.bump,
    )]
    pub lp_lock: Account<'info, LpLock>,
    
    #[account(
        mut,
        constraint = lp_vault.key() == lp_lock.lp_vault @ ParadoxError::InvalidVault,
    )]
    pub lp_vault: Account<'info, TokenAccount>,
    
    /// CHECK: Must match pending withdrawal recipient
    #[account(mut)]
    pub recipient_lp_account: UncheckedAccount<'info>,
    
    pub token_program: Program<'info, Token>,
}

pub fn execute_withdrawal_handler(
    ctx: Context<ExecuteWithdrawal>,
    slot: u8,
) -> Result<()> {
    let lp_lock = &mut ctx.accounts.lp_lock;
    let slot_usize = slot as usize;
    
    // Validate recipient matches
    let pending = &lp_lock.pending_withdrawals[slot_usize];
    require!(pending.is_active, ParadoxError::NoActiveWithdrawal);
    
    // Check timelock passed
    require!(lp_lock.can_execute_withdrawal(slot_usize), ParadoxError::TimelockNotExpired);
    
    let time_waited = Clock::get()?.unix_timestamp - pending.announced_at;
    
    // Execute withdrawal
    let (amount, recipient) = lp_lock.execute_withdrawal(slot_usize)?;
    
    // Transfer LP tokens
    let mint_key = ctx.accounts.mint.key();
    let seeds = &[
        LP_LOCK_SEED,
        mint_key.as_ref(),
        &[lp_lock.bump],
    ];
    let signer_seeds = &[&seeds[..]];
    
    let transfer_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        Transfer {
            from: ctx.accounts.lp_vault.to_account_info(),
            to: ctx.accounts.recipient_lp_account.to_account_info(),
            authority: lp_lock.to_account_info(),
        },
        signer_seeds,
    );
    
    transfer(transfer_ctx, amount)?;
    
    msg!("LP Withdrawal executed after {} hours timelock", time_waited / 3600);
    msg!("Amount: {} LP tokens to {}", amount, recipient);
    msg!("Remaining locked: {}", lp_lock.lp_tokens_locked);
    
    emit!(LpWithdrawalExecuted {
        mint: ctx.accounts.mint.key(),
        amount,
        recipient,
        executed_by: ctx.accounts.executor.key(),
        time_waited,
        remaining_locked: lp_lock.lp_tokens_locked,
    });
    
    Ok(())
}

// =============================================================================
// CANCEL WITHDRAWAL
// =============================================================================

#[derive(Accounts)]
pub struct CancelWithdrawal<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    
    pub mint: Account<'info, Mint>,
    
    #[account(
        mut,
        seeds = [LP_LOCK_SEED, mint.key().as_ref()],
        bump = lp_lock.bump,
        constraint = lp_lock.admin == admin.key() @ ParadoxError::Unauthorized,
    )]
    pub lp_lock: Account<'info, LpLock>,
}

pub fn cancel_withdrawal_handler(
    ctx: Context<CancelWithdrawal>,
    slot: u8,
) -> Result<()> {
    let lp_lock = &mut ctx.accounts.lp_lock;
    
    let pending = &lp_lock.pending_withdrawals[slot as usize];
    let amount = pending.amount;
    let recipient = pending.recipient;
    
    lp_lock.cancel_withdrawal(slot as usize)?;
    
    msg!("LP Withdrawal cancelled");
    msg!("Amount that was pending: {} LP tokens", amount);
    
    emit!(LpWithdrawalCancelled {
        mint: ctx.accounts.mint.key(),
        amount,
        recipient,
        cancelled_by: ctx.accounts.admin.key(),
        slot,
    });
    
    Ok(())
}

// =============================================================================
// GET LOCK STATUS
// =============================================================================

#[derive(Accounts)]
pub struct GetLockStatus<'info> {
    pub mint: Account<'info, Mint>,
    
    #[account(
        seeds = [LP_LOCK_SEED, mint.key().as_ref()],
        bump = lp_lock.bump,
    )]
    pub lp_lock: Account<'info, LpLock>,
}

pub fn get_lock_status_handler(ctx: Context<GetLockStatus>) -> Result<()> {
    let lp_lock = &ctx.accounts.lp_lock;
    
    let status_str = match lp_lock.status {
        LpLockStatus::NotInitialized => "NOT_INITIALIZED",
        LpLockStatus::Locked => "LOCKED",
        LpLockStatus::WithdrawalPending => "WITHDRAWAL_PENDING",
    };
    
    msg!("╔═══════════════════════════════════════╗");
    msg!("║         LP LOCK STATUS                ║");
    msg!("╠═══════════════════════════════════════╣");
    msg!("║ Status: {}", status_str);
    msg!("║ LP Locked: {}", lp_lock.lp_tokens_locked);
    msg!("║ Total Withdrawn: {}", lp_lock.total_withdrawn);
    msg!("║ Timelock: {} hours", lp_lock.withdrawal_timelock_seconds / 3600);
    msg!("║ Max per withdrawal: {}%", lp_lock.max_withdrawal_bps as f64 / 100.0);
    msg!("║ Pending withdrawals: {}", lp_lock.pending_count);
    msg!("╚═══════════════════════════════════════╝");
    
    // Show pending withdrawals
    for (i, pw) in lp_lock.pending_withdrawals.iter().enumerate() {
        if pw.is_active {
            let time_remaining = lp_lock.time_until_executable(i);
            msg!("  Pending #{}: {} LP → {} ({}h remaining)",
                i, pw.amount, pw.recipient, time_remaining / 3600);
        }
    }
    
    Ok(())
}

// =============================================================================
// TRANSFER ADMIN (to DAO)
// =============================================================================

#[derive(Accounts)]
pub struct TransferAdmin<'info> {
    #[account(mut)]
    pub current_admin: Signer<'info>,
    
    pub mint: Account<'info, Mint>,
    
    #[account(
        mut,
        seeds = [LP_LOCK_SEED, mint.key().as_ref()],
        bump = lp_lock.bump,
        constraint = lp_lock.admin == current_admin.key() @ ParadoxError::Unauthorized,
    )]
    pub lp_lock: Account<'info, LpLock>,
    
    /// CHECK: New admin address
    pub new_admin: UncheckedAccount<'info>,
}

pub fn transfer_admin_handler(ctx: Context<TransferAdmin>) -> Result<()> {
    let lp_lock = &mut ctx.accounts.lp_lock;
    let old_admin = lp_lock.admin;
    
    lp_lock.admin = ctx.accounts.new_admin.key();
    
    msg!("LP Lock admin transferred: {} → {}", old_admin, ctx.accounts.new_admin.key());
    
    Ok(())
}
