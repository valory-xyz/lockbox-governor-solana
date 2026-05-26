use anchor_lang::prelude::*;
use anchor_spl::token::{self, Transfer};
use solana_program::{
    pubkey::Pubkey,
    program::invoke_signed,
    bpf_loader_upgradeable::set_upgrade_authority,
    bpf_loader_upgradeable::upgrade
};
use spl_token::instruction::{set_authority, AuthorityType};

pub use context::*;
pub use errors::*;
pub use events::*;
pub use message::*;
pub use state::*;

pub mod context;
pub mod errors;
pub mod events;
pub mod message;
pub mod state;

declare_id!("DWDGo2UkBUFZ3VitBfWRBMvRnHr7E2DSh57NK27xMYaB");

#[program]
pub mod lockbox_governor {
    use super::*;
    use solana_program::pubkey;
    use wormhole_anchor_sdk::wormhole;

    // SOL address
    const SOL: Pubkey = pubkey!("So11111111111111111111111111111111111111112");
    // OLAS address
    const OLAS: Pubkey = pubkey!("Ez3nzG9ofodYCvEmw73XhQ87LWNYVRM2s7diB5tBZPyM");

    /// Initializes a Lockbox Governor account, aka Config, that stores state data.
    ///
    /// # Arguments
    ///
    /// * `chain` - Emitter chain ID.
    /// * `timelock` - Emitter Timelock address.
    pub fn initialize(
    ctx: Context<InitializeLockboxGovernor>,
      chain: u16,
      timelock: [u8; 32],
    ) -> Result<()> {
        // Foreign emitter cannot share the same Wormhole Chain ID as the
        // Solana Wormhole program's. And cannot register a zero address.
        require!(
            chain > 0 && chain != wormhole::CHAIN_ID_SOLANA && !timelock.iter().all(|&x| x == 0),
            GovernorError::InvalidForeignEmitter,
        );

        // Get the config account
        let config = &mut ctx.accounts.config;

        // Anchor IDL default coder cannot handle wormhole::Finality enum,
        // so this value is stored as u8.
        config.finality = wormhole::Finality::Confirmed as u8;

        // Assign initialization parameters
        config.bump = [ctx.bumps.config];
        config.chain = chain;
        config.foreign_emitter = timelock;

        // Set zero initial values
        config.total_sol_transferred = 0;
        config.total_olas_transferred = 0;

        Ok(())
    }

    /// Transfers specified token amount to a governor specified ATA.
    /// This instruction reads a posted verified Wormhole message and verifies
    /// that the payload is of type TransferMessage (PAYLOAD_TRANSFER).
    /// The data is stored in a [Received] account.
    ///
    /// See TransferMessage struct for deserialization implementation.
    ///
    /// # Arguments
    ///
    /// * `vaa_hash` - Keccak256 hash of verified Wormhole message.
    pub fn transfer(
        ctx: Context<TransferLockboxGovernor>,
        vaa_hash: [u8; 32]
    ) -> Result<()> {
        let posted_message = &ctx.accounts.posted;

        let TransferMessage { source, destination, amount } = posted_message.data();
        let source_account = Pubkey::try_from(*source).unwrap();
        let destination_account = Pubkey::try_from(*destination).unwrap();

        // Check source account
        if source_account != ctx.accounts.source_account.key() {
            return Err(GovernorError::WrongAccount.into());
        }
        // Check destination account
        if destination_account != ctx.accounts.destination_account.key() {
            return Err(GovernorError::WrongAccount.into());
        }

        msg!("Source {:?}", source_account);
        msg!("Destination {:?}", destination_account);
        msg!("Amount {:?}", *amount);

        // Check that the token mint is SOL or OLAS
        if ctx.accounts.source_account.mint == SOL && ctx.accounts.destination_account.mint == SOL {
            ctx.accounts.config.total_sol_transferred += *amount;
        } else if ctx.accounts.source_account.mint == OLAS && ctx.accounts.destination_account.mint == OLAS {
            ctx.accounts.config.total_olas_transferred += *amount;
        } else {
            return Err(GovernorError::WrongTokenMint.into());
        }

        // Check the account balance
        if *amount > ctx.accounts.source_account.amount {
            return Err(GovernorError::Overflow.into());
        }

        // TODO: verifications

        // Transfer the amount of SOL
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.source_account.to_account_info(),
                    to: ctx.accounts.destination_account.to_account_info(),
                    authority: ctx.accounts.config.to_account_info(),
                },
                &[&ctx.accounts.config.seeds()],
            ),
            *amount
        )?;

        // Save batch ID, keccak256 hash and governor message sequence.
        ctx.accounts.received.batch_id = posted_message.batch_id();
        ctx.accounts.received.wormhole_message_hash = vaa_hash;
        ctx.accounts.received.sequence = posted_message.sequence();

        emit!(TransferEvent {
            signer: ctx.accounts.signer.key(),
            token: ctx.accounts.source_account.mint,
            destination: destination_account,
            amount: *amount
        });

        Ok(())
    }

    /// Transfers all token balances to governor specified ATAs.
    /// This instruction reads a posted verified Wormhole message and verifies
    /// that the payload is of type TransferAllMessage (PAYLOAD_TRANSFER_ALL).
    /// The data is stored in a [Received] account.
    ///
    /// See TransferAllMessage struct for deserialization implementation.
    ///
    /// # Arguments
    ///
    /// * `vaa_hash` - Keccak256 hash of verified Wormhole message.
    pub fn transfer_all(
        ctx: Context<TransferAllLockboxGovernor>,
        vaa_hash: [u8; 32]
    ) -> Result<()> {
        let posted_message = &ctx.accounts.posted;

        let TransferAllMessage { source_sol, source_olas, destination_sol, destination_olas } = posted_message.data();
        let source_account_sol = Pubkey::try_from(*source_sol).unwrap();
        let source_account_olas = Pubkey::try_from(*source_olas).unwrap();
        let destination_account_sol = Pubkey::try_from(*destination_sol).unwrap();
        let destination_account_olas = Pubkey::try_from(*destination_olas).unwrap();

        msg!("Source account SOL {:?}", source_account_sol);
        msg!("Source account OLAS {:?}", source_account_olas);
        msg!("Destination account SOL {:?}", destination_account_sol);
        msg!("Destination account OLAS {:?}", destination_account_olas);

        // Check source accounts
        if source_account_sol != ctx.accounts.source_account_sol.key() {
            return Err(GovernorError::WrongAccount.into());
        }
        if source_account_olas != ctx.accounts.source_account_olas.key() {
            return Err(GovernorError::WrongAccount.into());
        }
        // Check destination accounts
        if destination_account_sol != ctx.accounts.destination_account_sol.key() {
            return Err(GovernorError::WrongAccount.into());
        }
        if destination_account_olas != ctx.accounts.destination_account_olas.key() {
            return Err(GovernorError::WrongAccount.into());
        }

        // Check that the first token mint is SOL
        if ctx.accounts.source_account_sol.mint != SOL || ctx.accounts.destination_account_sol.mint != SOL {
            return Err(GovernorError::WrongTokenMint.into());
        }

        // Check that the second token mint is OLAS
        if ctx.accounts.source_account_olas.mint != OLAS || ctx.accounts.destination_account_olas.mint != OLAS {
            return Err(GovernorError::WrongTokenMint.into());
        }

        // Get all amounts
        let amount_sol = ctx.accounts.source_account_sol.amount;
        let amount_olas = ctx.accounts.source_account_olas.amount;
        ctx.accounts.config.total_sol_transferred += amount_sol;
        ctx.accounts.config.total_olas_transferred += amount_olas;

        // Transfer SOL balance
        if amount_sol > 0 {
            token::transfer(
                CpiContext::new_with_signer(
                    ctx.accounts.token_program.to_account_info(),
                    Transfer {
                        from: ctx.accounts.source_account_sol.to_account_info(),
                        to: ctx.accounts.destination_account_sol.to_account_info(),
                        authority: ctx.accounts.config.to_account_info(),
                    },
                    &[&ctx.accounts.config.seeds()],
                ),
                amount_sol,
            )?;
        }

        // Transfer OLAS balance
        if amount_olas > 0 {
            token::transfer(
                CpiContext::new_with_signer(
                    ctx.accounts.token_program.to_account_info(),
                    Transfer {
                        from: ctx.accounts.source_account_olas.to_account_info(),
                        to: ctx.accounts.destination_account_olas.to_account_info(),
                        authority: ctx.accounts.config.to_account_info(),
                    },
                    &[&ctx.accounts.config.seeds()],
                ),
                amount_olas,
            )?;
        }

        // Save batch ID, keccak256 hash and governor message sequence.
        ctx.accounts.received.batch_id = posted_message.batch_id();
        ctx.accounts.received.wormhole_message_hash = vaa_hash;
        ctx.accounts.received.sequence = posted_message.sequence();

        emit!(TransferAllEvent {
            signer: ctx.accounts.signer.key(),
            destination_account_sol,
            destination_account_olas,
            amount_sol,
            amount_olas
        });

        Ok(())
    }

    /// Transfers SOL and OLAS ATAs to a governor specified authority.
    /// This instruction reads a posted verified Wormhole message and verifies
    /// that the payload is of type TransferTokenAccountsMessage (PAYLOAD_TRANSFER_TOKEN_ACCOUNTS).
    /// The data is stored in a [Received] account.
    ///
    /// See TransferTokenAccountsMessage struct for deserialization implementation.
    ///
    /// # Arguments
    ///
    /// * `vaa_hash` - Keccak256 hash of verified Wormhole message.
    pub fn transfer_token_accounts(
        ctx: Context<TransferTokenAccountsLockboxGovernor>,
        vaa_hash: [u8; 32]
    ) -> Result<()> {
        let posted_message = &ctx.accounts.posted;

        let TransferTokenAccountsMessage { source_sol, source_olas, destination } = posted_message.data();
        let source_account_sol = Pubkey::try_from(*source_sol).unwrap();
        let source_account_olas = Pubkey::try_from(*source_olas).unwrap();
        let destination_account = Pubkey::try_from(*destination).unwrap();

        msg!("Source account SOL {:?}", source_account_sol);
        msg!("Source account OLAS {:?}", source_account_olas);
        msg!("Destination account {:?}", destination_account);

        // Check source accounts
        if source_account_sol != ctx.accounts.source_account_sol.key() {
            return Err(GovernorError::WrongAccount.into());
        }
        if source_account_olas != ctx.accounts.source_account_olas.key() {
            return Err(GovernorError::WrongAccount.into());
        }
        // Check destination account
        if destination_account != ctx.accounts.destination_account.key() {
            return Err(GovernorError::WrongAccount.into());
        }

        // Check that the first token mint is SOL
        if ctx.accounts.source_account_sol.mint != SOL {
            return Err(GovernorError::WrongTokenMint.into());
        }

        // Check that the second token mint is OLAS
        if ctx.accounts.source_account_olas.mint != OLAS {
            return Err(GovernorError::WrongTokenMint.into());
        }

        // Transfer SOL token associated account
        invoke_signed(
            &set_authority(
                ctx.accounts.token_program.key,
                ctx.accounts.source_account_sol.to_account_info().key,
                Some(ctx.accounts.destination_account.to_account_info().key),
                AuthorityType::AccountOwner,
                ctx.accounts.config.to_account_info().key,
                &[],
            )?,
            &[
                ctx.accounts.source_account_sol.to_account_info(),
                ctx.accounts.config.to_account_info(),
                ctx.accounts.token_program.to_account_info(),
            ],
            &[&ctx.accounts.config.seeds()],
        )?;

        // Transfer OLAS token associated account
        invoke_signed(
            &set_authority(
                ctx.accounts.token_program.key,
                ctx.accounts.source_account_olas.to_account_info().key,
                Some(ctx.accounts.destination_account.to_account_info().key),
                AuthorityType::AccountOwner,
                ctx.accounts.config.to_account_info().key,
                &[],
            )?,
            &[
                ctx.accounts.source_account_olas.to_account_info(),
                ctx.accounts.config.to_account_info(),
                ctx.accounts.token_program.to_account_info(),
            ],
            &[&ctx.accounts.config.seeds()],
        )?;

        // Save batch ID, keccak256 hash and governor message sequence.
        ctx.accounts.received.batch_id = posted_message.batch_id();
        ctx.accounts.received.wormhole_message_hash = vaa_hash;
        ctx.accounts.received.sequence = posted_message.sequence();

        emit!(TransferTokenAccountsEvent {
            signer: ctx.accounts.signer.key(),
            source_account_sol,
            source_account_olas,
            destination_account
        });

        Ok(())
    }

    /// Sets program upgrade authority provided by the governor.
    /// This instruction reads a posted verified Wormhole message and verifies
    /// that the payload is of type SetUpgradeAuthorityMessage (PAYLOAD_SET_UPGRADE_AUTHORITY).
    /// The data is stored in a [Received] account.
    ///
    /// See SetUpgradeAuthorityMessage struct for deserialization implementation.
    ///
    /// # Arguments
    ///
    /// * `vaa_hash` - Keccak256 hash of verified Wormhole message.
    pub fn set_program_upgrade_authority(
        ctx: Context<SetUpgradeAuthorityLockboxGovernor>,
        vaa_hash: [u8; 32]
    ) -> Result<()> {
        let posted_message = &ctx.accounts.posted;

        let SetUpgradeAuthorityMessage { program_id_bytes, upgrade_authority } = posted_message.data();
        let program_account = Pubkey::try_from(*program_id_bytes).unwrap();
        let upgrade_authority_account = Pubkey::try_from(*upgrade_authority).unwrap();

        msg!("Program account {:?}", program_account);
        msg!("Upgrade authority {:?}", upgrade_authority_account);

        // Check program account that changes the authority
        if program_account != ctx.accounts.program_account.key() {
            return Err(GovernorError::WrongAccount.into());
        }
        // Check authority destination account
        if upgrade_authority_account != ctx.accounts.upgrade_authority_account.key() {
            return Err(GovernorError::WrongAccount.into());
        }

        // Change upgrade authority
        invoke_signed(
            &set_upgrade_authority(
                ctx.accounts.program_account.to_account_info().key,
                ctx.accounts.config.to_account_info().key,
                Some(ctx.accounts.upgrade_authority_account.to_account_info().key)
            ),
            &[
                ctx.accounts.program_data_account.to_account_info(),
                ctx.accounts.config.to_account_info(),
                ctx.accounts.upgrade_authority_account.to_account_info()
            ],
            &[&ctx.accounts.config.seeds()]
        )?;

        // Save batch ID, keccak256 hash and governor message sequence.
        ctx.accounts.received.batch_id = posted_message.batch_id();
        ctx.accounts.received.wormhole_message_hash = vaa_hash;
        ctx.accounts.received.sequence = posted_message.sequence();

        emit!(SetUpgradeAuthorityEvent {
            signer: ctx.accounts.signer.key(),
            program_account,
            upgrade_authority_account
        });

        Ok(())
    }

    /// Upgrades the program via the governor dictated buffer account.
    /// This instruction reads a posted verified Wormhole message and verifies
    /// that the payload is of type UpgradeProgramMessage (PAYLOAD_UPGRADE_PROGRAM).
    /// The data is stored in a [Received] account.
    ///
    /// See UpgradeProgramMessage struct for deserialization implementation.
    ///
    /// # Arguments
    ///
    /// * `vaa_hash` - Keccak256 hash of verified Wormhole message.
    pub fn upgrade_program(
        ctx: Context<UpgradeProgramLockboxGovernor>,
        vaa_hash: [u8; 32]
    ) -> Result<()> {
        let posted_message = &ctx.accounts.posted;

        let UpgradeProgramMessage { program_id_bytes, buffer_account_bytes, spill_account_bytes } =
            posted_message.data();
        let program_account = Pubkey::try_from(*program_id_bytes).unwrap();
        let buffer_account = Pubkey::try_from(*buffer_account_bytes).unwrap();
        let spill_account = Pubkey::try_from(*spill_account_bytes).unwrap();

        // Check program account that changes the authority
        if program_account != ctx.accounts.program_account.key() {
            return Err(GovernorError::WrongAccount.into());
        }
        // Check buffer account
        if buffer_account != ctx.accounts.buffer_account.key() {
            return Err(GovernorError::WrongAccount.into());
        }
        // Check spill account
        if spill_account != ctx.accounts.spill_account.key() {
            return Err(GovernorError::WrongAccount.into());
        }

        msg!("Program account {:?}", program_account);
        msg!("Buffer account {:?}", buffer_account);
        msg!("Spill account {:?}", spill_account);

        // Transfer the token associated account
        invoke_signed(
            &upgrade(
                ctx.accounts.program_account.to_account_info().key,
                ctx.accounts.buffer_account.to_account_info().key,
                ctx.accounts.config.to_account_info().key,
                ctx.accounts.spill_account.to_account_info().key
            ),
            &[
                ctx.accounts.program_data_account.to_account_info(),
                ctx.accounts.program_account.to_account_info(),
                ctx.accounts.buffer_account.to_account_info(),
                ctx.accounts.spill_account.to_account_info(),
                ctx.accounts.rent.to_account_info(),
                ctx.accounts.clock.to_account_info(),
                ctx.accounts.config.to_account_info()
            ],
            &[&ctx.accounts.config.seeds()]
        )?;

        // Save batch ID, keccak256 hash and governor message sequence.
        ctx.accounts.received.batch_id = posted_message.batch_id();
        ctx.accounts.received.wormhole_message_hash = vaa_hash;
        ctx.accounts.received.sequence = posted_message.sequence();

        emit!(UpgradeProgramEvent {
            signer: ctx.accounts.signer.key(),
            program_account,
            buffer_account,
            spill_account
        });

        Ok(())
    }
}
