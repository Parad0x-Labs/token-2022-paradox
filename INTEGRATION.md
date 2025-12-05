# Integration Guide

This is a research SDK. Core tokenomics logic is complete - DEX-specific parts are yours to implement.

---

## What's Done

- Token-2022 transfer fees ✅
- Fee collection + distribution ✅
- LP lock with timelocks ✅
- Vesting schedules ✅
- Treasury management ✅
- Armageddon mode ✅

---

## What You Implement

The SDK has integration hooks where you plug in YOUR DEX logic:

| Hook | Location | Your Job |
|------|----------|----------|
| LP price calculation | `lp_growth.rs:115` | Fetch price from your DEX (Orca/Raydium/Meteora) |
| LP growth execution | `lp_growth.rs:178` | Call your DEX's add-liquidity CPI |
| Fee harvesting amount | `harvest_fees.rs:125` | Track actual harvested amounts |

---

## Example: Orca Whirlpool

```rust
// In lp_growth.rs - replace placeholder with:
fn get_lp_value_in_sol(&self) -> Result<u64> {
    // Fetch from Orca Whirlpool
    let pool = WhirlpoolState::load(&self.pool_address)?;
    let price = pool.sqrt_price.to_price()?;
    Ok(self.lp_tokens.checked_mul(price)?)
}
```

## Example: Raydium

```rust
fn get_lp_value_in_sol(&self) -> Result<u64> {
    // Fetch from Raydium AMM
    let pool = AmmInfo::load(&self.pool_address)?;
    // ... your implementation
}
```

---

## Why Placeholders?

Every DEX has different:
- Account structures
- CPI interfaces  
- Price calculation methods

We give you the tokenomics engine. You wire it to your DEX.

---

*This is intentional. Not missing code - integration points.*

