# Token-2022 Paradox Edition

> SPL Token-2022 implementation with transfer fees, LP growth, and vesting mechanics.

**Made by [LabsX402](https://x.com/LabsX402) for Solana**

---

## Overview

A production-ready Token-2022 token with:

- ✅ **Transfer Fee Extension** (configurable 1-3%)
- ✅ **LP Growth Manager** (fees auto-grow liquidity)
- ✅ **Dev Vesting Vault** (cliff + linear unlock)
- ✅ **DAO Treasury** (governance-controlled)
- ✅ **Armageddon Mode** (emergency LP protection)

## Quick Start

```bash
# Install
npm install @labsx402/token-2022-paradox

# Or clone and build
git clone https://github.com/LabsX402/token-2022-paradox
cd token-2022-paradox
npm install
anchor build
```

## Usage

### 1. Create Token

```typescript
import { createParadoxToken } from '@labsx402/token-2022-paradox';

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
import { initLpGrowth } from '@labsx402/token-2022-paradox';

await initLpGrowth({
  mint: token.mint,
  lpPoolAddress: poolAddress,
  minFeeThreshold: 0.1 * LAMPORTS_PER_SOL,  // Min 0.1 SOL to trigger
  cooldownSeconds: 86400,  // 24h between growths
});
```

### 3. Setup Vesting

```typescript
import { initDevVesting } from '@labsx402/token-2022-paradox';

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
Made by LabsX402 for Solana
https://x.com/LabsX402
```

## Links

- Twitter: [@LabsX402](https://x.com/LabsX402)
- Main Project: [PHANTOM PARADOX](https://labsx402.github.io/test/)
- Docs: [Token Specs](https://labsx402.github.io/test/docs/token.html)

---

*Made by [LabsX402](https://x.com/LabsX402) for Solana*

