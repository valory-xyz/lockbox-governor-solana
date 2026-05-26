use anchor_lang::prelude::error_code;

#[error_code]
/// Errors relevant to this program's malfunction.
pub enum GovernorError {
    #[msg("Invalid foreign emitter")]
    /// Specified foreign emitter has an incorrect address.
    InvalidForeignEmitter,

    #[msg("Invalid foreign chain")]
    /// Specified emitter chain ID has is incorrect.
    InvalidForeignChain,

    #[msg("Wrong upgrade authority")]
    /// Wrong upgrade authority address.
    WrongUpgradeAuthority,

    #[msg("Wrong token mint")]
    /// Wrong token mint.
    WrongTokenMint,

    #[msg("Wrong account owner")]
    /// Wrong account owner.
    WrongAccountOwner,

    #[msg("Wrong account address")]
    /// Wrong account account.
    WrongAccount,

    #[msg("Overflow value")]
    /// Overflow value.
    Overflow
}
