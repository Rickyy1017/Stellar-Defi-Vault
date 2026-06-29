#![no_std]

mod admin;
mod balance;
mod errors;
mod events;
pub mod nft;
mod storage;
mod vault;
pub mod interface;
pub mod example_consumer;

pub use nft::StakeReceiptNFT;
pub use vault::VaultContract;

#[cfg(test)]
mod test;

#[cfg(test)]
mod test_integration;
