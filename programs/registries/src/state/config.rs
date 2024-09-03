use anchor_lang::prelude::*;

#[account]
#[derive(Default)]
/// Config account data.
pub struct Config {
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
}

// #[cfg(test)]
// pub mod test {
//     use super::*;
//     use std::mem::size_of;
//
//     #[test]
//     fn test_config() -> Result<()> {
//         assert_eq!(WormholeAddresses::LEN, std::mem::size_of::<WormholeAddresses>());
//         assert_eq!(
//             Config::MAXIMUM_SIZE,
//             size_of::<u64>()
//             + size_of::<WormholeAddresses>()
//             + size_of::<u8>()
//             + size_of::<u8>()
//             + size_of::<u16>()
//             + size_of::<Pubkey>()
//             + size_of::<u64>()
//             + size_of::<u64>()
//         );
//
//         Ok(())
//     }
// }