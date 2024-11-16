use std::sync::Arc;

use crate::client::{deserialize_anchor_account, get_nft_account_and_position_by_owner, send_txn};
use crate::instructions::{create_pool_instr, increase_liquidity_instr};
use crate::utils::{
    amount_with_slippage, get_pool_mints_inverse_fee, price_to_sqrt_price_x64, tick_with_spacing,
};
use crate::ClientConfig;
use anchor_client::Program;
use anyhow::Result;
use raydium_amm_v3::libraries::{liquidity_math, tick_math};
use solana_rpc_client::rpc_client::RpcClient;
use solana_sdk::instruction::AccountMeta;
use solana_sdk::program_pack::Pack;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::Transaction;

pub(crate) fn create_pool_tx(
    rpc_client: &RpcClient,
    pool_config: &ClientConfig,
    config_index: u16,
    payer: &Keypair,
    price: f64,
    mint0: Pubkey,
    mint1: Pubkey,
    open_time: u64,
) -> Result<()> {
    let mut price = price;
    let mut mint0 = mint0;
    let mut mint1 = mint1;
    if mint0 > mint1 {
        std::mem::swap(&mut mint0, &mut mint1);
        price = 1.0 / price;
    }
    println!("mint0:{}, mint1:{}, price:{}", mint0, mint1, price);
    let load_pubkeys = vec![mint0, mint1];
    let rsps = rpc_client.get_multiple_accounts(&load_pubkeys)?;
    let mint0_owner = rsps[0].clone().unwrap().owner;
    let mint1_owner = rsps[1].clone().unwrap().owner;
    let mint0_account = spl_token::state::Mint::unpack(&rsps[0].as_ref().unwrap().data).unwrap();
    let mint1_account = spl_token::state::Mint::unpack(&rsps[1].as_ref().unwrap().data).unwrap();
    let sqrt_price_x64 =
        price_to_sqrt_price_x64(price, mint0_account.decimals, mint1_account.decimals);
    let (amm_config_key, __bump) = Pubkey::find_program_address(
        &[
            raydium_amm_v3::states::AMM_CONFIG_SEED.as_bytes(),
            &config_index.to_be_bytes(),
        ],
        &pool_config.raydium_v3_program,
    );
    let tick = tick_math::get_tick_at_sqrt_price(sqrt_price_x64).unwrap();
    println!(
        "tick:{}, price:{}, sqrt_price_x64:{}, amm_config_key:{}",
        tick, price, sqrt_price_x64, amm_config_key
    );

    let create_pool_instr = create_pool_instr(
        &pool_config.clone(),
        amm_config_key,
        mint0,
        mint1,
        mint0_owner,
        mint1_owner,
        pool_config.tickarray_bitmap_extension.unwrap(),
        sqrt_price_x64,
        open_time,
    )?;

    // send
    let signers = vec![payer];
    let recent_hash = rpc_client.get_latest_blockhash()?;
    let txn = Transaction::new_signed_with_payer(
        &create_pool_instr,
        Some(&payer.pubkey()),
        &signers,
        recent_hash,
    );
    let signature = send_txn(&rpc_client, &txn, true)?;
    println!("{}", signature);

    Ok(())
}

pub(crate) async fn increase_liquidity_tx(
    program: Program<Arc<Keypair>>,
    rpc_client: RpcClient,
    pool_config: ClientConfig,
    payer: Keypair,
    tick_lower_price: f64,
    tick_upper_price: f64,
    is_base_0: bool,
    input_amount: u64,
) -> anyhow::Result<()> {
    // load pool to get observation
    dbg!(pool_config.pool_id_account.unwrap());
    // FIXME: create the pool first.
    let pool: raydium_amm_v3::states::PoolState = program
        .account(pool_config.pool_id_account.unwrap())
        .await?;

    // load position
    let (_nft_tokens, positions) = get_nft_account_and_position_by_owner(
        &rpc_client,
        &payer.pubkey(),
        &pool_config.raydium_v3_program,
    );
    let rsps = rpc_client.get_multiple_accounts(&positions)?;
    let mut user_positions = Vec::new();
    for rsp in rsps {
        match rsp {
            None => continue,
            Some(rsp) => {
                let position = deserialize_anchor_account::<
                    raydium_amm_v3::states::PersonalPositionState,
                >(&rsp)?;
                user_positions.push(position);
            }
        }
    }

    let tick_lower_price_x64 =
        price_to_sqrt_price_x64(tick_lower_price, pool.mint_decimals_0, pool.mint_decimals_1);
    let tick_upper_price_x64 =
        price_to_sqrt_price_x64(tick_upper_price, pool.mint_decimals_0, pool.mint_decimals_1);
    let tick_lower_index = tick_with_spacing(
        tick_math::get_tick_at_sqrt_price(tick_lower_price_x64)?,
        pool.tick_spacing.into(),
    );
    let tick_upper_index = tick_with_spacing(
        tick_math::get_tick_at_sqrt_price(tick_upper_price_x64)?,
        pool.tick_spacing.into(),
    );
    println!(
        "tick_lower_index:{}, tick_upper_index:{}",
        tick_lower_index, tick_upper_index
    );
    let tick_lower_price_x64 = tick_math::get_sqrt_price_at_tick(tick_lower_index)?;
    let tick_upper_price_x64 = tick_math::get_sqrt_price_at_tick(tick_upper_index)?;
    let liquidity = if is_base_0 {
        liquidity_math::get_liquidity_from_single_amount_0(
            pool.sqrt_price_x64,
            tick_lower_price_x64,
            tick_upper_price_x64,
            input_amount,
        )
    } else {
        liquidity_math::get_liquidity_from_single_amount_1(
            pool.sqrt_price_x64,
            tick_lower_price_x64,
            tick_upper_price_x64,
            input_amount,
        )
    };
    let (amount_0, amount_1) = liquidity_math::get_delta_amounts_signed(
        pool.tick_current,
        pool.sqrt_price_x64,
        tick_lower_index,
        tick_upper_index,
        liquidity as i128,
    )?;
    println!(
        "amount_0:{}, amount_1:{}, liquidity:{}",
        amount_0, amount_1, liquidity
    );
    // calc with slippage
    let amount_0_with_slippage = amount_with_slippage(amount_0 as u64, pool_config.slippage, true);
    let amount_1_with_slippage = amount_with_slippage(amount_1 as u64, pool_config.slippage, true);
    // calc with transfer_fee
    let transfer_fee = get_pool_mints_inverse_fee(
        &rpc_client,
        pool.token_mint_0,
        pool.token_mint_1,
        amount_0_with_slippage,
        amount_1_with_slippage,
    );
    println!(
        "transfer_fee_0:{}, transfer_fee_1:{}",
        transfer_fee.0.transfer_fee, transfer_fee.1.transfer_fee
    );
    let amount_0_max = (amount_0_with_slippage as u64)
        .checked_add(transfer_fee.0.transfer_fee)
        .unwrap();
    let amount_1_max = (amount_1_with_slippage as u64)
        .checked_add(transfer_fee.1.transfer_fee)
        .unwrap();

    let tick_array_lower_start_index =
        raydium_amm_v3::states::TickArrayState::get_array_start_index(
            tick_lower_index,
            pool.tick_spacing.into(),
        );
    let tick_array_upper_start_index =
        raydium_amm_v3::states::TickArrayState::get_array_start_index(
            tick_upper_index,
            pool.tick_spacing.into(),
        );
    let mut find_position = raydium_amm_v3::states::PersonalPositionState::default();
    for position in user_positions {
        if position.pool_id == pool_config.pool_id_account.unwrap()
            && position.tick_lower_index == tick_lower_index
            && position.tick_upper_index == tick_upper_index
        {
            find_position = position.clone();
        }
    }
    if find_position.nft_mint != Pubkey::default()
        && find_position.pool_id == pool_config.pool_id_account.unwrap()
    {
        // personal position exist
        let mut remaining_accounts = Vec::new();
        remaining_accounts.push(AccountMeta::new_readonly(
            pool_config.tickarray_bitmap_extension.unwrap(),
            false,
        ));

        let increase_instr = increase_liquidity_instr(
            &pool_config.clone(),
            pool_config.pool_id_account.unwrap(),
            pool.token_vault_0,
            pool.token_vault_1,
            pool.token_mint_0,
            pool.token_mint_1,
            find_position.nft_mint,
            spl_associated_token_account::get_associated_token_address(
                &payer.pubkey(),
                &pool_config.mint0.unwrap(),
            ),
            spl_associated_token_account::get_associated_token_address(
                &payer.pubkey(),
                &pool_config.mint1.unwrap(),
            ),
            remaining_accounts,
            liquidity,
            amount_0_max,
            amount_1_max,
            tick_lower_index,
            tick_upper_index,
            tick_array_lower_start_index,
            tick_array_upper_start_index,
        )?;
        // send
        let signers = vec![&payer];
        let recent_hash = rpc_client.get_latest_blockhash()?;
        let txn = Transaction::new_signed_with_payer(
            &increase_instr,
            Some(&payer.pubkey()),
            &signers,
            recent_hash,
        );
        let signature = send_txn(&rpc_client, &txn, true)?;
        println!("{}", signature);
    } else {
        // personal position not exist
        println!("personal position exist:{:?}", find_position);
    }

    Ok(())
}
