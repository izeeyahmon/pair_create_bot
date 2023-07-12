use ethers::{
    core::types::{Address, Filter},
    prelude::*,
    providers::{Http, Provider},
    utils::format_units,
};
use eyre::Result;
use std::sync::Arc;
use std::{thread, time::Duration};
mod addwebhook;
use addwebhook::send_webhook;
abigen!(
    UniswapFactory,
    r#"[
    event PairCreated(address indexed token0, address indexed token1,address indexed pair, uint)
    ]"#,
);

abigen!(
    ERC20,
    r#"[
    function name() public view virtual returns (string)
    function symbol() public view virtual returns (string)
    ]"#,
);
abigen!(
    UniswapPair,
    r#"[
    function getReserves() external view returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast)
    ]"#,
);
const HTTP_URL: &str = "http://10.234.32.252:8545";
const UNISWAP_FACTORY: &str = "0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f";
const WETH_ADDRESS: &str = "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2";
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    loop {
        let provider = Provider::<Http>::try_from(HTTP_URL).unwrap();
        let provider = Arc::new(provider);
        let block_number: U64 = provider.get_block_number().await.unwrap();
        let address: Address = UNISWAP_FACTORY.parse().unwrap();
        let filter = Filter::new()
            .address(address)
            .event("PairCreated(address,address,address,uint256)")
            .from_block(block_number);
        let logs = provider.get_logs(&filter).await.unwrap();
        println!("{:?}", block_number);
        for log in logs.iter() {
            let token0 = Address::from(log.topics[1]);
            let token1 = Address::from(log.topics[2]);
            let weth_address: Address = WETH_ADDRESS.parse().unwrap();
            let pairadd = Address::from(&log.data[12..32].try_into()?);
            let pair_contract = UniswapPair::new(pairadd, provider.clone());
            if token0 == weth_address {
                let liq = pair_contract.get_reserves().await.unwrap();
                let liq_0 = format_units(U256::from(liq.0), "ether").unwrap();
                let token1_contract = ERC20::new(token1, provider.clone());
                let token1_symbol: String = token1_contract.symbol().await.unwrap();
                println!(
                    "New Pair Detected {:?}/WETH at {:?} with {:?}ETH pooled ETH",
                    &token1_symbol, pairadd, liq_0
                );
                if send_webhook(token1_symbol.clone(), pairadd, liq_0.clone())
                    .await
                    .is_ok()
                {
                    println!("Webhook sent");
                }
            } else {
                let liq = pair_contract.get_reserves().await.unwrap();
                let liq_1 = format_units(U256::from(liq.1), "ether").unwrap();
                let token0_contract = ERC20::new(token0, provider.clone());
                let token0_symbol: String = token0_contract.symbol().await.unwrap();
                println!(
                    "New Pair Detected {:?}/WETH at {:?} with {:?}ETH pooled ETH",
                    &token0_symbol, pairadd, liq_1
                );
                send_webhook(token0_symbol.clone(), pairadd, liq_1.clone())
                    .await
                    .unwrap();
            }
        }
        thread::sleep(Duration::from_secs(8));
    }
}
