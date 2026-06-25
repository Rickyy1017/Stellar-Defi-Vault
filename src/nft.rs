use soroban_sdk::{contract, contractimpl, contracttype, contracterror, Address, Env};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum NftError {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    NotMinter = 3,
    NonTransferable = 4,
    NoReceipt = 5,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct Receipt {
    pub pool_contract: Address,
    pub staked_amount: i128,
    pub staked_at_ledger: u32,
}

#[contracttype]
#[derive(Clone)]
enum NftDataKey {
    Minter,
    Receipt(Address),
}

#[contract]
pub struct StakeReceiptNFT;

#[contractimpl]
impl StakeReceiptNFT {
    /// Initialize the NFT contract. `minter` is the vault contract allowed to mint/burn.
    pub fn initialize(env: Env, minter: Address) -> Result<(), NftError> {
        if env.storage().instance().has(&NftDataKey::Minter) {
            return Err(NftError::AlreadyInitialized);
        }
        env.storage().instance().set(&NftDataKey::Minter, &minter);
        Ok(())
    }

    /// Mint a non-transferable receipt NFT to `to`. Only the registered minter (vault) may call this.
    pub fn mint(
        env: Env,
        to: Address,
        pool_contract: Address,
        staked_amount: i128,
        staked_at_ledger: u32,
    ) -> Result<(), NftError> {
        let minter: Address = env
            .storage()
            .instance()
            .get(&NftDataKey::Minter)
            .ok_or(NftError::NotInitialized)?;
        minter.require_auth();

        let receipt = Receipt {
            pool_contract,
            staked_amount,
            staked_at_ledger,
        };
        env.storage()
            .persistent()
            .set(&NftDataKey::Receipt(to), &receipt);
        Ok(())
    }

    /// Burn the receipt for `user`. Only the registered minter (vault) may call this.
    pub fn burn(env: Env, user: Address) -> Result<(), NftError> {
        let minter: Address = env
            .storage()
            .instance()
            .get(&NftDataKey::Minter)
            .ok_or(NftError::NotInitialized)?;
        minter.require_auth();

        env.storage()
            .persistent()
            .remove(&NftDataKey::Receipt(user));
        Ok(())
    }

    /// Transfer always reverts — receipts are non-transferable (soulbound).
    pub fn transfer(_env: Env, _from: Address, _to: Address) -> Result<(), NftError> {
        Err(NftError::NonTransferable)
    }

    /// Returns true if `user` currently holds a receipt.
    pub fn has_receipt(env: Env, user: Address) -> bool {
        env.storage()
            .persistent()
            .has(&NftDataKey::Receipt(user))
    }

    /// Returns the receipt metadata for `user`, if one exists.
    pub fn get_receipt(env: Env, user: Address) -> Result<Receipt, NftError> {
        env.storage()
            .persistent()
            .get(&NftDataKey::Receipt(user))
            .ok_or(NftError::NoReceipt)
    }
}
