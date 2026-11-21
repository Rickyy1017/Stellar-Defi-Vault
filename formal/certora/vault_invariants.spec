// Certora Sunbeam prototype for the Stellar DeFi Vault (issue #877).
//
// This file is a specification prototype, not a passing CI check. The rules
// below describe the intended obligations in a simplified CVL model; they have
// not been parsed or run by the Certora Prover. They are intended to be adapted
// to the Sunbeam SVM harness. See docs/FORMAL_VERIFICATION.md for the
// evaluation, unproven properties and the decision not to run this in CI.
//
// The contract is a Soroban Rust contract. Sunbeam exposes it to CVL through
// a generated harness; the method names below match the public entrypoints
// used by the proof obligations:
//   total_shares(), shares_of(address), preview_redeem(uint256),
//   stake(address, uint256), withdraw(address, uint256), add_yield(uint256)
//
// Ghost state mirrors the raw Symbol-keyed storage in src/balance.rs:
//   totalShares      <- TotalShares
//   totalDeposited   <- TotalDeposited
//   shareBalance[u]  <- ShareBalance(u)

using VaultContract as vault;

ghost mathint totalShares;
ghost mathint totalDeposited;
ghost mapping(address => mathint) shareBalance;

// INV-1: every minted share is attributed to a staker and the total is the
// sum of the individual balances.
rule share_conservation() {
    assert totalShares == sum(shareBalance);
}

// INV-2: accounting values are never negative.
rule non_negative_accounting() {
    assert totalShares >= 0;
    assert totalDeposited >= 0;
    assert forall address u . shareBalance[u] >= 0;
}

// INV-4: redeeming the whole supply returns exactly the tracked principal.
rule full_pool_redeemable() {
    assert totalShares > 0 => preview_redeem(totalShares) == totalDeposited;
}

// INV-5: adding yield without minting shares cannot lower the share price.
rule share_price_monotonic_under_yield(uint256 delta) {
    mathint priceBefore = totalDeposited / totalShares;
    mathint priceAfter = (totalDeposited + delta) / totalShares;
    assert delta > 0 => priceAfter >= priceBefore;
}

// INV-6: a deposit followed by a withdrawal of the minted shares cannot
// return more than was deposited.
rule no_value_creation_over_cycle(address user, uint256 amount) {
    uint256 minted;
    uint256 returned;
    stake@withrevert(user, amount);
    minted = shareBalance[user];
    withdraw@withrevert(user, minted);
    returned = totalDeposited;
    assert returned <= amount;
}

// INV-3 (partial): the vault's token balance must back tracked principal.
// The live token contract is modelled as an uninterpreted function; this rule
// states the obligation the harness must discharge, not a proof of it.
rule solvency_obligation() {
    assert token_balance(vault) >= totalDeposited;
}
