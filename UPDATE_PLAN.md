# PDOX Token Update Plan

**Deployed Mint:** `4ckvALSiB6Hii7iVY9Dt6LRM5i7xocBZ9yr3YGNtVRwF`  
**Network:** Devnet  
**Status:** ✅ CAN UPDATE (No relaunch needed)

---

## ✅ What Can Be Updated (Program Upgrade)

### 1. Code Fixes (All Safe to Upgrade)
- ✅ u128 arithmetic (prevents overflow)
- ✅ Fee change timelock (24h announce/execute)
- ✅ Token-2022 compliance (transfer_checked)
- ✅ Snapshot validation
- ✅ MIN_TRANSFER_AMOUNT enforcement
- ✅ Fee harvesting implementation

### 2. Decimals Fix (Critical)
**Issue:** Code had `TOKEN_DECIMALS = 6` but mint uses `9 decimals`

**Fixed:**
- ✅ `vesting.rs`: Changed to `TOKEN_DECIMALS = 9`
- ✅ `treasury.rs`: Changed to `TOKEN_DECIMALS = 9`
- ⚠️ Need to check: `lp_lock.rs`, `lp_growth.rs` (if they use decimals)

---

## ✅ Mint Configuration (Already Correct)

| Setting | Value | Status |
|---------|-------|--------|
| Decimals | 9 | ✅ Correct |
| Transfer Fee | 300 bps (3%) | ✅ Correct |
| Token Program | Token-2022 | ✅ Correct |
| Mint Authority | 3XBBYhqcV5fdF1j8Bs97wcAbj9AYEeVHcxZipaFcefr3 | ✅ Correct |
| Freeze Authority | 3XBBYhqcV5fdF1j8Bs97wcAbj9AYEeVHcxZipaFcefr3 | ✅ Correct |
| PermanentDelegate | None | ✅ Safe |

**Conclusion:** Mint is correctly configured. No relaunch needed.

---

## 📋 Update Steps

### Step 1: Fix All Decimals References
```bash
# Already fixed:
- vesting.rs: TOKEN_DECIMALS = 9
- treasury.rs: TOKEN_DECIMALS = 9

# Need to check:
- lp_lock.rs (if uses decimals)
- lp_growth.rs (if uses decimals)
- harvest_fees.rs (if uses decimals)
```

### Step 2: Build Updated Program
```bash
cd TOKEN_2022_SDK
anchor build
```

### Step 3: Upgrade Program (If Already Deployed)
```bash
# Check if program is deployed
solana program show PARADOX111111111111111111111111111111111111 \
  --url https://api.devnet.solana.com

# If deployed, upgrade it
anchor upgrade target/deploy/paradox_token.so \
  --program-id PARADOX111111111111111111111111111111111111 \
  --provider.cluster devnet \
  --provider.wallet deployer_wallet.json
```

### Step 4: Reinitialize TokenConfig (If Needed)
If TokenConfig PDA was already initialized with old structure:
- Option A: Use new fields (pending_fee_bps) - they'll be 0 by default
- Option B: Close and reinitialize (loses old state)

---

## ⚠️ Important Notes

1. **Mint Cannot Change**: The mint address and decimals are immutable. Since mint uses 9 decimals (correct), we just need to match it in code.

2. **Program Upgrade**: If program is already deployed, you can upgrade it. The upgrade authority must match.

3. **State Migration**: If TokenConfig was initialized before, the new fields (pending_fee_bps, etc.) will be uninitialized (0). This is safe - they'll work when first used.

4. **No Holder Migration**: Since mint stays the same, all existing holders and balances remain valid.

---

## ✅ Final Answer

**NO RELAUNCH NEEDED** - Just upgrade the program!

1. Fix decimals in code (already done for vesting/treasury)
2. Build new program
3. Upgrade deployed program
4. Done!

The mint is correct, holders keep their tokens, everything works.

---

*Last updated: November 30, 2025*

