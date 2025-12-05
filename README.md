# Token-2022 Paradox Edition

> SPL Token-2022 implementation with transfer fees, LP growth, and vesting mechanics.

**Made by [Parad0x-Labs](https://x.com/Parad0x-Labs) for Solana**

---

## 🚀 Devnet Deployment

| Component | Address | Link |
|-----------|---------|------|
| **PDOX Token V3** | `9umyHgCSv6xuAv6bczUsR7hBKqyCAZCmPcc4eVhAGrfN` | [View](https://solscan.io/token/9umyHgCSv6xuAv6bczUsR7hBKqyCAZCmPcc4eVhAGrfN?cluster=devnet) |
| **LP Pool** | Orca Whirlpool SOL/PDOX | [View](https://www.orca.so/pools?chainId=solanaDevnet&tokens=So11111111111111111111111111111111111111112&tokens=9umyHgCSv6xuAv6bczUsR7hBKqyCAZCmPcc4eVhAGrfN) |
| **Parent Program** | `7j4qvD77zadbvrKYmahMQbFS5f8tEseW9kj62LYuWmer` | [View](https://solscan.io/account/7j4qvD77zadbvrKYmahMQbFS5f8tEseW9kj62LYuWmer?cluster=devnet) |

**Token Specs:**
- Type: Token-2022 (SPL Token Extensions)
- Transfer Fee: 3%
- Supply: 10,000,000 PDOX (mint authority retained)
- LP: Orca Whirlpool (SOL/PDOX) - 10 SOL + 10M PDOX

---

## Overview

A production-ready Token-2022 token with:

- ✅ **Transfer Fee Extension** (configurable 1-3%)
- ✅ **LP Growth Manager** (fees auto-grow liquidity)
- ✅ **Dev Vesting Vault** (cliff + linear unlock)
- ✅ **DAO Treasury** (governance-controlled)
- ✅ **Armageddon Mode** ([docs](./ARMAGEDDON.md)) - emergency LP protection

## Integration

This is a research SDK. Core logic done, DEX-specific hooks are yours to implement. See [INTEGRATION.md](./INTEGRATION.md)

---

## Quick Start

```bash
# Install
npm install @Parad0x-Labs/token-2022-paradox

# Or clone and build
git clone https://github.com/Parad0x-Labs/token-2022-paradox
cd token-2022-paradox
npm install
anchor build
```

## Usage

### 1. Create Token

```typescript
import { createParadoxToken } from '@Parad0x-Labs/token-2022-paradox';

const token = await createParadoxToken({
  name: 'My Token',
  symbol: 'MTK',
  decimals: 9,
  totalSupply: 1_000_000_000,
  transferFeeBps: 300,  // 3%
  maxTransferFee: 1_000_000_000,  // 1 SOL worth
});

console.log('Token Mint:', token.mint.toBase58());
```

### 2. Initialize LP Growth

```typescript
import { initLpGrowth } from '@Parad0x-Labs/token-2022-paradox';

await initLpGrowth({
  mint: token.mint,
  lpPoolAddress: poolAddress,
  minFeeThreshold: 0.1 * LAMPORTS_PER_SOL,  // Min 0.1 SOL to trigger
  cooldownSeconds: 86400,  // 24h between growths
});
```

### 3. Setup Vesting

```typescript
import { initDevVesting } from '@Parad0x-Labs/token-2022-paradox';

await initDevVesting({
  mint: token.mint,
  totalAllocation: 100_000_000,  // 100M tokens
  liquidAtTge: 0,  // 0% liquid at launch
  cliffMonths: 6,
  vestingMonths: 36,
});
```

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                     TOKEN-2022 PARADOX                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐      │
│  │   TRANSFER   │───▶│     FEE      │───▶│   LP GROWTH  │      │
│  │    (3%)      │    │  COLLECTOR   │    │   MANAGER    │      │
│  └──────────────┘    └──────────────┘    └──────────────┘      │
│                             │                    │               │
│                             ▼                    ▼               │
│                      ┌──────────────┐    ┌──────────────┐       │
│                      │    BURN      │    │  ADD TO LP   │       │
│                      │   (15%)      │    │   (70%)      │       │
│                      └──────────────┘    └──────────────┘       │
│                                                                  │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐      │
│  │  DEV VEST    │    │ DAO TREASURY │    │  ARMAGEDDON  │      │
│  │  (cliff+     │    │ (governance) │    │  (emergency) │      │
│  │   linear)    │    │              │    │              │      │
│  └──────────────┘    └──────────────┘    └──────────────┘      │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

## Configuration

| Parameter | Default | Range | Description |
|-----------|---------|-------|-------------|
| `transferFeeBps` | 300 | 100-300 | Transfer fee in basis points |
| `lpShareBps` | 7000 | 6000-8000 | % of fees to LP (70%) |
| `burnShareBps` | 1500 | 1000-2000 | % of fees burned (15%) |
| `treasuryShareBps` | 1500 | 1000-2000 | % to treasury (15%) |
| `cliffSeconds` | 15552000 | - | 6 month cliff |
| `vestingSeconds` | 94608000 | - | 36 month vesting |

## File Structure

```
token-2022-paradox/
├── programs/
│   └── paradox_token/
│       └── src/
│           ├── lib.rs              # Program entry
│           ├── state/
│           │   ├── token_config.rs # Token configuration
│           │   ├── lp_growth.rs    # LP Growth Manager
│           │   ├── vesting.rs      # Dev/DAO vesting
│           │   └── armageddon.rs   # Emergency mode
│           └── instructions/
│               ├── create_token.rs
│               ├── init_lp_growth.rs
│               ├── execute_lp_growth.rs
│               ├── init_vesting.rs
│               ├── request_unlock.rs
│               └── trigger_armageddon.rs
├── sdk/
│   └── src/
│       ├── index.ts
│       ├── token.ts
│       ├── lp.ts
│       └── vesting.ts
├── tests/
├── Anchor.toml
├── Cargo.toml
└── package.json
```

## License

**Business Source License 1.1**

- ✅ View, study, fork for personal/educational use
- ✅ Run on devnet/testnet
- ❌ Commercial use requires license until December 2028
- ✅ After December 2028: Converts to MIT

**Attribution Required:**
```
Made by Parad0x-Labs for Solana
https://x.com/Parad0x-Labs
```

## Links

- Twitter: [@Parad0x-Labs](https://x.com/Parad0x-Labs)
- Main Project: [PHANTOM PARADOX](https://Parad0x-Labs.github.io/test/)
- Docs: [Token Specs](https://Parad0x-Labs.github.io/test/docs/token.html)

---

*Made by [Parad0x-Labs](https://x.com/Parad0x-Labs) for Solana*

