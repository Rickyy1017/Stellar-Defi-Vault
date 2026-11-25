//! Shared duplicate checks for address `Vec` parameters (issue #919).

use soroban_sdk::{contracterror, Address, Vec};

/// Typed error when the same address appears more than once in a vector argument.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum VecParamError {
    DuplicateEntry = 1,
}

/// Returns `DuplicateEntry` when any two entries in `addresses` are equal.
/// Bounded O(n²); list lengths are capped at each call site.
pub fn ensure_unique_addresses(addresses: &Vec<Address>) -> Result<(), VecParamError> {
    let n = addresses.len();
    let mut i = 0u32;
    while i < n {
        let a = addresses.get(i).unwrap();
        let mut j = i + 1;
        while j < n {
            if addresses.get(j).unwrap() == a {
                return Err(VecParamError::DuplicateEntry);
            }
            j += 1;
        }
        i += 1;
    }
    Ok(())
}

/// Like [`ensure_unique_addresses`], using only the address field of each pair.
pub fn ensure_unique_address_pairs_bool(
    pairs: &Vec<(Address, bool)>,
) -> Result<(), VecParamError> {
    let n = pairs.len();
    let mut i = 0u32;
    while i < n {
        let (a, _) = pairs.get(i).unwrap();
        let mut j = i + 1;
        while j < n {
            let (b, _) = pairs.get(j).unwrap();
            if &b == &a {
                return Err(VecParamError::DuplicateEntry);
            }
            j += 1;
        }
        i += 1;
    }
    Ok(())
}

/// Like [`ensure_unique_addresses`], using only the address field of each pair.
pub fn ensure_unique_address_pairs_u32(
    pairs: &Vec<(Address, u32)>,
) -> Result<(), VecParamError> {
    let n = pairs.len();
    let mut i = 0u32;
    while i < n {
        let (a, _) = pairs.get(i).unwrap();
        let mut j = i + 1;
        while j < n {
            let (b, _) = pairs.get(j).unwrap();
            if &b == &a {
                return Err(VecParamError::DuplicateEntry);
            }
            j += 1;
        }
        i += 1;
    }
    Ok(())
}

/// Map [`VecParamError`] into the vault's primary error enum (`VaultError` is
/// at the Soroban 50-variant cap; `DuplicateEntry` aliases `InvalidAddress`).
pub fn into_vault_error(err: VecParamError) -> crate::errors::VaultError {
    match err {
        VecParamError::DuplicateEntry => crate::errors::VaultError::DuplicateEntry,
    }
}