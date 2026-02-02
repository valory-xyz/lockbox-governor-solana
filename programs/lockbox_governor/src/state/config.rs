use anchor_lang::prelude::*;

#[derive(Default, AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq)]
/// Wormhole program related addresses.
pub struct WormholeAddresses {
    /// [BridgeData](wormhole_anchor_sdk::wormhole::BridgeData) address.
    pub bridge: Pubkey,
    /// [FeeCollector](wormhole_anchor_sdk::wormhole::FeeCollector) address.
    pub fee_collector: Pubkey,
    /// [SequenceTracker](wormhole_anchor_sdk::wormhole::SequenceTracker) address.
    pub sequence: Pubkey,
}

impl WormholeAddresses {
    pub const LEN: usize =
          32 // config
        + 32 // fee_collector
        + 32 // sequence
    ;
}

#[account]
#[derive(Default)]
/// Config account data.
pub struct Config {
    /// Wormhole program's relevant addresses.
    pub wormhole: WormholeAddresses,
    /// AKA consistency level. u8 representation of Solana's
    /// [Finality](wormhole_anchor_sdk::wormhole::Finality).
    pub finality: u8,
    // Config bump
    pub bump: [u8; 1],
    // Foreign chain Id
    pub chain: u16,
    // Foreign emitter address in bytes array
    pub foreign_emitter: [u8; 32],
    // Total SOL amount transferred
    pub total_sol_transferred: u64,
    // Total OLAS amount transferred
    pub total_olas_transferred: u64
}

impl Config {
    pub const LEN: usize = 8 // discriminator
        + WormholeAddresses::LEN
        + 1  // finality
        + 1  // bump
        + 2  // chain Id
        + 32 // foreign emitter
        + 8  // SOL amount transferred
        + 8  // OALS amount transferred
        
    ;
    /// AKA `b"config"`.
    pub const SEED_PREFIX: &'static [u8; 6] = b"config";

    pub fn seeds(&self) -> [&[u8]; 2] {
        [
            //&b"config"[..],
            Self::SEED_PREFIX,
            self.bump.as_ref()
        ]
    }

    /// Convenience method to check whether an address equals the one saved in this account.
    pub fn verify(&self, check_address: &[u8; 32]) -> bool {
        return *check_address == self.foreign_emitter;
    }
}

#[cfg(test)]
pub mod test {
    use super::*;
    use std::mem::size_of;

    #[test]
    fn test_config() -> Result<()> {
        assert_eq!(WormholeAddresses::LEN, std::mem::size_of::<WormholeAddresses>());
        assert_eq!(
            Config::MAXIMUM_SIZE, 
            size_of::<u64>()
            + size_of::<WormholeAddresses>()
            + size_of::<u8>()
            + size_of::<u8>()
            + size_of::<u16>()
            + size_of::<Pubkey>()
            + size_of::<u64>()
            + size_of::<u64>()
        );

        Ok(())
    }
}