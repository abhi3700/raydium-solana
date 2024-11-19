# raydium-solana

Play with **Raydium DEX on Solana**.

## Objectives

Network: Devnet

Users:

> You can create yours manually or run this [script](./client/src/create_token.rs) and save it like this for quick use in CLI whenever you need.

```toml
[admin]
path = ~/.config/solana/id.json
address = DdSX6JDnN4KmBbc5pSDW7e18uT43R2MiWWwvE268wSJc

[alice]
path = ~/.config/solana/alice.json
address = CGdKiDkufmnjMhQDtbBKbCLc7jgT9jTXe5pdnywcNeXd

[bob]
path = ~/.config/solana/bob.json
address = 7i35XMGWhbrKBsQemMRVeAHCwxNKpnWyNZB7d9NMsQs1
```

- [x] Create SPL Tokens

<details>
  <summary>Details</summary>

- `DONUT`
  - Token mint: `8jHJmZhCcc6Vp5Hdb9kh6hPyDjyQow5iNSNuv7BbjB6T`
  - Token account: `3YgxQbsj1xoWotFC1B6oEgHktHDqC1LDAFb5pbWKW5J3`
  - decimals: 6
  - ATA for users:
    - Alice: `9TnkjkD7FS3Xpq2LfnmeQNasWE9PPaF3B2T3psgbRoeg`
    - Bob: `99jajJmVXfszVdKwHQUjY5yiKCmUtF5yg8dQGRS9Aspy`
  - Funded users ✅
- `PIE`
  - Token mint: `2FGEMK1D324CHEdLyZTzZ3k8croXEqLHQAnvonuTMifH`
  - Token account: `B8KNVa68aKEDGWZfLWPQgbwgxXouaHNYtDkkbhFCJJYr`
  - decimals: 6
  - ATA for users:
    - Alice: `7Xg6rgAu2axWWhk65AmVh2wWi5qtuhEoCtzD4xdfHLpa`
    - Bob: `HTvXsDGdsdxgwg3Twb7RQS32uuFCVrNE6ahW7Ag5G18W`
  - Funded users ✅

</details>

- [ ] Call `AddLiquidity` to add liquidity to the `DONUT:PIE` pair.
- [ ] Call `RemoveLiquidity` to remove liquidity from the `DONUT:PIE` pair.
- [ ] Call them via a custom program: '**Bundler**'.

## Usage

1. Copy paste the [.config.example.ini](./config.example.ini) file and fill in the details.
2. Create your own tokens, I suggest. Simply use my [script](https://github.com/abhi3700/sol-playground/blob/main/scripts/token.sh) with custom values if you want to create your own token.
3. Run:

```sh
cargo run -r --bin client
```

## Status

This is a work in progress.

Problem:

```
"Play with RaydiumV3 AMM"
mint0:7tUqb71uKD7ANVPZYfm4vYk2YCyA7sTMnkwVa6hCqBji, mint1:7tUqb71uKD7ANVPZYfm4vYk2YCyA7sTMnkwVa6hCqBji, price:1.5
tick:4054, price:1.5, sqrt_price_x64:22592555198148960256, amm_config_key:B9H7TR8PSjJT7nuW2tuPkFC63z7drtMZ4LoCtD7PrCN1
[client/src/instructions.rs:36:5] &payer.pubkey() = CGdKiDkufmnjMhQDtbBKbCLc7jgT9jTXe5pdnywcNeXd
[client/src/instructions.rs:50:5] &pool_account_key = 4MBAKamG93Cz14ACXqsSgXBRH6hE8bBwqdzea6rjdF28
[client/src/instructions.rs:59:5] &token_vault_0 = 7DeXAToFxoHDCdy2jqPgexVZ56aWPw7jpcCS1Vdc9dGi
[client/src/instructions.rs:68:5] &token_vault_1 = 7DeXAToFxoHDCdy2jqPgexVZ56aWPw7jpcCS1Vdc9dGi
[client/src/instructions.rs:76:5] &observation_key = 4CPgcETZNc7eTmx1jySAKM3BqoMnDDd3hJQQt78jdHed
Error: Error processing Instruction 0: custom program error: 0x0

Caused by:
    Error processing Instruction 0: custom program error: 0x0
```

Basically, the pool account is not getting created.
