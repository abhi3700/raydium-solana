use anchor_client::Cluster;
use anyhow::anyhow;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::{ops::Sub, str::FromStr};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mint = Pubkey::from_str("2FGEMK1D324CHEdLyZTzZ3k8croXEqLHQAnvonuTMifH")?;
    let rpc_client = RpcClient::new(Cluster::Devnet.url().to_string());
    let token_amount_struct = rpc_client.get_token_supply(&mint).await?;
    println!("{:?}", token_amount_struct);
    let total_supply = token_amount_struct.ui_amount.ok_or_else(|| anyhow!(".."))?;
    println!("Total supply: {:#?}", total_supply);

    let decimals = token_amount_struct.decimals;
    println!("Decimals: {:#?}", decimals);

    let token_account = Pubkey::from_str("B8KNVa68aKEDGWZfLWPQgbwgxXouaHNYtDkkbhFCJJYr")?;
    let token_minter_balance = rpc_client
        .get_token_account_balance(&token_account)
        .await?
        .ui_amount
        .ok_or_else(|| anyhow!(".."))?;
    println!("Balance of token mint: {:#?}", token_minter_balance);
    let circulating_supply: f64 = total_supply.sub(token_minter_balance);
    println!(
        "Circulating supply (already distributed): {:#?}",
        circulating_supply
    );

    Ok(())
}
