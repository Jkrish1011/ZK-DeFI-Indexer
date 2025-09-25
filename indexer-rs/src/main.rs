use alloy::{
    contract::{ContractInstance, Interface},
    dyn_abi::DynSolValue,
    network::{EthereumWallet, TransactionBuilder, NetworkWallet},
    providers::{Provider, ProviderBuilder, WsConnect},
    primitives::{address, U256, hex, B256,Log as ETHLog, LogData, FixedBytes, Address},
    rpc::types::{Filter,Log, TransactionRequest, BlockNumberOrTag},
    signers::local::LocalSigner,
    sol
};

use std::str::from_utf8;
use alloy::sol_types::SolEvent;
use tokio::task::JoinHandle;
use std::panic;
use std::future::Future;
use chrono::format::Fixed;
use hex as justHex;
use std::fs::read_to_string;
use rand::thread_rng;
use std::path::PathBuf;
use eyre::Result;
use futures_util::StreamExt;
use dotenv::dotenv;
use std::env;
use std::str::FromStr;
use std::{collections::HashSet, sync::Arc, time::Duration};
use tokio::{sync::RwLock, time};
use serde::{Deserialize, Serialize};

sol! {
    #[allow(missing_docs)]
    #[sol(rpc)]
    ARBITRUM,
    "src/abi/ARBITRUM.json"
}


#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    let alchemy_url = env::var("ETHEREUM_MAINNET_WSS_URL")
        .expect("ETHEREUM_MAINNET_WSS_URL must be set in .env");
    let arbitrum_contract = env::var("ARBITRUM_CONTRACT_ADDRESS")
        .expect("ARBITRUM_CONTRACT_ADDRESS must be set in .env");
    println!("Connecting to Alchemy: {alchemy_url:?}");

    // Create WebSocket connection
    let ws = WsConnect::new(alchemy_url);
    
    // Create provider with WebSocket transport
    let provider = ProviderBuilder::new()
        .on_ws(ws)
        .await?;

    println!("Connected! Subscribing to new blocks...");

    // Parse the string (expects a 0x-prefixed hex address)
    let arbitrum_address: Address = arbitrum_contract
        .parse()
        .expect("ARBITRUM_CONTRACT_ADDRESS must be a valid 0x-prefixed address");

    let filter = Filter::new()
        // By NOT specifying an `event` or `event_signature` we listen to ALL events of the
        // contract.
        .address(arbitrum_address)
        .from_block(BlockNumberOrTag::Latest);

    // Subscribe to logs.
    let sub = provider.subscribe_logs(&filter).await?;
    let mut stream = sub.into_stream();

    while let Some(log) = stream.next().await {
        // Match on topic 0, the hash of the signature of the event.
        println!("Received log: {:?}", log);
    }


    // // Subscribe to block headers.
    // let subscription = provider.subscribe_blocks().await?;
    // let mut stream = subscription.into_stream().take(2);

    // while let Some(header) = stream.next().await {
    //     println!("Received block number: {}", header.number);
    // }

    // Subscribe to new block headers
    // let sub = provider.subscribe_blocks().await?;
    // let mut stream = sub.into_stream();

    // // Process incoming blocks
    // while let Some(block_header) = stream.next().await {
    //     println!("\n🔗 New Block: #{}", block_header.number.unwrap_or(0));
        
    //     // Get full block with transactions
    //     if let Some(full_block) = provider
    //         .get_block(true.into())
    //         .await? 
    //     {
    //         println!("Transactions in block: {}", full_block.transactions.len());
            
    //         // Filter for potential Uniswap/Arbitrum transactions
    //         for tx in &full_block.transactions {
    //             if let Some(to_address) = &tx.to {
    //                 // Add your Uniswap/Arbitrum contract addresses here
    //                 let uniswap_v3_router = arbitrum_contract;
                    
    //                 if format!("{:?}", to_address).to_lowercase() == uniswap_v3_router.to_lowercase() {
    //                     println!("  📊 Potential Uniswap transaction: {:?}", tx.hash);
    //                     // Add your transaction analysis logic here
    //                 }
    //             }
    //         }
    //     }
    // }

    Ok(())
}