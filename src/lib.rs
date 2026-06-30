#![no_std]

mod admin;
mod balance;
mod errors;
mod events;
pub mod example_consumer;
pub mod interface;
pub mod nft;
mod storage;
mod vault;

pub use nft::StakeReceiptNFT;
pub use vault::VaultContract;

#[cfg(test)]
mod test;

#[cfg(test)]
mod test_integration;
