use anchor_lang::prelude::*;

#[event]
pub struct TransferEvent {
    // Signer (user)
    #[index]
    pub signer: Pubkey,
    // Token mint
    #[index]
    pub token: Pubkey,
    // Destination account address
    #[index]
    pub destination: Pubkey,
    // SOL / OLAS amount transferred
    pub amount: u64
}

#[event]
pub struct TransferAllEvent {
    // Signer (user)
    #[index]
    pub signer: Pubkey,
    // SOL destination account address
    #[index]
    pub destination_account_sol: Pubkey,
    // OLAS destination account address
    #[index]
    pub destination_account_olas: Pubkey,
    // SOL amount transferred
    pub amount_sol: u64,
    // OLAS amount transferred
    pub amount_olas: u64
}

#[event]
pub struct TransferTokenAccountsEvent {
    // Signer (user)
    #[index]
    pub signer: Pubkey,
    // SOL source account address
    #[index]
    pub source_account_sol: Pubkey,
    // OLAS source account address
    #[index]
    pub source_account_olas: Pubkey,
    // New owner destination account address
    #[index]
    pub destination_account: Pubkey
}

#[event]
pub struct SetUpgradeAuthorityEvent {
    // Signer (user)
    #[index]
    pub signer: Pubkey,
    // Program account address
    #[index]
    pub program_account: Pubkey,
    // New authority destination account address
    #[index]
    pub upgrade_authority_account: Pubkey
}

#[event]
pub struct UpgradeProgramEvent {
    // Signer (user)
    #[index]
    pub signer: Pubkey,
    // Program account address
    #[index]
    pub program_account: Pubkey,
    // New program buffer account address
    #[index]
    pub buffer_account: Pubkey,
    // Spill account address
    #[index]
    pub spill_account: Pubkey
}