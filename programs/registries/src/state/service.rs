use anchor_lang::prelude::*;

#[derive(Default, AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq)]
pub struct AgentParams {
    pub agent_id: u32,
    pub num_agent_instances: u32,
    pub bond: u64
}

impl AgentParams {
    pub const LEN: usize =
          32  // agent id
        + 32  // num agent instances
        + 64  // bond
    ;
}


#[account]
#[derive(Default)]
/// Service account data.
pub struct Service {
    // Service token
    pub token: Pubkey,
    // Service security deposit
    pub security_deposit: u64,
    // Service multisig address
    pub multisig: Pubkey,
    // IPFS hashes pointing to the config metadata
    pub config_hash: [u8; 32],
    // Agent instance signers threshold: must no less than ceil((n * 2 + 1) / 3) of all the agent instances combined
    // This number will be enough to have ((2^32 - 1) * 3 - 1) / 2, which is bigger than 6.44b
    pub threshold: u32,
    // Total number of agent instances. We assume that the number of instances is bounded by 2^32 - 1
    pub max_num_agent_instances: u32,
    // Actual number of agent instances. This number is less or equal to maxNumAgentInstances
    pub num_agent_instances: u32,
    // Service state
    pub state: u8,
    // Set of canonical agent Ids for the service, each agent corresponding number of agent instances,
    // and a bond corresponding to each agent Id
    pub agent_params: Vec<AgentParams>,
}

impl Service {
    pub const LEN: usize = 8  // discriminator
        + 32    // token address
        + 8     // security deposit
        + 32    // multisig address
        + 32    // config hash
        + 4     // threshold
        + 4     // max num instances
        + 4     // current num instances
        + AgentParams::LEN * 16  // agents info (max 16)
    ;
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