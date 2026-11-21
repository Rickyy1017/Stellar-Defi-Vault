#![cfg(test)]
//! `insta` snapshots of the topics and data emitted by public entrypoints
//! (issue #878).
//!
//! Each test drives one public entrypoint and snapshots the events whose
//! topic[0] is one of the documented symbols for that entrypoint. Addresses
//! are rendered as the literal `address`, so the snapshot is stable across runs
//! even though `Address::generate` is random; symbols, integers, booleans,
//! strings, vectors, and maps are rendered by value.
//!
//! An intentional schema change is accepted with `cargo insta accept`, then
//! reviewed in the diff like any other interface change. CI runs with
//! `INSTA_UPDATE=no`, so an unaccepted change fails the build. See
//! `docs/EVENT_SNAPSHOTS.md`.

extern crate std;

use std::string::String as StdString;

use soroban_sdk::{
    testutils::{Address as _, Events as _, Ledger as _},
    token,
    xdr::{ContractEventBody, Int128Parts, ScVal, UInt128Parts},
    Address, Env,
};

use crate::{
    storage::PauseReason,
    vault::{VaultContract, VaultContractClient, STELLAR_LEDGERS_PER_YEAR},
};

struct Fixture<'a> {
    env: Env,
    vault: VaultContractClient<'a>,
    token_admin: token::StellarAssetClient<'a>,
    admin: Address,
    alice: Address,
    bob: Address,
}

impl<'a> Fixture<'a> {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().with_mut(|li| {
            li.min_temp_entry_ttl = 10_000_000;
            li.min_persistent_entry_ttl = 10_000_000;
            li.max_entry_ttl = 10_000_000;
        });

        let admin = Address::generate(&env);
        let alice = Address::generate(&env);
        let bob = Address::generate(&env);
        let token_address = env.register_stellar_asset_contract(admin.clone());
        let token_admin = token::StellarAssetClient::new(&env, &token_address);
        token_admin.mint(&alice, &20_000_000);
        token_admin.mint(&admin, &20_000_000);

        let vault_id = env.register_contract(None, VaultContract);
        let vault = VaultContractClient::new(&env, &vault_id);
        vault.initialize(&admin, &token_address, &0_u32, &None, &None);

        Fixture {
            env,
            vault,
            token_admin,
            admin,
            alice,
            bob,
        }
    }

    fn fund_rewards(&self) {
        self.token_admin.mint(&self.admin, &5_000_000);
        self.vault.fund_reward_pool(&self.admin, &5_000_000);
        self.vault.set_reward_rate_bps(&1_000_u32);
    }
}

fn i128_value(parts: &Int128Parts) -> i128 {
    ((parts.hi as i128) << 64) | (parts.lo as i128)
}

fn u128_value(parts: &UInt128Parts) -> u128 {
    ((parts.hi as u128) << 64) | (parts.lo as u128)
}

/// Stable, address-free rendering of an XDR value.
fn scval_repr(value: &ScVal) -> StdString {
    match value {
        ScVal::Bool(v) => std::format!("bool({v})"),
        ScVal::Void => "void".into(),
        ScVal::U32(v) => std::format!("u32({v})"),
        ScVal::I32(v) => std::format!("i32({v})"),
        ScVal::U64(v) => std::format!("u64({v})"),
        ScVal::I64(v) => std::format!("i64({v})"),
        ScVal::U128(parts) => std::format!("u128({})", u128_value(parts)),
        ScVal::I128(parts) => std::format!("i128({})", i128_value(parts)),
        ScVal::Bytes(bytes) => std::format!("bytes(len={})", bytes.0.len()),
        ScVal::String(string) => std::format!("string({})", string.0),
        ScVal::Symbol(symbol) => std::format!("symbol({})", symbol.0),
        ScVal::Vec(Some(items)) => {
            let parts: std::vec::Vec<StdString> = items.0.iter().map(scval_repr).collect();
            std::format!("vec[{}]", parts.join(", "))
        }
        ScVal::Vec(None) => "vec[]".into(),
        ScVal::Map(Some(entries)) => {
            let parts: std::vec::Vec<StdString> = entries
                .0
                .iter()
                .map(|entry| {
                    std::format!("{} => {}", scval_repr(&entry.key), scval_repr(&entry.val))
                })
                .collect();
            std::format!("map{{{}}}", parts.join(", "))
        }
        ScVal::Map(None) => "map{}".into(),
        ScVal::Address(_) => "address".into(),
        ScVal::ContractInstance(_) => "contract_instance".into(),
        ScVal::LedgerKeyContractInstance => "ledger_key_contract_instance".into(),
        ScVal::LedgerKeyNonce(_) => "ledger_key_nonce".into(),
        _ => "other".into(),
    }
}

/// Render the events from the most recent contract invocation whose topic[0]
/// symbol is in `symbols`.
fn trace_for(env: &Env, symbols: &[&str]) -> StdString {
    let events = env.events().all();
    let mut lines: std::vec::Vec<StdString> = std::vec::Vec::new();

    for event in events.events() {
        let body = match &event.body {
            ContractEventBody::V0(body) => body,
        };
        let Some(ScVal::Symbol(symbol)) = body.topics.first() else {
            continue;
        };
        let name = std::format!("{}", symbol.0);
        if !symbols.contains(&name.as_str()) {
            continue;
        }

        let topics: std::vec::Vec<StdString> = body.topics.iter().map(scval_repr).collect();
        lines.push(std::format!(
            "{}: topics=[{}] data={}",
            lines.len(),
            topics.join(", "),
            scval_repr(&body.data)
        ));
    }

    lines.join("\n")
}

#[test]
fn deposit_event_schema_is_stable() {
    let f = Fixture::new();
    f.vault.deposit(&f.alice, &1_000_000, &None);

    let trace = trace_for(&f.env, &["deposit"]);
    assert!(!trace.is_empty(), "deposit emitted no event");
    insta::assert_snapshot!("deposit", trace);
}

#[test]
fn withdraw_event_schema_is_stable() {
    let f = Fixture::new();
    f.vault.deposit(&f.alice, &1_000_000, &None);
    f.vault.withdraw(&f.alice, &400_000);

    let trace = trace_for(&f.env, &["withdraw"]);
    assert!(!trace.is_empty(), "withdraw emitted no event");
    insta::assert_snapshot!("withdraw", trace);
}

#[test]
fn claim_event_schema_is_stable() {
    let f = Fixture::new();
    f.fund_rewards();
    f.vault.stake(&f.alice, &1_000_000);

    f.env.ledger().with_mut(|li| {
        li.sequence_number = STELLAR_LEDGERS_PER_YEAR;
    });
    f.vault.claim(&f.alice);

    let trace = trace_for(&f.env, &["claimed"]);
    assert!(!trace.is_empty(), "claim emitted no event");
    insta::assert_snapshot!("claim", trace);
}

#[test]
fn pause_event_schema_is_stable() {
    let f = Fixture::new();
    f.vault.pause(
        &PauseReason::Other,
        &soroban_sdk::String::from_str(&f.env, "test"),
    );

    let trace = trace_for(&f.env, &["paused"]);
    assert!(!trace.is_empty(), "pause emitted no event");
    insta::assert_snapshot!("pause", trace);
}

#[test]
fn unpause_event_schema_is_stable() {
    let f = Fixture::new();
    f.vault.pause(
        &PauseReason::Other,
        &soroban_sdk::String::from_str(&f.env, "test"),
    );
    f.vault.unpause();

    let trace = trace_for(&f.env, &["unpaused"]);
    assert!(!trace.is_empty(), "unpause emitted no event");
    insta::assert_snapshot!("unpause", trace);
}

#[test]
fn fee_event_schema_is_stable() {
    let f = Fixture::new();
    f.vault.set_unstake_fee_bps(&f.admin, &100_u32);

    let trace = trace_for(&f.env, &["fee_set"]);
    assert!(!trace.is_empty(), "set_unstake_fee_bps emitted no event");
    insta::assert_snapshot!("fee", trace);
}

#[test]
fn yield_event_schema_is_stable() {
    let f = Fixture::new();
    f.vault.add_yield(&f.admin, &50_000);

    let trace = trace_for(&f.env, &["yield_add"]);
    assert!(!trace.is_empty(), "add_yield emitted no event");
    insta::assert_snapshot!("yield", trace);
}

#[test]
fn propose_admin_event_schema_is_stable() {
    let f = Fixture::new();
    f.vault.propose_admin(&f.admin, &f.bob);

    let trace = trace_for(&f.env, &["adm_prop"]);
    assert!(!trace.is_empty(), "propose_admin emitted no event");
    insta::assert_snapshot!("propose_admin", trace);
}

#[test]
fn accept_admin_event_schema_is_stable() {
    let f = Fixture::new();
    f.vault.propose_admin(&f.admin, &f.bob);
    f.vault.accept_admin(&f.bob);

    let trace = trace_for(&f.env, &["admin_set", "adm_act", "adm_acpt"]);
    assert!(!trace.is_empty(), "accept_admin emitted no event");
    insta::assert_snapshot!("accept_admin", trace);
}

#[test]
fn deprecated_transfer_admin_event_schema_is_stable() {
    let f = Fixture::new();
    #[allow(deprecated)]
    f.vault.transfer_admin(&f.bob);

    let trace = trace_for(&f.env, &["deprec", "admin_set", "adm_act"]);
    assert!(!trace.is_empty(), "transfer_admin emitted no event");
    insta::assert_snapshot!("deprecated_transfer_admin", trace);
}

#[test]
fn withdraw_only_event_schema_is_stable() {
    let f = Fixture::new();
    f.vault.enter_withdraw_only_mode(&f.admin);

    let trace = trace_for(&f.env, &["wd_only"]);
    assert!(
        !trace.is_empty(),
        "enter_withdraw_only_mode emitted no event"
    );
    insta::assert_snapshot!("withdraw_only", trace);
}

#[test]
fn sunset_event_schema_is_stable() {
    let f = Fixture::new();
    f.vault.initiate_sunset(&f.admin, &1_000_000_u32);

    let trace = trace_for(&f.env, &["snst_ini"]);
    assert!(!trace.is_empty(), "initiate_sunset emitted no event");
    insta::assert_snapshot!("sunset", trace);
}
