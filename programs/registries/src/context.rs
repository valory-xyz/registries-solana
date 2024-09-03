use anchor_lang::prelude::*;
use anchor_lang::system_program;

use crate::{
    state::{Config},
};

#[derive(Accounts)]
pub struct InitializeRegistries<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,

    #[account(
        init,
        payer = signer,
        seeds = [Config::SEED_PREFIX],
        bump,
        space = Config::LEN
    )]
    /// Config account, which saves program data useful for other instructions.
    pub config: Box<Account<'info, Config>>,

    #[account(address = system_program::ID)]
    pub system_program: Program<'info, System>,
    //#[account(address = sysvar::rent::ID)]
    pub rent: Sysvar<'info, Rent>
}
