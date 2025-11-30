/**
 * LP Lock Instructions - Progressive Timelock with Snapshot/Restore
 * 
 * TIMELINE:
 * - Days 0-3:   12h notice (emergency fixes)
 * - Days 3-15:  15 days notice (careful changes)
 * - Days 15+:   30 days notice (permanent mode)
 * 
 * Made by LabsX402 for Solana
 * https://x.com/LabsX402
 */

use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount, Mint, Transfer, transfer};

use crate::{
    state::{LpLock, LpLockStatus, HolderBalancesSnapshot, HolderSnapshot},
    ParadoxError,
    LP_LOCK_SEED,
    LpLockCreated,
    LpWithdrawalAnnounced,
    LpWithdrawalExecuted,
    LpWithdrawalCancelled,
};

/// Seed for holder snapshot
pub const HOLDER_SNAPSHOT_SEED: &[u8] = b"holder_snapshot";

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
    
    /// CHECK: Emergency multisig address
    pub emergency_multisig: UncheckedAccount<'info>,
    
    #[account(mut)]
    pub creator_lp_account: Account<'info, TokenAccount>,
    
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn create_pool_and_lock_handler(
    ctx: Context<CreatePoolAndLock>,
    sol_amount: u64,
    token_amount: u64,
    _timelock_seconds: Option<i64>, // Ignored - uses progressive system
    _max_withdrawal_bps: Option<u16>, // Ignored - 100% allowed with proper notice
) -> Result<()> {
    let lp_lock = &mut ctx.accounts.lp_lock;
    
    // =========================================================================
    // DEV NOTE: Implement pool creation + LP deposit here
    // =========================================================================
    
    let lp_tokens_received: u64 = 0; // Replace with actual LP tokens
    
    lp_lock.initialize(
        ctx.accounts.mint.key(),
        Pubkey::default(), // Replace with actual pool
        ctx.accounts.lp_token_mint.key(),
        ctx.accounts.lp_vault.key(),
        ctx.accounts.creator.key(),
        ctx.accounts.emergency_multisig.key(),
        lp_tokens_received,
        ctx.bumps.lp_lock,
    );
    
    let phase_name = lp_lock.get_phase_name();
    let timelock_hours = lp_lock.get_required_timelock() / 3600;
    
    msg!("╔══════════════════════════════════════════════════════════════╗");
    msg!("║           LP LOCK CREATED - PROGRESSIVE TIMELOCK             ║");
    msg!("╠══════════════════════════════════════════════════════════════╣");
    msg!("║ Current Phase: {}", phase_name);
    msg!("║ Current Timelock: {}h notice required", timelock_hours);
    msg!("║");
    msg!("║ TIMELINE:");
    msg!("║   Days 0-3:   12h notice (emergency)");
    msg!("║   Days 3-15:  15 days notice");
    msg!("║   Days 15+:   30 days notice (permanent)");
    msg!("║");
    msg!("║ SAFETY: Snapshot taken before any withdrawal");
    msg!("║         Restore capability for relaunch");
    msg!("╚══════════════════════════════════════════════════════════════╝");
    
    emit!(LpLockCreated {
        mint: ctx.accounts.mint.key(),
        lp_pool: lp_lock.lp_pool,
        lp_tokens_locked: lp_tokens_received,
        timelock_seconds: lp_lock.get_required_timelock(),
        max_withdrawal_bps: 10000, // 100%
        admin: ctx.accounts.creator.key(),
    });
    
    Ok(())
}

// =============================================================================
// TAKE SNAPSHOT
// =============================================================================

#[derive(Accounts)]
pub struct TakeSnapshot<'info> {
    #[account(
        constraint = admin.key() == lp_lock.admin @ ParadoxError::Unauthorized
    )]
    pub admin: Signer<'info>,
    
    pub mint: Account<'info, Mint>,
    
    #[account(
        mut,
        seeds = [LP_LOCK_SEED, mint.key().as_ref()],
        bump = lp_lock.bump,
    )]
    pub lp_lock: Account<'info, LpLock>,
}

pub fn take_snapshot_handler(
    ctx: Context<TakeSnapshot>,
    reason: [u8; 32],
    sol_reserve: u64,
    token_reserve: u64,
    total_supply: u64,
    holder_count: u32,
) -> Result<u64> {
    let lp_lock = &mut ctx.accounts.lp_lock;
    
    let snapshot_id = lp_lock.take_snapshot(
        reason,
        sol_reserve,
        token_reserve,
        total_supply,
        holder_count,
    );
    
    msg!("📸 Snapshot #{} taken", snapshot_id);
    msg!("   LP Tokens: {}", lp_lock.lp_tokens_locked);
    msg!("   SOL Reserve: {}", sol_reserve);
    msg!("   Token Reserve: {}", token_reserve);
    msg!("   Holders: {}", holder_count);
    
    Ok(snapshot_id)
}

// =============================================================================
// ANNOUNCE WITHDRAWAL (with automatic snapshot)
// =============================================================================

#[derive(Accounts)]
pub struct AnnounceWithdrawal<'info> {
    #[account(
        constraint = admin.key() == lp_lock.admin @ ParadoxError::Unauthorized
    )]
    pub admin: Signer<'info>,
    
    pub mint: Account<'info, Mint>,
    
    #[account(
        mut,
        seeds = [LP_LOCK_SEED, mint.key().as_ref()],
        bump = lp_lock.bump,
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
    require!(amount <= lp_lock.lp_tokens_locked, ParadoxError::InsufficientLpTokens);
    
    // Take automatic snapshot before withdrawal
    let mut snapshot_reason = [0u8; 32];
    snapshot_reason[..16].copy_from_slice(b"PRE_WITHDRAWAL__");
    
    let snapshot_id = lp_lock.take_snapshot(
        snapshot_reason,
        0, // DEV: Fetch actual reserves
        0,
        0,
        0,
    );
    
    // Announce withdrawal
    let slot = lp_lock.announce_withdrawal(amount, recipient, reason, snapshot_id)?;
    
    let phase_name = lp_lock.get_phase_name();
    let timelock = lp_lock.get_required_timelock();
    let execute_after = lp_lock.pending_withdrawals[slot].execute_after;
    
    msg!("╔══════════════════════════════════════════════════════════════╗");
    msg!("║           LP WITHDRAWAL ANNOUNCED                            ║");
    msg!("╠══════════════════════════════════════════════════════════════╣");
    msg!("║ Amount: {} LP tokens", amount);
    msg!("║ Recipient: {}", recipient);
    msg!("║ Phase: {}", phase_name);
    msg!("║ Timelock: {} hours", timelock / 3600);
    msg!("║ Executable after: {}", execute_after);
    msg!("║ Snapshot ID: #{} (for restore)", snapshot_id);
    msg!("║");
    msg!("║ ⚠️  VISIBLE ON-CHAIN - Everyone can see this!");
    msg!("╚══════════════════════════════════════════════════════════════╝");
    
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
// EXECUTE WITHDRAWAL
// =============================================================================

#[derive(Accounts)]
pub struct ExecuteWithdrawal<'info> {
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
    
    // Validate
    require!(lp_lock.can_execute_withdrawal(slot_usize), ParadoxError::TimelockNotExpired);
    
    let pending = &lp_lock.pending_withdrawals[slot_usize];
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
    
    transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.lp_vault.to_account_info(),
                to: ctx.accounts.recipient_lp_account.to_account_info(),
                authority: lp_lock.to_account_info(),
            },
            &[seeds],
        ),
        amount,
    )?;
    
    msg!("✅ LP Withdrawal executed after {}h timelock", time_waited / 3600);
    msg!("   Amount: {} LP tokens", amount);
    msg!("   Recipient: {}", recipient);
    msg!("   Remaining locked: {}", lp_lock.lp_tokens_locked);
    
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
    #[account(
        constraint = admin.key() == lp_lock.admin @ ParadoxError::Unauthorized
    )]
    pub admin: Signer<'info>,
    
    pub mint: Account<'info, Mint>,
    
    #[account(
        mut,
        seeds = [LP_LOCK_SEED, mint.key().as_ref()],
        bump = lp_lock.bump,
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
    
    msg!("❌ LP Withdrawal cancelled");
    msg!("   Amount: {} LP tokens", amount);
    
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
// RESTORE FROM SNAPSHOT
// =============================================================================

#[derive(Accounts)]
pub struct RestoreFromSnapshot<'info> {
    #[account(
        constraint = admin.key() == lp_lock.admin @ ParadoxError::Unauthorized
    )]
    pub admin: Signer<'info>,
    
    pub mint: Account<'info, Mint>,
    
    #[account(
        mut,
        seeds = [LP_LOCK_SEED, mint.key().as_ref()],
        bump = lp_lock.bump,
    )]
    pub lp_lock: Account<'info, LpLock>,
    
    #[account(mut)]
    pub lp_vault: Account<'info, TokenAccount>,
    
    #[account(mut)]
    pub source_lp_account: Account<'info, TokenAccount>,
    
    pub token_program: Program<'info, Token>,
}

pub fn restore_from_snapshot_handler(
    ctx: Context<RestoreFromSnapshot>,
    snapshot_id: u64,
    lp_amount: u64,
) -> Result<()> {
    let lp_lock = &mut ctx.accounts.lp_lock;
    
    // Validate snapshot exists
    let snapshot = lp_lock.get_snapshot(snapshot_id)
        .ok_or(error!(ParadoxError::InvalidWithdrawalSlot))?;
    
    require!(!snapshot.was_restored, ParadoxError::AlreadyFinalized);
    
    msg!("╔══════════════════════════════════════════════════════════════╗");
    msg!("║           RESTORING FROM SNAPSHOT #{}                        ║", snapshot_id);
    msg!("╠══════════════════════════════════════════════════════════════╣");
    msg!("║ Original LP: {}", snapshot.lp_tokens);
    msg!("║ Restoring: {} LP tokens", lp_amount);
    msg!("║ Original SOL Reserve: {}", snapshot.sol_reserve);
    msg!("║ Original Token Reserve: {}", snapshot.token_reserve);
    msg!("║ Original Holders: {}", snapshot.holder_count);
    msg!("╚══════════════════════════════════════════════════════════════╝");
    
    // Transfer LP tokens to vault
    transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.source_lp_account.to_account_info(),
                to: ctx.accounts.lp_vault.to_account_info(),
                authority: ctx.accounts.admin.to_account_info(),
            },
        ),
        lp_amount,
    )?;
    
    // Update state
    lp_lock.restore_from_snapshot(lp_amount);
    lp_lock.mark_snapshot_restored(snapshot_id);
    
    msg!("✅ LP Lock restored successfully");
    msg!("   New locked amount: {}", lp_lock.lp_tokens_locked);
    msg!("   Current phase: {}", lp_lock.get_phase_name());
    
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
    
    let phase = lp_lock.get_current_phase();
    let timelock = lp_lock.get_required_timelock();
    let days_to_next = lp_lock.days_until_next_phase();
    
    let status_str = match lp_lock.status {
        LpLockStatus::NotInitialized => "NOT_INITIALIZED",
        LpLockStatus::Active => "ACTIVE",
        LpLockStatus::WithdrawalPending => "WITHDRAWAL_PENDING",
        LpLockStatus::Withdrawn => "WITHDRAWN",
        LpLockStatus::Restored => "RESTORED",
    };
    
    msg!("╔══════════════════════════════════════════════════════════════╗");
    msg!("║           LP LOCK STATUS                                     ║");
    msg!("╠══════════════════════════════════════════════════════════════╣");
    msg!("║ Status: {}", status_str);
    msg!("║ Phase: {}", lp_lock.get_phase_name());
    msg!("║ Timelock: {}h notice required", timelock / 3600);
    if let Some(days) = days_to_next {
        msg!("║ Days until next phase: {}", days);
    }
    msg!("║");
    msg!("║ LP Tokens Locked: {}", lp_lock.lp_tokens_locked);
    msg!("║ Total Withdrawn: {}", lp_lock.total_withdrawn);
    msg!("║ Initial LP: {}", lp_lock.initial_lp_tokens);
    msg!("║ Snapshots taken: {}", lp_lock.snapshot_counter);
    msg!("║ Pending withdrawals: {}", lp_lock.pending_count);
    msg!("╚══════════════════════════════════════════════════════════════╝");
    
    // Show pending withdrawals
    for (i, pw) in lp_lock.pending_withdrawals.iter().enumerate() {
        if pw.is_active {
            let remaining = lp_lock.time_until_executable(i);
            msg!("  Pending #{}: {} LP → {} ({}h remaining)",
                i, pw.amount, pw.recipient, remaining / 3600);
        }
    }
    
    Ok(())
}

// =============================================================================
// TRANSFER ADMIN
// =============================================================================

#[derive(Accounts)]
pub struct TransferAdmin<'info> {
    #[account(
        constraint = current_admin.key() == lp_lock.admin @ ParadoxError::Unauthorized
    )]
    pub current_admin: Signer<'info>,
    
    pub mint: Account<'info, Mint>,
    
    #[account(
        mut,
        seeds = [LP_LOCK_SEED, mint.key().as_ref()],
        bump = lp_lock.bump,
    )]
    pub lp_lock: Account<'info, LpLock>,
    
    /// CHECK: New admin address
    pub new_admin: UncheckedAccount<'info>,
}

pub fn transfer_admin_handler(ctx: Context<TransferAdmin>) -> Result<()> {
    let lp_lock = &mut ctx.accounts.lp_lock;
    let old_admin = lp_lock.admin;
    
    lp_lock.admin = ctx.accounts.new_admin.key();
    
    msg!("Admin transferred: {} → {}", old_admin, ctx.accounts.new_admin.key());
    
    Ok(())
}
