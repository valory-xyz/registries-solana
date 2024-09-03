use anchor_lang::prelude::*;
use anchor_spl::token::{self, Transfer};
use spl_token::instruction::{set_authority, AuthorityType};

pub use context::*;
pub use errors::*;
pub use events::*;
pub use state::*;

pub mod context;
pub mod errors;
pub mod events;
pub mod state;

declare_id!("7J1mLX2ozMwU6p6mX7zuXMoZf5SBwLBZrGevJHpXP98k");

#[program]
pub mod registries {
    use super::*;

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
