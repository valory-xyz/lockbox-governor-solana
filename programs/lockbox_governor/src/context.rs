use anchor_lang::prelude::*;
use wormhole_anchor_sdk::wormhole;
use anchor_spl::token::{self, Token, TokenAccount};
use solana_program::{
    system_program,
    sysvar,
    bpf_loader_upgradeable
};

use crate::{
    errors::GovernorError,
    message::{
        TransferMessage,
        TransferAllMessage,
        TransferTokenAccountsMessage,
        SetUpgradeAuthorityMessage,
        UpgradeProgramMessage
    },
    state::{Config, Received},
};

#[derive(Accounts)]
pub struct InitializeLockboxGovernor<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,

    #[account(
        init,
        payer = signer,
        seeds = [Config::SEED_PREFIX],
        bump,
        space = Config::LEN
    )]
    /// Config account, which saves program data useful for other instructions.
    pub config: Box<Account<'info, Config>>,

    #[account(address = system_program::ID)]
    pub system_program: Program<'info, System>,
    #[account(address = sysvar::rent::ID)]
    pub rent: Sysvar<'info, Rent>
}

#[derive(Accounts)]
#[instruction(vaa_hash: [u8; 32])]
pub struct TransferLockboxGovernor<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,

    #[account(
        seeds = [Config::SEED_PREFIX],
        bump,
        constraint = config.verify(posted.emitter_address()) @ GovernorError::InvalidForeignEmitter,
        constraint = posted.emitter_chain() == config.chain @ GovernorError::InvalidForeignChain
    )]
    /// Config account. Wormhole PDAs specified in the config are checked
    /// against the Wormhole accounts in this context. Read-only.
    pub config: Box<Account<'info, Config>>,

    // Wormhole program.
    /// CHECK: Wormhole program account. Required for posted account check. Read-only.
    pub wormhole_program: UncheckedAccount<'info>,

    #[account(
        seeds = [
            wormhole::SEED_PREFIX_POSTED_VAA,
            &vaa_hash
        ],
        bump,
        seeds::program = wormhole_program
    )]
    /// Verified Wormhole message account. The Wormhole program verified
    /// signatures and posted the account data here. Read-only.
    pub posted: Box<Account<'info, wormhole::PostedVaa<TransferMessage>>>,

    #[account(
        init,
        payer = signer,
        seeds = [
            Received::SEED_PREFIX,
            &posted.emitter_chain().to_le_bytes()[..],
            &posted.sequence().to_le_bytes()[..]
        ],
        bump,
        space = Received::LEN
    )]
    /// Received account. [`receive_message`](crate::receive_message) will
    /// deserialize the Wormhole message's payload and save it to this account.
    /// This account cannot be overwritten, and will prevent Wormhole message
    /// replay with the same sequence.
    pub received: Box<Account<'info, Received>>,

    #[account(mut,
        constraint = source_account.owner == config.key() @ GovernorError::WrongAccountOwner
    )]
    // Source account must be owned by the Config account.
    pub source_account: Box<Account<'info, TokenAccount>>,

    #[account(mut,
        constraint = source_account.mint == destination_account.mint @ GovernorError::WrongTokenMint
    )]
    // Source and destination token mint addresses must match.
    pub destination_account: Box<Account<'info, TokenAccount>>,

    #[account(address = token::ID)]
    pub token_program: Program<'info, Token>,

    /// System program
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
#[instruction(vaa_hash: [u8; 32])]
pub struct TransferAllLockboxGovernor<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,

    #[account(
        seeds = [Config::SEED_PREFIX],
        bump,
        constraint = config.verify(posted.emitter_address()) @ GovernorError::InvalidForeignEmitter,
        constraint = posted.emitter_chain() == config.chain @ GovernorError::InvalidForeignChain
    )]
    /// Config account. Wormhole PDAs specified in the config are checked
    /// against the Wormhole accounts in this context. Read-only.
    pub config: Box<Account<'info, Config>>,

    // Wormhole program.
    /// CHECK: Wormhole program account. Required for posted account check. Read-only.
    pub wormhole_program: UncheckedAccount<'info>,

    #[account(
        seeds = [
            wormhole::SEED_PREFIX_POSTED_VAA,
            &vaa_hash
        ],
        bump,
        seeds::program = wormhole_program
    )]
    /// Verified Wormhole message account. The Wormhole program verified
    /// signatures and posted the account data here. Read-only.
    pub posted: Box<Account<'info, wormhole::PostedVaa<TransferAllMessage>>>,

    #[account(
        init,
        payer = signer,
        seeds = [
            Received::SEED_PREFIX,
            &posted.emitter_chain().to_le_bytes()[..],
            &posted.sequence().to_le_bytes()[..]
        ],
        bump,
        space = Received::LEN
    )]
    /// Received account. [`receive_message`](crate::receive_message) will
    /// deserialize the Wormhole message's payload and save it to this account.
    /// This account cannot be overwritten, and will prevent Wormhole message
    /// replay with the same sequence.
    pub received: Box<Account<'info, Received>>,

    #[account(mut,
        constraint = source_account_sol.owner == config.key() @ GovernorError::WrongAccountOwner
    )]
    // Source SOL account must be owned by the Config account.
    pub source_account_sol: Box<Account<'info, TokenAccount>>,

    #[account(mut,
        constraint = source_account_olas.owner == config.key() @ GovernorError::WrongAccountOwner,
        constraint = source_account_sol.mint != source_account_olas.mint @ GovernorError::WrongTokenMint
    )]
    // Source OLAS account must be owned by the Config account.
    // Source SOL and OLAS accounts must have different mints.
    pub source_account_olas: Box<Account<'info, TokenAccount>>,

    #[account(mut,
        constraint = source_account_sol.mint == destination_account_sol.mint @ GovernorError::WrongTokenMint
    )]
    // Source and destination SOL accounts must have the same token mint.
    pub destination_account_sol: Box<Account<'info, TokenAccount>>,

    #[account(mut,
        constraint = source_account_olas.mint == destination_account_olas.mint @ GovernorError::WrongTokenMint
    )]
    // Source and destination OLAS accounts must have the same token mint.
    pub destination_account_olas: Box<Account<'info, TokenAccount>>,

    #[account(address = token::ID)]
    pub token_program: Program<'info, Token>,

    /// System program
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
#[instruction(vaa_hash: [u8; 32])]
pub struct TransferTokenAccountsLockboxGovernor<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,

    #[account(
        seeds = [Config::SEED_PREFIX],
        bump,
        constraint = config.verify(posted.emitter_address()) @ GovernorError::InvalidForeignEmitter,
        constraint = posted.emitter_chain() == config.chain @ GovernorError::InvalidForeignChain
    )]
    /// Config account. Wormhole PDAs specified in the config are checked
    /// against the Wormhole accounts in this context. Read-only.
    pub config: Box<Account<'info, Config>>,

    // Wormhole program.
    /// CHECK: Wormhole program account. Required for posted account check. Read-only.
    pub wormhole_program: UncheckedAccount<'info>,

    #[account(
        seeds = [
            wormhole::SEED_PREFIX_POSTED_VAA,
            &vaa_hash
        ],
        bump,
        seeds::program = wormhole_program
    )]
    /// Verified Wormhole message account. The Wormhole program verified
    /// signatures and posted the account data here. Read-only.
    pub posted: Box<Account<'info, wormhole::PostedVaa<TransferTokenAccountsMessage>>>,

    #[account(
        init,
        payer = signer,
        seeds = [
            Received::SEED_PREFIX,
            &posted.emitter_chain().to_le_bytes()[..],
            &posted.sequence().to_le_bytes()[..]
        ],
        bump,
        space = Received::LEN
    )]
    /// Received account. [`receive_message`](crate::receive_message) will
    /// deserialize the Wormhole message's payload and save it to this account.
    /// This account cannot be overwritten, and will prevent Wormhole message
    /// replay with the same sequence.
    pub received: Box<Account<'info, Received>>,

    #[account(mut,
        constraint = source_account_sol.owner == config.key() @ GovernorError::WrongAccountOwner
    )]
    // Source SOL account must be owned by the Config account.
    pub source_account_sol: Box<Account<'info, TokenAccount>>,

    #[account(mut,
        constraint = source_account_olas.owner == config.key() @ GovernorError::WrongAccountOwner
    )]
    // Source OLAS account must be owned by the Config account.
    pub source_account_olas: Box<Account<'info, TokenAccount>>,

    /// CHECK: This is any account provided by the governor.
    #[account(mut)]
    pub destination_account: UncheckedAccount<'info>,

    #[account(address = token::ID)]
    pub token_program: Program<'info, Token>,
    
    /// System program
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
#[instruction(vaa_hash: [u8; 32])]
pub struct SetUpgradeAuthorityLockboxGovernor<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,

    #[account(
        seeds = [Config::SEED_PREFIX],
        bump,
        constraint = config.verify(posted.emitter_address()) @ GovernorError::InvalidForeignEmitter,
        constraint = posted.emitter_chain() == config.chain @ GovernorError::InvalidForeignChain
    )]
    /// Config account. Wormhole PDAs specified in the config are checked
    /// against the Wormhole accounts in this context. Read-only.
    pub config: Box<Account<'info, Config>>,

    // Wormhole program.
    /// CHECK: Wormhole program account. Required for posted account check. Read-only.
    pub wormhole_program: UncheckedAccount<'info>,

    #[account(
        seeds = [
            wormhole::SEED_PREFIX_POSTED_VAA,
            &vaa_hash
        ],
        bump,
        seeds::program = wormhole_program
    )]
    /// Verified Wormhole message account. The Wormhole program verified
    /// signatures and posted the account data here. Read-only.
    pub posted: Box<Account<'info, wormhole::PostedVaa<SetUpgradeAuthorityMessage>>>,

    #[account(
        init,
        payer = signer,
        seeds = [
            Received::SEED_PREFIX,
            &posted.emitter_chain().to_le_bytes()[..],
            &posted.sequence().to_le_bytes()[..]
        ],
        bump,
        space = Received::LEN
    )]
    /// Received account. [`receive_message`](crate::receive_message) will
    /// deserialize the Wormhole message's payload and save it to this account.
    /// This account cannot be overwritten, and will prevent Wormhole message
    /// replay with the same sequence.
    pub received: Box<Account<'info, Received>>,

    /// CHECK: Any program where the Config account is the upgrade authority.
    #[account(mut,
        constraint = program_account.executable == true,
    )]
    pub program_account: UncheckedAccount<'info>,

    #[account(mut,
        constraint = program_data_account.upgrade_authority_address == Some(config.key()) @ GovernorError::WrongUpgradeAuthority
    )]
    // Program data upgrade authority must be the Config account.
    pub program_data_account: Box<Account<'info, ProgramData>>,

    /// CHECK: This is any account provided by the governor.
    #[account(mut)]
    pub upgrade_authority_account: UncheckedAccount<'info>,

    /// CHECK: BPF Loader Upgraedable account.
    #[account(address = bpf_loader_upgradeable::ID)]
    pub bpf_loader_upgradeable: UncheckedAccount<'info>,

    /// System program
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(vaa_hash: [u8; 32])]
pub struct UpgradeProgramLockboxGovernor<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,

    #[account(
        seeds = [Config::SEED_PREFIX],
        bump,
        constraint = config.verify(posted.emitter_address()) @ GovernorError::InvalidForeignEmitter,
        constraint = posted.emitter_chain() == config.chain @ GovernorError::InvalidForeignChain
    )]
    /// Config account. Wormhole PDAs specified in the config are checked
    /// against the Wormhole accounts in this context. Read-only.
    pub config: Box<Account<'info, Config>>,

    // Wormhole program.
    /// CHECK: Wormhole program account. Required for posted account check. Read-only.
    pub wormhole_program: UncheckedAccount<'info>,

    #[account(
        seeds = [
            wormhole::SEED_PREFIX_POSTED_VAA,
            &vaa_hash
        ],
        bump,
        seeds::program = wormhole_program
    )]
    /// Verified Wormhole message account. The Wormhole program verified
    /// signatures and posted the account data here. Read-only.
    pub posted: Box<Account<'info, wormhole::PostedVaa<UpgradeProgramMessage>>>,

    #[account(
        init,
        payer = signer,
        seeds = [
            Received::SEED_PREFIX,
            &posted.emitter_chain().to_le_bytes()[..],
            &posted.sequence().to_le_bytes()[..]
        ],
        bump,
        space = Received::LEN
    )]
    /// Received account. [`receive_message`](crate::receive_message) will
    /// deserialize the Wormhole message's payload and save it to this account.
    /// This account cannot be overwritten, and will prevent Wormhole message
    /// replay with the same sequence.
    pub received: Box<Account<'info, Received>>,

    /// CHECK: Any program where the Config account is the upgrade authority.
    #[account(mut,
        constraint = program_account.executable == true,
    )]
    pub program_account: UncheckedAccount<'info>,

    #[account(mut,
        constraint = program_data_account.upgrade_authority_address == Some(config.key()) @ GovernorError::WrongUpgradeAuthority
    )]
    // Program data upgrade authority must be the Config account.
    pub program_data_account: Box<Account<'info, ProgramData>>,

    /// CHECK: Buffer account to deploy a new program from. The Config account must be the upgrade authority.
    #[account(mut)]
    pub buffer_account: UncheckedAccount<'info>,

    /// CHECK: This can be any account specified by the governor.
    #[account(mut)]
    pub spill_account: UncheckedAccount<'info>,

    /// CHECK: BPF Loader Upgradeable account.
    #[account(address = bpf_loader_upgradeable::ID)]
    pub bpf_loader_upgradeable: UncheckedAccount<'info>,

    #[account(address = sysvar::rent::ID)]
    pub rent: Sysvar<'info, Rent>,
    #[account(address = sysvar::clock::ID)]
    pub clock: Sysvar<'info, Clock>,

    /// System program
    pub system_program: Program<'info, System>,
}
