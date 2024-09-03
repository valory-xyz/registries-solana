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
