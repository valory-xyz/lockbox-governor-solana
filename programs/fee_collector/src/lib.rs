use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use solana_program::{
  pubkey::Pubkey,
  program::invoke_signed,
  bpf_loader_upgradeable::set_upgrade_authority,
  bpf_loader_upgradeable::upgrade,
  system_program,
  sysvar
};
use spl_token::instruction::{set_authority, AuthorityType};

pub use context::*;
pub use error::*;
pub use message::*;
pub use state::*;

pub mod context;
pub mod error;
pub mod message;
pub mod state;

declare_id!("DWDGo2UkBUFZ3VitBfWRBMvRnHr7E2DSh57NK27xMYaB");

#[program]
pub mod lockbox_governor {
  use super::*;
  use solana_program::pubkey;

  // SOL address
  const SOL: Pubkey = pubkey!("So11111111111111111111111111111111111111112");
  // OLAS address
  const OLAS: Pubkey = pubkey!("Ez3nzG9ofodYCvEmw73XhQ87LWNYVRM2s7diB5tBZPyM");

  /// Initializes a Lockbox account that stores state data.
  pub fn initialize(
    ctx: Context<InitializeLockboxGovernor>,
      chain: u16,
      timelock: [u8; 32],
  ) -> Result<()> {
  // Foreign emitter cannot share the same Wormhole Chain ID as the
  // Solana Wormhole program's. And cannot register a zero address.
  require!(
      chain > 0 && chain != wormhole::CHAIN_ID_SOLANA && !address.iter().all(|&x| x == 0),
      ErrorCode::InvalidForeignEmitter,
  );

    // Get the fee governor account
    let governor = &mut ctx.accounts.governor;

    // TODO Owner is not needed, hardcode it as a Timelock during the initialization
    // Set the owner of the config (effectively the owner of the program).
    config.owner = ctx.accounts.owner.key();

    // config.wormhole is not needed (for sending only)

    // Set default values for posting Wormhole messages.
    //
    // Zero means no batching.
    config.batch_id = 0;

    // Anchor IDL default coder cannot handle wormhole::Finality enum,
    // so this value is stored as u8.
    config.finality = wormhole::Finality::Confirmed as u8;

    // TODO Make this in a better way as a constant withing the state
    // Get the anchor-derived bump
    let bump = *ctx.bumps.get("governor").ok_or(ErrorCode::BumpNotFound)?;

    // TODO Chain and address are for foreign_emitter - set it during the initialization
    // Initialize Lockbox manager account
    governor.initialize(
      chain,
      timelock,
      bump
    )?;

    Ok(())
  }

  /// Transfer token funds.
  pub fn transfer(
    ctx: Context<TransferLockboxGovernor>,
    amount: u64
  ) -> Result<()> {
    // Check that the token mint is SOL or OLAS
    if ctx.accounts.collector_account.mint == SOL && ctx.accounts.destination_account.mint == SOL {
      ctx.accounts.governor.total_sol_transferred += amount;
    } else if ctx.accounts.collector_account.mint == OLAS && ctx.accounts.destination_account.mint == OLAS {
      ctx.accounts.governor.total_olas_transferred += amount;
    } else {
      return Err(ErrorCode::WrongTokenMint.into());
    }

    // Transfer the amount of SOL
    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.collector_account.to_account_info(),
                to: ctx.accounts.destination_account.to_account_info(),
                authority: ctx.accounts.governor.to_account_info(),
            },
            &[&ctx.accounts.governor.seeds()],
        ),
        amount
    )?;

    Ok(())
  }

  /// Transfer token funds.
  pub fn transfer_all(
    ctx: Context<TransferAllLockboxGovernor>
  ) -> Result<()> {
    // Check that the first token mint is SOL
    if ctx.accounts.collector_account_sol.mint != SOL || ctx.accounts.destination_account_sol.mint != SOL {
      return Err(ErrorCode::WrongTokenMint.into());
    }

    // Check that the second token mint is OLAS
    if ctx.accounts.collector_account_olas.mint != OLAS || ctx.accounts.destination_account_olas.mint != OLAS {
      return Err(ErrorCode::WrongTokenMint.into());
    }

    // Get all amounts
    let amount_sol = ctx.accounts.collector_account_sol.amount;
    let amount_olas = ctx.accounts.collector_account_olas.amount;

    // TODO optimize with creating context and calling transfer one by one
    // Transfer the amount of SOL
    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.collector_account_sol.to_account_info(),
                to: ctx.accounts.destination_account_sol.to_account_info(),
                authority: ctx.accounts.governor.to_account_info(),
            },
            &[&ctx.accounts.governor.seeds()],
        ),
        amount_sol,
    )?;

    // Transfer the amount of OLAS
    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.collector_account_olas.to_account_info(),
                to: ctx.accounts.destination_account_olas.to_account_info(),
                authority: ctx.accounts.governor.to_account_info(),
            },
            &[&ctx.accounts.governor.seeds()],
        ),
        amount_olas,
    )?;

    Ok(())
  }

  /// Transfer token account.
  pub fn transfer_token_accounts(
    ctx: Context<TransferTokenAccountsLockboxGovernor>
  ) -> Result<()> {
    // Check that the first token mint is SOL
    if ctx.accounts.collector_account_sol.mint != SOL {
      return Err(ErrorCode::WrongTokenMint.into());
    }

    // Check that the second token mint is OLAS
    if ctx.accounts.collector_account_olas.mint != OLAS {
      return Err(ErrorCode::WrongTokenMint.into());
    }

    // Transfer SOL token associated account
    invoke_signed(
        &set_authority(
            ctx.accounts.token_program.key,
            ctx.accounts.collector_account_sol.to_account_info().key,
            Some(ctx.accounts.destination.to_account_info().key),
            AuthorityType::AccountOwner,
            ctx.accounts.governor.to_account_info().key,
            &[],
        )?,
        &[
            ctx.accounts.collector_account_sol.to_account_info(),
            ctx.accounts.governor.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
        ],
        &[&ctx.accounts.governor.seeds()],
    )?;

    // Transfer OLAS token associated account
    invoke_signed(
        &set_authority(
            ctx.accounts.token_program.key,
            ctx.accounts.collector_account_olas.to_account_info().key,
            Some(ctx.accounts.destination.to_account_info().key),
            AuthorityType::AccountOwner,
            ctx.accounts.governor.to_account_info().key,
            &[],
        )?,
        &[
            ctx.accounts.collector_account_olas.to_account_info(),
            ctx.accounts.governor.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
        ],
        &[&ctx.accounts.governor.seeds()],
    )?;

    Ok(())
  }

  /// Change upgrade authority.
  pub fn change_upgrade_authority(
    ctx: Context<ChangeUpgradeAuthorityLockboxGovernor>
  ) -> Result<()> {
    // Change upgrade authority
    invoke_signed(
        &set_upgrade_authority(
            ctx.accounts.program_to_update_authority.to_account_info().key,
            ctx.accounts.governor.to_account_info().key,
            Some(ctx.accounts.destination.to_account_info().key)
        ),
        &[
            ctx.accounts.program_data_to_update_authority.to_account_info(),
            ctx.accounts.governor.to_account_info(),
            ctx.accounts.destination.to_account_info()
        ],
        &[&ctx.accounts.governor.seeds()]
    )?;

    Ok(())
  }

  /// Upgrade the program.
  pub fn upgrade_program(
    ctx: Context<UpgradeProgramLockboxGovernor>
  ) -> Result<()> {
    // Transfer the token associated account
    invoke_signed(
        &upgrade(
            ctx.accounts.program_address.to_account_info().key,
            ctx.accounts.buffer_address.to_account_info().key,
            ctx.accounts.governor.to_account_info().key,
            ctx.accounts.spill_address.to_account_info().key
        ),
        &[
            ctx.accounts.program_data_address.to_account_info(),
            ctx.accounts.program_address.to_account_info(),
            ctx.accounts.buffer_address.to_account_info(),
            ctx.accounts.spill_address.to_account_info(),
            ctx.accounts.rent.to_account_info(),
            ctx.accounts.clock.to_account_info(),
            ctx.accounts.governor.to_account_info()
        ],
        &[&ctx.accounts.governor.seeds()]
    )?;

    Ok(())
  }

//     /// This instruction reads a posted verified Wormhole message and verifies
//     /// that the payload is of type [HelloWorldMessage::Hello] (payload ID == 1). HelloWorldMessage
//     /// data is stored in a [Received] account.
//     ///
//     /// See [HelloWorldMessage] enum for deserialization implementation.
//     ///
//     /// # Arguments
//     ///
//     /// * `vaa_hash` - Keccak256 hash of verified Wormhole message
//     pub fn receive_message(ctx: Context<ReceiveMessage>, vaa_hash: [u8; 32]) -> Result<()> {
//         let posted_message = &ctx.accounts.posted;
//
//         if let HelloWorldMessage::Hello { message } = posted_message.data() {
//             // HelloWorldMessage cannot be larger than the maximum size of the account.
//             require!(
//                 message.len() <= MESSAGE_MAX_LENGTH,
//                 HelloWorldError::InvalidMessage,
//             );
//
//             // Save batch ID, keccak256 hash and message payload.
//             let received = &mut ctx.accounts.received;
//             received.batch_id = posted_message.batch_id();
//             received.wormhole_message_hash = vaa_hash;
//             received.message = message.clone();
//
//             // Done
//             Ok(())
//         } else {
//             Err(HelloWorldError::InvalidMessage.into())
//         }
//     }
}

#[derive(Accounts)]
pub struct InitializeLockboxGovernor<'info> {
  #[account(mut)]
  pub signer: Signer<'info>,

  #[account(init,
    seeds = [
      b"lockbox_governor".as_ref()
    ],
    bump,
    payer = signer,
    space = LockboxGovernor::LEN
  )]
  pub governor: Box<Account<'info, LockboxGovernor>>,

  #[account(address = system_program::ID)]
  pub system_program: Program<'info, System>,
  #[account(address = sysvar::rent::ID)]
  pub rent: Sysvar<'info, Rent>
}

#[derive(Accounts)]
pub struct TransferLockboxGovernor<'info> {
  #[account(mut)]
  pub signer: Signer<'info>,

  #[account(mut)]
  pub governor: Box<Account<'info, LockboxGovernor>>,

  #[account(mut)]
  pub collector_account: Box<Account<'info, TokenAccount>>,

  #[account(mut)]
  pub destination_account: Box<Account<'info, TokenAccount>>,

  #[account(address = token::ID)]
  pub token_program: Program<'info, Token>
}

#[derive(Accounts)]
pub struct TransferAllLockboxGovernor<'info> {
  #[account(mut)]
  pub signer: Signer<'info>,

  #[account(mut)]
  pub governor: Box<Account<'info, LockboxGovernor>>,

  #[account(mut)]
  pub collector_account_sol: Box<Account<'info, TokenAccount>>,

  #[account(mut)]
  pub collector_account_olas: Box<Account<'info, TokenAccount>>,

  #[account(mut)]
  pub destination_account_sol: Box<Account<'info, TokenAccount>>,

  #[account(mut)]
  pub destination_account_olas: Box<Account<'info, TokenAccount>>,

  #[account(address = token::ID)]
  pub token_program: Program<'info, Token>
}

#[derive(Accounts)]
pub struct TransferTokenAccountsLockboxGovernor<'info> {
  #[account(mut)]
  pub signer: Signer<'info>,

  #[account(mut)]
  pub governor: Box<Account<'info, LockboxGovernor>>,

  #[account(mut)]
  pub collector_account_sol: Box<Account<'info, TokenAccount>>,

  #[account(mut)]
  pub collector_account_olas: Box<Account<'info, TokenAccount>>,

  /// CHECK: Check later
  #[account(mut)]
  pub destination: UncheckedAccount<'info>,

  #[account(address = token::ID)]
  pub token_program: Program<'info, Token>
}

#[derive(Accounts)]
pub struct ChangeUpgradeAuthorityLockboxGovernor<'info> {
  #[account(mut)]
  pub signer: Signer<'info>,

  /// CHECK: Check later
  #[account(mut)]
  pub program_to_update_authority: UncheckedAccount<'info>,

  /// CHECK: Check later
  #[account(mut)]
  pub program_data_to_update_authority: UncheckedAccount<'info>,

  #[account(mut)]
  pub governor: Box<Account<'info, LockboxGovernor>>,

  /// CHECK: Check later
  #[account(mut)]
  pub destination: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct UpgradeProgramLockboxGovernor<'info> {
  #[account(mut)]
  pub signer: Signer<'info>,

  /// CHECK: Check later
  #[account(mut)]
  pub program_address: UncheckedAccount<'info>,

  /// CHECK: Check later
  #[account(mut)]
  pub program_data_address: UncheckedAccount<'info>,

  /// CHECK: Check later
  #[account(mut)]
  pub buffer_address: UncheckedAccount<'info>,

  #[account(mut)]
  pub spill_address: Box<Account<'info, TokenAccount>>,

  #[account(mut)]
  pub governor: Box<Account<'info, LockboxGovernor>>,

  #[account(address = sysvar::rent::ID)]
  pub rent: Sysvar<'info, Rent>,
  #[account(address = sysvar::clock::ID)]
  pub clock: Sysvar<'info, Clock>
}


#[error_code]
pub enum ErrorCode {
  #[msg("Invalid foreign emitter")]
  InvalidForeignEmitter,
  #[msg("Bump not found")]
  BumpNotFound,
  #[msg("Wrong token mint")]
  WrongTokenMint,
}


#[event]
pub struct TransferEvent {
  // Signer (user)
  #[index]
  pub signer: Pubkey,
  // SOL amount transferred
  pub sol_transferred: u64,
  // OLAS amount transferred
  pub olas_transferred: u64
}
