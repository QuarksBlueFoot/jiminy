use anchor_lang::prelude::*;

declare_id!("AnchorVau1t111111111111111111111111111111111");

#[program]
pub mod bench_anchor_vault {
    use super::*;

    pub fn init_vault(ctx: Context<InitVault>) -> Result<()> {
        let vault = &mut ctx.accounts.vault;
        vault.balance = 0;
        vault.authority = ctx.accounts.authority.key();
        Ok(())
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        require!(amount > 0, VaultError::ZeroAmount);

        // Transfer lamports from depositor to vault.
        let depositor = &ctx.accounts.depositor;
        let vault_ai = ctx.accounts.vault.to_account_info();

        **depositor.to_account_info().try_borrow_mut_lamports()? -= amount;
        **vault_ai.try_borrow_mut_lamports()? += amount;

        let vault = &mut ctx.accounts.vault;
        vault.balance = vault.balance.checked_add(amount).ok_or(VaultError::Overflow)?;
        Ok(())
    }

    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        require!(amount > 0, VaultError::ZeroAmount);

        let vault = &mut ctx.accounts.vault;
        require_gte!(vault.balance, amount, VaultError::InsufficientFunds);

        vault.balance = vault.balance.checked_sub(amount).ok_or(VaultError::Overflow)?;

        let vault_ai = vault.to_account_info();
        let recipient = &ctx.accounts.recipient;

        **vault_ai.try_borrow_mut_lamports()? -= amount;
        **recipient.to_account_info().try_borrow_mut_lamports()? += amount;

        Ok(())
    }

    pub fn close_vault(_ctx: Context<CloseVault>) -> Result<()> {
        // Anchor handles the close via the `close` constraint.
        Ok(())
    }
}

// ── Accounts ─────────────────────────────────────────────────────────────────

#[derive(Accounts)]
pub struct InitVault<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        init,
        payer = authority,
        space = 8 + Vault::INIT_SPACE,
    )]
    pub vault: Account<'info, Vault>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub depositor: Signer<'info>,

    #[account(mut)]
    pub vault: Account<'info, Vault>,
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    pub authority: Signer<'info>,

    #[account(
        mut,
        has_one = authority,
    )]
    pub vault: Account<'info, Vault>,

    /// CHECK: Receives lamports.
    #[account(
        mut,
        constraint = vault.key() != recipient.key() @ VaultError::SameAccount,
    )]
    pub recipient: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct CloseVault<'info> {
    pub authority: Signer<'info>,

    #[account(
        mut,
        has_one = authority,
        close = destination,
    )]
    pub vault: Account<'info, Vault>,

    /// CHECK: Receives remaining lamports on close.
    #[account(
        mut,
        constraint = vault.key() != destination.key() @ VaultError::SameAccount,
    )]
    pub destination: AccountInfo<'info>,
}

// ── State ────────────────────────────────────────────────────────────────────

#[account]
#[derive(InitSpace)]
pub struct Vault {
    pub balance: u64,
    pub authority: Pubkey,
}

// ── Errors ───────────────────────────────────────────────────────────────────

#[error_code]
pub enum VaultError {
    #[msg("Amount must be greater than zero")]
    ZeroAmount,
    #[msg("Insufficient vault balance")]
    InsufficientFunds,
    #[msg("Arithmetic overflow")]
    Overflow,
    #[msg("Source and destination cannot be the same")]
    SameAccount,
}
