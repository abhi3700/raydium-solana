mod client;
mod instructions;
mod transactions;
mod utils;

use anchor_client::Client;
use anchor_client::Cluster;
use anyhow::anyhow;
use anyhow::Result;
use ini::Ini;
use raydium_amm_v3::states::POOL_TICK_ARRAY_BITMAP_SEED;
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signer::keypair::read_keypair_file;
use std::str::FromStr;
use std::sync::Arc;
use std::time::SystemTime;
use transactions::create_pool_tx;
use transactions::increase_liquidity_tx;

#[derive(Clone, Debug, PartialEq)]
pub struct ClientConfig {
    http_url: String,
    ws_url: String,
    payer_path: String,
    admin_path: String,
    raydium_v3_program: Pubkey,
    slippage: f64,
    amm_config_key: Pubkey,
    mint0: Option<Pubkey>,
    mint1: Option<Pubkey>,
    pool_id_account: Option<Pubkey>,
    tickarray_bitmap_extension: Option<Pubkey>,
    amm_config_index: u16,
}

fn load_cfg(client_config: &String) -> Result<ClientConfig> {
    let config =
        Ini::load_from_file(client_config).expect("Expect 'config.ini' file at the project root");
    // let _map = config.load(client_config).unwrap();
    let global_section = config
        .section(Some("Global"))
        .expect("Didn't define Global section");
    // WARN: Just replace unwrap with expect.
    let http_url = global_section.get("http_url").unwrap().to_string();
    if http_url.is_empty() {
        panic!("http_url must not be empty");
    }
    let ws_url = global_section.get("ws_url").unwrap().to_string();
    if ws_url.is_empty() {
        panic!("ws_url must not be empty");
    }
    let payer_path = global_section.get("payer_path").unwrap().to_string();
    if payer_path.is_empty() {
        panic!("payer_path must not be empty");
    }
    let admin_path = global_section.get("admin_path").unwrap().to_string();
    if admin_path.is_empty() {
        panic!("admin_path must not be empty");
    }

    let raydium_v3_program_str = global_section.get("raydium_v3_program").unwrap();
    if raydium_v3_program_str.is_empty() {
        panic!("raydium_v3_program must not be empty");
    }
    let raydium_v3_program = Pubkey::from_str(&raydium_v3_program_str).unwrap();
    let slippage = global_section.get("slippage").unwrap().parse::<f64>()?;

    let pool_section = config
        .section(Some("Pool"))
        .expect("Didn't define Pool section");
    let mut mint0 = None;
    let mint0_str = pool_section.get("mint1").unwrap();
    if !mint0_str.is_empty() {
        mint0 = Some(Pubkey::from_str(&mint0_str).unwrap());
    }
    let mut mint1 = None;
    let mint1_str = pool_section.get("mint1").unwrap();
    if !mint1_str.is_empty() {
        mint1 = Some(Pubkey::from_str(&mint1_str).unwrap());
    }
    let amm_config_index = pool_section
        .get("amm_config_index")
        .unwrap()
        .parse::<u16>()?;

    let (amm_config_key, __bump) = Pubkey::find_program_address(
        &[
            raydium_amm_v3::states::AMM_CONFIG_SEED.as_bytes(),
            &amm_config_index.to_be_bytes(),
        ],
        &raydium_v3_program,
    );

    let pool_id_account = if mint0 != None && mint1 != None {
        if mint0.unwrap() > mint1.unwrap() {
            let temp_mint = mint0;
            mint0 = mint1;
            mint1 = temp_mint;
        }
        Some(
            Pubkey::find_program_address(
                &[
                    raydium_amm_v3::states::POOL_SEED.as_bytes(),
                    amm_config_key.to_bytes().as_ref(),
                    mint0.unwrap().to_bytes().as_ref(),
                    mint1.unwrap().to_bytes().as_ref(),
                ],
                &raydium_v3_program,
            )
            .0,
        )
    } else {
        None
    };
    let tickarray_bitmap_extension = if pool_id_account != None {
        Some(
            Pubkey::find_program_address(
                &[
                    POOL_TICK_ARRAY_BITMAP_SEED.as_bytes(),
                    pool_id_account.unwrap().to_bytes().as_ref(),
                ],
                &raydium_v3_program,
            )
            .0,
        )
    } else {
        None
    };

    Ok(ClientConfig {
        http_url,
        ws_url,
        payer_path,
        admin_path,
        raydium_v3_program,
        slippage,
        amm_config_key,
        mint0,
        mint1,
        pool_id_account,
        tickarray_bitmap_extension,
        amm_config_index,
    })
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("\"Play with RaydiumV3 AMM\"");
    let config_file = &"config.ini".to_string();

    let pool_config = load_cfg(&config_file).unwrap();
    // Admin and cluster params.
    let payer = read_keypair_file(&*shellexpand::tilde(&pool_config.payer_path))
        .map_err(|_| anyhow!("failed in getting payer"))?;
    let payer = Arc::new(payer);
    let admin = read_keypair_file(&*shellexpand::tilde(&pool_config.admin_path))
        .map_err(|_| anyhow!("failed in getting admin"))?;
    // solana rpc client
    let rpc_client = RpcClient::new(pool_config.http_url.to_string());

    // anchor client.
    let anchor_config = pool_config.clone();
    let url = Cluster::Custom(anchor_config.http_url, anchor_config.ws_url);
    let wallet = payer.clone();
    let anchor_client = Client::new(url, wallet);
    let ray_program = anchor_client.program(pool_config.raydium_v3_program)?;

    /* Call create_pool externally */
    let config_index = 1;
    let price = 1.5;
    let mint0 = pool_config
        .mint0
        .ok_or_else(|| anyhow!("Invalid mint0 pubkey"))?;
    let mint1 = pool_config
        .mint0
        .ok_or_else(|| anyhow!("Invalid mint1 pubkey"))?;
    if true {
        create_pool_tx(
            &rpc_client,
            &pool_config,
            config_index,
            &payer,
            price,
            mint0,
            mint1,
            {
                let now = SystemTime::now();
                now.duration_since(std::time::UNIX_EPOCH)?.as_secs()
            },
        )?;
    }

    let payer = read_keypair_file(&*shellexpand::tilde(&pool_config.payer_path))
        .map_err(|_| anyhow!("failed in getting payer"))?;

    // for price 1.5, kept these tick values.
    // `$ target/debug/client price-to-tick 1.5`
    // price:1.5, tick:4054
    let tick_lower_price = 4001.0;
    let tick_upper_price = 4101.0;
    let is_base_0 = true;
    let input_amount = 100;
    increase_liquidity_tx(
        ray_program,
        rpc_client,
        pool_config,
        payer,
        tick_lower_price,
        tick_upper_price,
        is_base_0,
        input_amount,
    )
    .await?;

    Ok(())
}
