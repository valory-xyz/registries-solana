use anchor_lang::prelude::*;
use anchor_spl::token::{self, Transfer};
use solana_program::{
    pubkey::Pubkey,
    program::invoke_signed
};
use spl_token::instruction::{set_authority, AuthorityType};

pub use context::*;
pub use errors::*;
pub use events::*;
pub use state::*;

pub mod context;
pub mod errors;
pub mod events;
pub mod state;

declare_id!("3bEWaJfLxAe5qTwiqD1hVjLhqdKTcLVrkZZGZM43bWNe");

#[program]
pub mod registries {
    use super::*;
    use solana_program::pubkey;

    /// Initializes a Registries account, aka Config, that stores state data.
    pub fn initialize(ctx: Context<InitializeRegistries>) -> Result<()> {
        // Get the config account
        let config = &mut ctx.accounts.config;

        // Assign initialization parameters
        config.bump = [ctx.bumps.config];

        // Set zero initial values
        config.total_sol_transferred = 0;

        Ok(())
    }
}
