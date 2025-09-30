use alloy::{
    contract::{ContractInstance, Interface},
    dyn_abi::DynSolValue,
    network::{EthereumWallet, TransactionBuilder, NetworkWallet},
    providers::{Provider, ProviderBuilder, WsConnect},
    primitives::{address, U256, hex, B256,Log as ETHLog, LogData, FixedBytes, Address, Bytes},
    rpc::types::{Filter, Log, TransactionRequest, BlockNumberOrTag},
    signers::local::LocalSigner,
    sol,
    sol_types::SolEvent,
    rlp::Decodable,
    consensus::{
        EthereumTxEnvelope,
        TypedTransaction,
        Transaction,
        transaction::{TxEip4844Variant, SignerRecoverable},
    },
};

use std::{
    str::{from_utf8,FromStr},
    panic,
    future::Future,
    fs::read_to_string,
    path::Path,
    env,
    collections::HashSet, 
    sync::Arc, 
    time::Duration,
    io::Read,
};

use eyre::Result;
use serde::{Deserialize, Serialize};
use reqwest::{Error, Client};
use brotli::Decompressor;

