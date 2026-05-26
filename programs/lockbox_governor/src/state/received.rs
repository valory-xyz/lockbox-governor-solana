use anchor_lang::prelude::*;

#[account]
#[derive(Default)]
/// Received account.
pub struct Received {
    /// AKA nonce. Practically should always be zero, but we save it anyway.
    pub batch_id: u32,
    /// Keccak256 hash of verified Wormhole message.
    pub wormhole_message_hash: [u8; 32],
    /// Message sequence.
    pub sequence: u64,
}

impl Received {
    pub const LEN: usize = 8 // discriminator
        + 4  // batch_id
        + 32 // wormhole_message_hash
        + 8  // sequence
    ;
    /// AKA `b"received"`.
    pub const SEED_PREFIX: &'static [u8; 8] = b"received";
}

#[cfg(test)]
pub mod test {
    use super::*;
    use std::mem::size_of;

    #[test]
    fn test_received() -> Result<()> {
        assert_eq!(
            Received::LEN,
            size_of::<u64>()
                + size_of::<u32>()
                + size_of::<[u8; 32]>()
                + size_of::<u64>()
        );

        Ok(())
    }
}
