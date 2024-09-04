use anchor_lang::prelude::*;

#[account]
#[derive(Default)]
/// Config account data.
pub struct Config {
    // Config bump
    pub bump: [u8; 1],
    // Total number of services
    pub num_services: u64
}

impl Config {
    pub const LEN: usize = 8 // discriminator
        + 1  // bump
        + 8  // Total number of services
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