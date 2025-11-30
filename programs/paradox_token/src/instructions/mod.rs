/**
 * Instructions for Token-2022 Paradox Edition
 * 
 * Made by LabsX402 for Solana
 * https://x.com/LabsX402
 */

pub mod init_token_config;
pub mod update_token_config;
pub mod lp_growth;
pub mod lp_lock;
pub mod vesting;
pub mod treasury;
pub mod armageddon;
pub mod fees;

pub use init_token_config::*;
pub use update_token_config::*;
pub use lp_growth::*;
pub use lp_lock::*;
pub use vesting::*;
pub use treasury::*;
pub use armageddon::*;
pub use fees::*;

