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
    rlp::{Decodable, Encodable},
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

use tokio::{
    task::JoinHandle,
    sync::RwLock, 
    time,
};

use c_kzg::{
    KzgSettings,
    Blob,
    KzgCommitment,
};

use chrono::format::Fixed;
use hex as justHex;
use rand::thread_rng;
use eyre::Result;
use futures_util::StreamExt;
use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use reqwest::{Error, Client};
use brotli::Decompressor;

mod utils;

use utils::helpers::save_bytes_to_file;

sol! {
    #[allow(missing_docs)]
    #[sol(rpc)]
    ARBITRUM,
    "src/abi/ARBITRUM.json"
}

sol! {
    #[derive(Debug)]
    struct TimeBounds {
        uint64 minTimestamp;
        uint64 maxTimestamp;
        uint64 minBlockNumber;
        uint64 maxBlockNumber;
    }

    #[derive(Debug)]
    enum BatchDataLocation {
        /// @notice The data can be found in the transaction call data
        TxInput,
        /// @notice The data can be found in an event emitted during the transaction
        SeparateBatchEvent,
        /// @notice This batch contains no data
        NoData,
        /// @notice The data can be found in the 4844 data blobs on this transaction
        Blob
    }

    #[derive(Debug)]
    event SequencerBatchDelivered(
        uint256 indexed batchSequenceNumber,
        bytes32 indexed beforeAcc,
        bytes32 indexed afterAcc,
        bytes32 delayedAcc,
        uint256 afterDelayedMessagesRead,
        TimeBounds timeBounds,
        BatchDataLocation dataLocation
    );
}

fn calculate_slot_number(blockNumber: u64) -> u64 {
    // const slot = Math.floor((timestamp - 1663224000) / 12) + 4700013;
    let slot = (blockNumber - 15537394) + 4700013;
    println!("slot: {}", &slot.to_string());
    slot
}

fn commitment_to_hex(commitment: &KzgCommitment) -> String {
    format!("0x{}", justHex::encode(commitment.as_ref()))
}

fn compute_kzg_commitment(blob: &[u8]) -> Option<String> {
    println!("Attempting KZG commitment computation on blob of size: {}", blob.len());
    let setup_path = "trusted_setup.txt";

    if !Path::new(setup_path).exists() {
        println!("ERROR: trusted_setup.txt doesn't exists!");
        return None;
    }

    let kzg_settings = match KzgSettings::load_trusted_setup_file(&Path::new(setup_path), 0) {
        Ok(settings) => settings,
        Err(e) => {
            println!("ERROR:Failed to load trusted setup: {:?}", e);
            return None;
        }
    };

    // Prepare blob data - KZG expects exactly 131072 bytes (4096 field elements * 32 bytes each)
    let required_size = 4096 * 32; // 131072 bytes
    let blob_data = if blob.len() == required_size {
        println!("Blob is already the correct size: {} bytes", required_size);
        blob.to_vec()
    } else if blob.len() < required_size {
        println!("Blob is too small ({}), padding to {} bytes", blob.len(), required_size);
        let mut padded = blob.to_vec();
        padded.resize(required_size, 0);
        padded
    } else {
        println!("Blob is too large ({}), truncating to {} bytes", blob.len(), required_size);
        blob[..required_size].to_vec()
    };

    let blob_converted = match Blob::from_bytes(&blob_data) {
        Ok(blob) => blob,
        Err(e) => {
            println!("ERROR:Failed to convert blob to KZG blob: {:?}", e);
            return None;
        }
    };

    let commitment: Option<String> = match kzg_settings.blob_to_kzg_commitment(&blob_converted) {
        Ok(commitment) => {
            let commitment_hex = commitment_to_hex(&commitment);
            // println!("commitment: {}", &commitment_hex);
            Some(commitment_hex)
        },
        Err(e) => {
            println!("ERROR:Failed to compute KZG commitment: {:?}", e);
            None
        }
    };
    println!("KZG commitment computed successfully");
    commitment
}

// 5) Brotli with fallback to brotli2 (more robust)
fn try_brotli_decompress(data: &[u8]) -> Option<Vec<u8>> {
    // First try brotli crate with a large buffer
    {
        let mut decompressed = Vec::new();
        let mut reader = brotli::Decompressor::new(data, 1 << 20);
        if std::io::Read::read_to_end(&mut reader, &mut decompressed).is_ok() {
            return Some(decompressed);
        }
    }
    // Fallback: brotli2
    {
        use brotli2::bufread::BrotliDecoder;
        let cursor = std::io::Cursor::new(data);
        let mut decoder = BrotliDecoder::new(cursor);
        let mut decompressed = Vec::new();
        if std::io::Read::read_to_end(&mut decoder, &mut decompressed).is_ok() {
            return Some(decompressed);
        }
    }
    None
}

// ---- RLP helpers: compute total length of the next RLP item and peel a stream ----
fn rlp_item_total_len(input: &[u8]) -> Option<usize> {
    if input.is_empty() {
        return None;
    }
    let b0 = input[0];

    match b0 {
        0x00..=0x7f => Some(1), // single byte
        0x80..=0xb7 => {
            let len = (b0 - 0x80) as usize;
            if input.len() < 1 + len { return None; }
            Some(1 + len)
        }
        0xb8..=0xbf => {
            let len_of_len = (b0 - 0xb7) as usize;
            if input.len() < 1 + len_of_len { return None; }
            let mut l: usize = 0;
            for &byte in &input[1..1 + len_of_len] {
                l = (l << 8) | (byte as usize);
            }
            if input.len() < 1 + len_of_len + l { return None; }
            Some(1 + len_of_len + l)
        }
        0xc0..=0xf7 => {
            let payload_len = (b0 - 0xc0) as usize;
            if input.len() < 1 + payload_len { return None; }
            Some(1 + payload_len)
        }
        0xf8..=0xff => {
            let len_of_len = (b0 - 0xf7) as usize;
            if input.len() < 1 + len_of_len { return None; }
            let mut l: usize = 0;
            for &byte in &input[1..1 + len_of_len] {
                l = (l << 8) | (byte as usize);
            }
            if input.len() < 1 + len_of_len + l { return None; }
            Some(1 + len_of_len + l)
        }
    }
}

// Peel the next RLP item as a byte-string, returning (payload_bytes, total_consumed)
// Returns None if the input is malformed or the item is an RLP list (not a string/bytes).
fn rlp_peel_string(input: &[u8]) -> Option<(Vec<u8>, usize)> {
    if input.is_empty() { return None; }
    let b0 = input[0];
    match b0 {
        0x00..=0x7f => {
            // single byte, payload is that byte
            Some((vec![b0], 1))
        }
        0x80..=0xb7 => {
            let len = (b0 - 0x80) as usize;
            if input.len() < 1 + len { return None; }
            let payload = input[1..1+len].to_vec();
            Some((payload, 1 + len))
        }
        0xb8..=0xbf => {
            let len_of_len = (b0 - 0xb7) as usize;
            if input.len() < 1 + len_of_len { return None; }
            let mut l: usize = 0;
            for &byte in &input[1..1 + len_of_len] {
                l = (l << 8) | (byte as usize);
            }
            if input.len() < 1 + len_of_len + l { return None; }
            let start = 1 + len_of_len;
            let payload = input[start..start + l].to_vec();
            Some((payload, 1 + len_of_len + l))
        }
        0xc0..=0xff => {
            // It's a list; Nitro segments are expected to be byte-strings.
            None
        }
    }
}

// Try to decode a concatenated stream of RLP items, returning each item's byte-string payload.
// If any item is not a byte-string or parsing fails, return None.
fn try_decode_rlp_segments(input: &[u8]) -> Option<Vec<Vec<u8>>> {
    let mut out: Vec<Vec<u8>> = Vec::new();
    let mut cursor = input;
    while !cursor.is_empty() {
        let (payload, consumed) = match rlp_peel_string(cursor) {
            Some(v) => v,
            None => return None,
        };
        out.push(payload);
        cursor = &cursor[consumed..];
    }
    if out.is_empty() { None } else { Some(out) }
}


// Decode a single raw tx into alloy's TxEnvelope
// Fixed: Using TxEip4844Variant as the generic parameter
fn decode_tx(raw: &[u8]) -> Result<EthereumTxEnvelope<TxEip4844Variant>> {

    if raw.first().copied() != Some(0x02) {
        eyre::bail!("decode_tx called with non EIP 4844 payload (first byte = {:#x?})", raw.first());
    }
    let mut slice = raw;
    let tx = EthereumTxEnvelope::decode(&mut slice)?;
    Ok(tx)
}

/// Parse the *raw‑batch* format used by Arbitrum sequencer inboxes.
///
/// Returns a vector where each element is the **raw bytes of a single transaction**.
pub fn decode_raw_batch(data: &[u8]) -> Result<Vec<Vec<u8>>> {
    // Strip the 4-byte version field (big-endian, we ignore it
    if data.len() < 4 {
        println!("payload too short for version field!");
        return Ok(Vec::new());
    }

    let _version = u32::from_be_bytes(data[0..4].try_into().unwrap());
    let mut cursor = &data[4..];

    // Read transaction count (little-endian u32)
    if cursor.len() < 4 {
        println!("payload too short for transaction count!");
        return Ok(Vec::new());
    }

    let tx_count = u32::from_le_bytes(cursor[0..4].try_into().unwrap()) as usize;
    println!("Raw-batch header reports {} transactions", tx_count);
    cursor = &cursor[4..];

    // Sanity‑check: the reported size should not exceed the remaining buffer.
    if tx_count > cursor.len() {
        println!("Warning: reported size ({}) > actual remaining ({}) will decode until buffer ends", tx_count, cursor.len());
    }

    // Repeatedly parse and decode RLP-encoded transaction.
    let mut txs = Vec::new();
    let mut remaining = cursor;

    while !remaining.is_empty() {
        // The first byte of a valid 4844 tx must be 0x02.
        // If it isn’t, we abort – this protects us from mis‑aligned data.
        if remaining[0] != 0x02 {
            eyre::bail!(
                "expected transaction type 0x02 but found {:#x} at offset {}",
                remaining[0],
                data.len() - remaining.len()
            );
        }

        // `decode_tx` will consume exactly one transaction and return the envelope.
        // We clone the slice first so we can keep the raw bytes for later use.
        let raw_tx = remaining.to_vec(); // full remaining buffer
        let envelope = decode_tx(&raw_tx)?;

        // Determine how many bytes were consumed by the envelope.
        // `EthereumTxEnvelope::decode` consumes from the slice we give it,
        // leaving the remainder behind. We can reuse that logic here:
        let mut slice = &raw_tx[..];
        let _ = EthereumTxEnvelope::<TxEip4844Variant>::decode(&mut slice)?;
        let consumed = raw_tx.len() - slice.len();

        // Store the exact raw transaction bytes.
        txs.push(raw_tx[..consumed].to_vec());

        // Advance the cursor.
        remaining = &remaining[consumed..];
    }

    // the number of extracted txs should match the header.
    if txs.len() != tx_count {
        println!("Warning: header claimed {} txs but we extracted {}", tx_count, txs.len());
    }
    Ok(txs)
}

// Extract Core Fields from a TxEnvelope
// Fixed: Using TxEip4844Variant as the generic parameter
fn extract_core_fields(tx: &EthereumTxEnvelope<TxEip4844Variant>) -> eyre::Result<()> {
    let from: Address = tx.recover_signer()?;
    
    // Analyze EIP-1559. Might vary.
    let to: Option<Address> = tx.to();
    let value: U256 = tx.value();
    let data: Bytes = tx.input().clone();
    let nonce: u64 = tx.nonce();

    println!("from: {from:?}");
    println!("to: {to:?}");
    println!("value: {value}");
    println!("nonce: {nonce}");
    println!("calldata (first 16 bytes): 0x{}", hex::encode(&data[..16.min(data.len())]));

    Ok(())
}

// Returns the bytes that contains the transaction list
/// * If the blob starts with the 4‑byte version + 4‑byte little‑endian size
///   (the standard 4844 header), the function slices out the payload
///   described by that size.
/// * If the blob is shorter than 8 bytes or the size field is bogus,
///   an error is returned.
/// * The returned `Vec<u8>` may still start with a Nitro flag (`0x0a`/`0x0b`);
///   the caller decides what to do with it.
fn extract_payload_from_4844_blob(blob: &[u8]) -> Result<Vec<u8>> {
    // if length is < 8, treat the whole as the paylod
    if blob.len() < 8 {
        println!("4844 blob < 8 bytes, returning whole buffer");
        return Ok(blob.to_vec());
    }

    //4 - byte version (big-endian) - not under consideration, just read it
    let version_be = u32::from_be_bytes(blob[0..4].try_into().unwrap());

    //4 - byte size (little-endian) length of the following data
    let size_le = u32::from_le_bytes(blob[4..8].try_into().unwrap());

    println!("4844 header: version(be)={:#x}, size(le)={}", version_be, size_le);

    let end = 8u64.saturating_add(size_le as u64) as usize;

    // check malformed size fields
    if size_le == 0 || end > blob.len() {
        println!("4844 header size invalid (size={}, end={}, blob={}), using whole unpacked buffer", size_le, end, blob.len());
        // Size field looks wrong - assume it's the whole blob
        return Ok(blob.to_vec());
    }

    Ok(blob[8..end].to_vec())
}

fn decode_nitro_batch(batch: &[u8]) -> eyre::Result<()> {
    if let Some(segments) = try_decode_rlp_segments(batch) {
        println!("Decoded {} segments from Nitro batch", segments.len());
        handle_segments(segments)?;
        Ok(())
    } else {
        eyre::bail!("Failed to decode RLP segments from Nitro batch");
    }
}

fn handle_segments(segments: Vec<Vec<u8>>) -> eyre::Result<()> {
    for (i, seg) in segments.iter().enumerate() {
        if seg.is_empty() {
            println!("Segment {} is empty, skipping", i);
            continue;
        }
        let kind = seg[0];
        let payload = &seg[1..];

        match kind {
            0 => {
                // L2 message
                decode_l2_message(payload)?;
            }
            1 => {
                // L2 message, brotli-compressed
                if let Some(decompressed) = try_brotli_decompress(payload) {
                    decode_l2_message(&decompressed)?;
                } else {
                    println!("Segment {}: failed to brotli-decompress L2 message", i);
                }
            }
            2 => {
                // delayed messages pointer; optional to resolve
                println!("Segment {}: delayed messages pointer (ignored for now)", i);
            }
            other => {
                println!("Segment {}: unknown kind {}, raw {}", i, other, hex::encode(seg));
            }
        }
    }
    Ok(())
}

fn decode_l2_message(msg: &[u8]) -> eyre::Result<()> {
    if msg.is_empty() {
        eyre::bail!("Empty L2 message");
    }
    let kind = msg[0];
    let payload = &msg[1..];

    match kind {
        0x04 => {
            println!("L2 message kind=SignedTx (0x04), len={}", payload.len());
            // Do not force EIP-1559; try a general decoder.
            // Option A: Attempt alloy's general envelope decoding (legacy/2930/1559).
            // If not readily available, start by logging and saving the tx for offline decode.
            // Example placeholder:
            // match decode_eth_tx_envelope(payload) { ... }

            // For now, just log the first bytes and optionally persist:
            println!("SignedTx first 64: {}", hex::encode(&payload[..64.min(payload.len())]));
            // save_raw_bytes_to_file("l2_signed_tx.bin", payload.to_vec())?;
        }
        0x03 => {
            println!("L2 message kind=Batch (0x03), nested frames");
            let mut cursor = payload;
            let mut idx = 0usize;
            while cursor.len() >= 8 {
                let len = u64::from_be_bytes(cursor[0..8].try_into().unwrap()) as usize;
                cursor = &cursor[8..];
                if cursor.len() < len {
                    eyre::bail!("Frame length exceeds remaining buffer");
                }
                let frame = &cursor[..len];
                cursor = &cursor[len..];

                println!("Nested frame {}: {} bytes", idx, len);
                decode_l2_message(frame)?;
                idx += 1;
            }
            if !cursor.is_empty() {
                println!("Trailing {} bytes after frames (ignored)", cursor.len());
            }
        }
        0x09 => {
            println!("L2 message kind=synthetic delayed (0x09); ignoring");
        }
        other => {
            println!("Unknown L2 message kind=0x{:02x}, payload first 32 {}", other, hex::encode(&payload[..32.min(payload.len())]));
        }
    }
    Ok(())
}

// Referencing to : https://github.com/OffchainLabs/nitro/blob/master/arbstate/inbox.go#L68-L113:
// https://github.com/OffchainLabs/nitro/blob/master/daprovider/daclient/daclient.go
// 7) Main entry for a single blob
// Pass in the event TimeBounds so we can auto-detect endianness of the 40-byte header
pub fn handle_raw_blob(
    raw_blob: &[u8],
    ev_min_ts: u64,
    ev_max_ts: u64,
    ev_min_bn: u64,
    ev_max_bn: u64,
) -> eyre::Result<()> {
    println!("\n=== Processing new blob ===");
    println!("Raw blob size: {} bytes", raw_blob.len());

    // Unpack to 126,976 bytes
    let unpacked = decode_4844_blob(raw_blob)?;
    println!(
        "Unpacked payload size: {}",
        unpacked.len()
    );
    println!(
        "Unpacked first 64 bytes: {}",
        justHex::encode(&unpacked[..64.min(unpacked.len())])
    );

    // No 8-byte version+size header for your feed; work on unpacked directly
    let payload = &unpacked;

    // Parse the 40-byte header with endian autodetection
    let (header, batch_data) =
        parse_header_autodetect(payload, ev_min_ts, ev_max_ts, ev_min_bn, ev_max_bn)?;
    println!("Header: {:?}", header);

    if batch_data.is_empty() {
        println!("Empty batch_data after header; skipping blob");
        return Ok(());
    }

    let flag = batch_data[0];
    println!("Flag byte after header: 0x{:02x}", flag);

    // AnyTrust/DAS
    if (flag & 0x80) != 0 {
        if batch_data.len() < 65 {
            println!("DAS header too short (<65 bytes); skipping blob");
            return Ok(());
        }
        let keyset_hash = &batch_data[1..33];
        let data_hash = &batch_data[33..65];
        println!("DAS cert: keyset_hash=0x{}", justHex::encode(keyset_hash));
        println!("DAS cert: data_hash  =0x{}", justHex::encode(data_hash));
        println!("DAC fetch required; stopping for this blob.");
        return Ok(());
    }

    // Whole-batch compressed
    if flag == 0x00 {
        let compressed = &batch_data[1..];
        if let Some(decompressed) = try_brotli_decompress(compressed) {
            println!(
                "Decompressed whole-batch: {} -> {} bytes",
                compressed.len(),
                decompressed.len()
            );
            // Try RLP segments first (some stacks encode segments as an RLP string list)
            if let Some(segs) = try_decode_rlp_segments(&decompressed) {
                println!("Decoded {} RLP segments (whole-batch)", segs.len());
                handle_segments(segs)?;
                return Ok(());
            }
            // Otherwise, try to treat the decompressed bytes as nested segment stream
            if let Err(e) = parse_top_level_segments(&decompressed) {
                println!("Nested segment stream failed after brotli: {e}");
            }
            return Ok(());
        } else {
            println!("Brotli decompression failed for whole-batch; skipping blob");
            return Ok(());
        }
    }

    // Non-DAS, non-whole-batch: parse a top-level segment stream
    if let Err(e) = parse_top_level_segments(batch_data) {
        println!("Top-level segment stream failed: {e}");
    }

    Ok(())
}

/// Parse a Nitro batch payload.  
/// Works for:
///   * flagged batches (`0x0a` plain, `0x0b` compressed)  
///   * flag‑less batches (some blobs from Blobscan)
/*
    When the first byte is not 0x0a/0x0b the data you receive is not a Nitro‑batch‑encoded list of (len || tx).
    Arbitrum stores those batches in a different layout that looks like

    +-------------------+-------------------+-------------------+
    | 4‑byte version   | 4‑byte tx‑count   | tx₀ … txₙ          |
    +-------------------+-------------------+-------------------+
*/
fn parse_nitro_batch(payload: &[u8]) -> eyre::Result<Vec<Vec<u8>>> {
    println!("payload size: {}", payload.len());
    
    // Detects the optional flag byte (0x0a or 0x0b – the two Nitro batch types).
    // Strip the optional batch‑type flag (0x0a = normal, 0x0b = compressed)
    let (mut data, start_offset) =  match payload.first() {
        Some(0x0a) => {
            println!("Detected nitro Flag (plain batch)");
            (payload[1..].to_vec(), 0usize)
        }
        Some(0x0b) => {
            println!("Detected nitro Flag (compressed batch)");
            (payload[1..].to_vec(), 0usize)
        }
        _ => {
            println!("No nitro flag detected");
            (payload.to_vec(), 0usize)
        }
    };


    // If the flag was 0x0b, the inner data is Brotli‑compressed.

    if let Some(&0x0b) = payload.first() {
        if let Some(decompressed) = try_brotli_decompress(&data) {
            println!(
                "Decompressed Brotli payload: {} → {} bytes",
                data.len(),
                decompressed.len()
            );
            data = decompressed;
        } else {
            println!("Brotli decompression failed! treating as raw");
        }
    }
    

    // Reading through the remaining bytes, reading little‑endian lengths.
    let mut txs = Vec::new();
    let mut offset = start_offset;

    while offset + 4 <= data.len() {
        // Read 4-byte little-endian length
        let len_bytes = &data[offset..offset + 4];
        let len = u32::from_le_bytes(len_bytes.try_into().unwrap()) as usize;
        offset += 4;

        // Checking if corrupted data or something else is wrong
        if len == 0 {
            println!("Zero-length entry at offset {}, skipping", offset - 4);
            continue;
        }

        // 10MB is far larger so check if we're going to overflow
        if len > 10_000_000 {
            println!("Unreasonable length {} at offset {}, skipping", len, offset - 4);
            break;
        }

        // Check if overflow
        if offset + len > data.len() {
            println!("Length {} would overflow buffer at offset {}, remaining {} bytes, skipping", len, offset, data.len() - offset);
            break;
        }
        
        let tx_bytes = data[offset..offset + len].to_vec();
        println!("Extracted tx #{} - {} bytes", txs.len(), len);
        txs.push(tx_bytes);

        offset += len;
    }

    println!("Finished parsing – {} transaction(s) extracted", txs.len());
    Ok(txs)
}

// 1) Lossless unpack: 4096 * 32 → 4096 * 31 (drop one leading pad byte per element)
fn decode_4844_blob(blob: &[u8]) -> Result<Vec<u8>> {
    const FIELD_ELEMENT_SIZE: usize = 32;
    const NUM_ELEMENTS: usize = 4096;
    const BYTES_PER_ELEMENT: usize = 31;

    if blob.len() != FIELD_ELEMENT_SIZE * NUM_ELEMENTS {
        eyre::bail!("Expected 131072-byte blob, got {}", blob.len());
    }

    let mut output = Vec::with_capacity(NUM_ELEMENTS * BYTES_PER_ELEMENT);
    for i in 0..NUM_ELEMENTS {
        let start = i * FIELD_ELEMENT_SIZE;
        let element = &blob[start..start + FIELD_ELEMENT_SIZE];
        output.extend_from_slice(&element[1..FIELD_ELEMENT_SIZE]);
    }

    println!(
        "Decoded 4844 blob: {} -> {} bytes",
        blob.len(),
        output.len()
    );
    Ok(output)
}


#[derive(Debug)]
struct BatchHeader {
    min_timestamp: u64,
    max_timestamp: u64,
    min_block_number: u64,
    max_block_number: u64,
    after_delayed_count: u64,
}

/// Parse the 40-byte batch header that precedes the actual batch payload
fn parse_batch_header(data: &[u8]) -> Result<(BatchHeader, &[u8])> {
    if data.len() < 40 {
        eyre::bail!("Data too short for batch header: {} bytes", data.len());
    }
    
    let header = BatchHeader {
        min_timestamp: u64::from_be_bytes(data[0..8].try_into().unwrap()),
        max_timestamp: u64::from_be_bytes(data[8..16].try_into().unwrap()),
        min_block_number: u64::from_be_bytes(data[16..24].try_into().unwrap()),
        max_block_number: u64::from_be_bytes(data[24..32].try_into().unwrap()),
        after_delayed_count: u64::from_be_bytes(data[32..40].try_into().unwrap()),
    };
    
    println!("Batch header parsed:");
    println!("  MinTimestamp: {}", header.min_timestamp);
    println!("  MaxTimestamp: {}", header.max_timestamp);
    println!("  MinBlockNumber: {}", header.min_block_number);
    println!("  MaxBlockNumber: {}", header.max_block_number);
    println!("  AfterDelayedCount: {}", header.after_delayed_count);
    
    Ok((header, &data[40..]))
}

// 2) Parse 40-byte Nitro TimeBounds header (BE)
fn parse_batch_header_be(data: &[u8]) -> eyre::Result<(BatchHeader, &[u8])> {
    if data.len() < 40 {
        eyre::bail!("Data too short for batch header: {} bytes", data.len());
    }
    let header = BatchHeader {
        min_timestamp: u64::from_be_bytes(data[0..8].try_into().unwrap()),
        max_timestamp: u64::from_be_bytes(data[8..16].try_into().unwrap()),
        min_block_number: u64::from_be_bytes(data[16..24].try_into().unwrap()),
        max_block_number: u64::from_be_bytes(data[24..32].try_into().unwrap()),
        after_delayed_count: u64::from_be_bytes(data[32..40].try_into().unwrap()),
    };
    Ok((header, &data[40..]))
}

// 3) Parse 40-byte Nitro TimeBounds header (LE)
fn parse_batch_header_le(data: &[u8]) -> eyre::Result<(BatchHeader, &[u8])> {
    if data.len() < 40 {
        eyre::bail!("Data too short for batch header: {} bytes", data.len());
    }
    let header = BatchHeader {
        min_timestamp: u64::from_le_bytes(data[0..8].try_into().unwrap()),
        max_timestamp: u64::from_le_bytes(data[8..16].try_into().unwrap()),
        min_block_number: u64::from_le_bytes(data[16..24].try_into().unwrap()),
        max_block_number: u64::from_le_bytes(data[24..32].try_into().unwrap()),
        after_delayed_count: u64::from_le_bytes(data[32..40].try_into().unwrap()),
    };
    Ok((header, &data[40..]))
}

// 4) Auto-detect endianness by comparing to the event's TimeBounds (pick the closer match)
fn parse_header_autodetect<'a>(
    data: &'a [u8],
    ev_min_ts: u64,
    ev_max_ts: u64,
    ev_min_bn: u64,
    ev_max_bn: u64,
) -> eyre::Result<(BatchHeader, &'a [u8])> {
    let be = parse_batch_header_be(data);
    let le = parse_batch_header_le(data);

    // score closeness to event
    fn score(h: &BatchHeader, ev: (u64, u64, u64, u64)) -> u128 {
        let (mts, xts, mb, xb) = ev;
        (h.min_timestamp as i128 - mts as i128).unsigned_abs() as u128
            + (h.max_timestamp as i128 - xts as i128).unsigned_abs() as u128
            + (h.min_block_number as i128 - mb as i128).unsigned_abs() as u128
            + (h.max_block_number as i128 - xb as i128).unsigned_abs() as u128
    }

    match (be, le) {
        (Ok((hb, rb)), Ok((hl, rl))) => {
            let ev = (ev_min_ts, ev_max_ts, ev_min_bn, ev_max_bn);
            let sb = score(&hb, ev);
            let sl = score(&hl, ev);
            if sl < sb {
                Ok((hl, rl))
            } else {
                Ok((hb, rb))
            }
        }
        (Ok(v), Err(_)) => Ok(v),
        (Err(_), Ok(v)) => Ok(v),
        (Err(e1), Err(_e2)) => Err(e1),
    }
}

// 6) Existing helpers you already have (rlp_peel_string, try_decode_rlp_segments, decode_l2_message, handle_segments) remain unchanged.
// Below is a new top-level segment parser that many Nitro batches use directly after the 40-byte header.

// kind (1 byte) + length (8 bytes, big-endian) + payload (len bytes)
fn plausible_kind(byte: u8) -> bool {
    matches!(byte, 0x00 | 0x01 | 0x02 | 0x03 | 0x04 | 0x09)
}

#[derive(Clone, Copy)]
enum LenCodec {
    U64Be,
    U64Le,
    U32Be,
    U32Le,
}

fn try_take_segment<'a>(mut buf: &'a [u8]) -> Option<((u8, usize), &'a [u8])> {
    // Try each header shape at this exact position
    for codec in [LenCodec::U64Be, LenCodec::U64Le, LenCodec::U32Be, LenCodec::U32Le] {
        if let Some((k, len, rest)) = try_header(buf, codec) {
            return Some(((k, len), rest));
        }
    }
    None
}

fn try_header<'a>(buf: &'a [u8], codec: LenCodec) -> Option<(u8, usize, &'a [u8])> {
    match codec {
        LenCodec::U64Be => {
            if buf.len() < 9 { return None; }
            let k = buf[0];
            let len = u64::from_be_bytes(buf[1..9].try_into().ok()?) as usize;
            if !plausible_kind(k) || len == 0 { return None; }
            let rest = &buf[9..];
            if rest.len() < len { return None; }
            Some((k, len, rest))
        }
        LenCodec::U64Le => {
            if buf.len() < 9 { return None; }
            let k = buf[0];
            let len = u64::from_le_bytes(buf[1..9].try_into().ok()?) as usize;
            if !plausible_kind(k) || len == 0 { return None; }
            let rest = &buf[9..];
            if rest.len() < len { return None; }
            Some((k, len, rest))
        }
        LenCodec::U32Be => {
            if buf.len() < 5 { return None; }
            let k = buf[0];
            let len = u32::from_be_bytes(buf[1..5].try_into().ok()?) as usize;
            if !plausible_kind(k) || len == 0 { return None; }
            let rest = &buf[5..];
            if rest.len() < len { return None; }
            Some((k, len, rest))
        }
        LenCodec::U32Le => {
            if buf.len() < 5 { return None; }
            let k = buf[0];
            let len = u32::from_le_bytes(buf[1..5].try_into().ok()?) as usize;
            if !plausible_kind(k) || len == 0 { return None; }
            let rest = &buf[5..];
            if rest.len() < len { return None; }
            Some((k, len, rest))
        }
    }
}

fn parse_top_level_segments(batch_data: &[u8]) -> eyre::Result<()> {
    if batch_data.is_empty() {
        return Ok(());
    }
    let flag = batch_data[0];
    println!("Top-level segments: flag=0x{:02x}", flag);

    let mut cursor = &batch_data[1..];

    // Keep decoding segments until we exhaust the buffer
    while !cursor.is_empty() {
        // First try at current position
        if let Some(((kind, len), rest_after_header)) = try_take_segment(cursor) {
            // Read segment bytes and advance
            let (seg, rest_after_seg) = rest_after_header.split_at(len);
            match kind {
                0x00 => {
                    println!("Segment kind=0x00 (L2), len={}", len);
                    if let Err(e) = decode_l2_message(seg) {
                        println!("L2 message decode failed: {e}");
                    }
                }
                0x01 => {
                    println!("Segment kind=0x01 (L2 brotli), len={}", len);
                    if let Some(decompressed) = try_brotli_decompress(seg) {
                        if let Err(e) = decode_l2_message(&decompressed) {
                            println!("L2 message (after brotli) decode failed: {e}");
                        }
                    } else {
                        println!("Brotli failed on L2 segment (kind=0x01)");
                    }
                }
                0x02 => {
                    println!("Segment kind=0x02 (delayed pointer), len={}", len);
                }
                other => {
                    println!("Unknown segment kind=0x{:02x}, len={}", other, len);
                }
            }
            cursor = rest_after_seg;
            continue;
        }

        // If no header shape matched, scan forward up to 32 bytes to resync
        let mut advanced = false;
        for skip in 1..=32.min(cursor.len()) {
            if let Some(((kind, len), rest_after_header)) = try_take_segment(&cursor[skip..]) {
                println!("Resynced after skipping {} byte(s)", skip);
                // consume the skipped bytes + header+payload in one go
                let (seg, rest_after_seg) = rest_after_header.split_at(len);
                match kind {
                    0x00 => {
                        println!("Segment kind=0x00 (L2), len={}", len);
                        if let Err(e) = decode_l2_message(seg) {
                            println!("L2 message decode failed: {e}");
                        }
                    }
                    0x01 => {
                        println!("Segment kind=0x01 (L2 brotli), len={}", len);
                        if let Some(decompressed) = try_brotli_decompress(seg) {
                            if let Err(e) = decode_l2_message(&decompressed) {
                                println!("L2 message (after brotli) decode failed: {e}");
                            }
                        } else {
                            println!("Brotli failed on L2 segment (kind=0x01)");
                        }
                    }
                    0x02 => {
                        println!("Segment kind=0x02 (delayed pointer), len={}", len);
                    }
                    other => {
                        println!("Unknown segment kind=0x{:02x}, len={}", other, len);
                    }
                }
                // We consumed: skip + header + len; reconstruct remaining cursor:
                let consumed = skip + (cursor[skip..].len() - rest_after_header.len()) + len;
                cursor = &cursor[consumed..];
                advanced = true;
                break;
            }
        }

        if !advanced {
            println!("Could not find a plausible segment header; stopping this blob.");
            break;
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    let alchemy_url = env::var("ETHEREUM_MAINNET_WSS_URL")
        .expect("ETHEREUM_MAINNET_WSS_URL must be set in .env");
    let arbitrum_contract = env::var("ARBITRUM_CONTRACT_ADDRESS")
        .expect("ARBITRUM_CONTRACT_ADDRESS must be set in .env");
    let arbitrum_sequencer_inbox_contract = env::var("ARBITRUM_SEQUENCER_INBOX_CONTRACT")
        .expect("ARBITRUM_SEQUENCER_INBOX_CONTRACT must be set in .env");

    let blobscan_api = env::var("BLOBSCAN_API")
        .expect("BLOBSCAN_API must be set in .env");

    // Create WebSocket connection
    let ws = WsConnect::new(&alchemy_url);
    
    // Create provider with WebSocket transport
    let provider = ProviderBuilder::new()
        .connect_ws(ws)
        .await?;
    
    println!("Connected! Subscribing to new blocks...");

    // Parse the string (expects a 0x-prefixed hex address)
    let arbitrum_address: Address = arbitrum_contract
        .parse()
        .expect("ARBITRUM_CONTRACT_ADDRESS must be a valid 0x-prefixed address");

    let arbitrum_sequencer_inbox_address: Address = arbitrum_sequencer_inbox_contract
        .parse()
        .expect("ARBITRUM_SEQUENCER_INBOX_CONTRACT must be a valid 0x-prefixed address");

    let filter = Filter::new()
        // By NOT specifying an `event` or `event_signature` we listen to ALL events of the
        // contract.
        .address(arbitrum_sequencer_inbox_address)
        .from_block(BlockNumberOrTag::Latest);

    // Subscribe to logs.
    let sub = provider.subscribe_logs(&filter).await?;
    let mut stream = sub.into_stream();

    while let Some(log) = stream.next().await {
        // Only attempt to decode if topic0 matches the SequencerBatchDelivered signature.
        if let Some(topic0) = log.topics().get(0) {
            if topic0 == &SequencerBatchDelivered::SIGNATURE_HASH {
                match SequencerBatchDelivered::decode_log(&log.inner) {
                    Ok(event) => {
                        println!("Received SequencerBatchDelivered event: {:#?}", &event);
                        println!("event.timeBounds.minBlockNumber: {}", event.timeBounds.minBlockNumber);
                        
                        let blobscanner_api = blobscan_api.clone().replace("BLOCK", event.timeBounds.minBlockNumber.to_string().as_str());
                        let response = reqwest::get(&blobscanner_api).await?;
                        let blob_data: serde_json::Value = response.json().await?;
                        
                        let blobs = blob_data.get("blobs").unwrap();

                        for blob in blobs.as_array().unwrap() {
                            // ---- fetch the raw 4844 blob ---------------
                            let data_storage_ref = blob.get("dataStorageReferences").unwrap();
                            // println!("data_storage_ref: {:#?}", &data_storage_ref);
                            let url = data_storage_ref.get(0).unwrap().get("url").unwrap().as_str().unwrap().to_string();
                            // println!("url: {}", &url);
                            let response = reqwest::get(&url).await?;
                            let raw_blob: Vec<u8> = response.bytes().await?.to_vec();
                            // println!("blob_data: {:#?}", &blob_data[..64]);

                            // println!("First bytes of raw blob : {}", hex::encode(&raw_blob));
                            save_bytes_to_file("raw_blob_1.txt", raw_blob.clone());
                            println!("trying to decompress the data using brotli");
                            
                            // The 4844 blob is usually raw; we keep the helper in case a provider returns
                            // a compressed blob directly.
                            // let raw_blob = match try_brotli_decompress(&raw_blob) {
                            //     Some(decompressed) => {
                            //         println!("Blob was Brotli compressed, decompressed to {} bytes", decompressed.len());
                            //         decompressed
                            //     }
                            //     None => {
                            //         println!("Blob is raw (not Brotli). First 64 bytes: {}", hex::encode(&raw_blob[..64.min(raw_blob.len())]));
                            //         raw_blob
                            //     }
                            // };
                            // ---- compute and compare KZG commitment ----
                            match compute_kzg_commitment(&raw_blob) {
                                Some(commitment) => {
                                    println!("commitment: {}", &commitment);
                                    println!("Commitment from event: {}", blob.get("commitment").unwrap().as_str().unwrap());
                                    if commitment != blob.get("commitment").unwrap().as_str().unwrap() {
                                        println!("Commitment does not match");
                                        continue;
                                    } else {
                                        println!("Commitment matches");
                                    }
                                }
                                None => {
                                    println!("Failed to compute KZG commitment");
                                    continue;
                                }
                            }
                            // Given `event.timeBounds` from the log decode:
                            let ev = &event.timeBounds;
                            let ev_min_ts = ev.minTimestamp as u64;
                            let ev_max_ts = ev.maxTimestamp as u64;
                            let ev_min_bn = ev.minBlockNumber as u64;
                            let ev_max_bn = ev.maxBlockNumber as u64;

                            if let Err(e) = handle_raw_blob(&raw_blob, ev_min_ts, ev_max_ts, ev_min_bn, ev_max_bn) {
                                // Do not stop the main loop
                                eprintln!("handle_raw_blob failed: {e}");
                            }
                        }

                    }
                    Err(e) => {
                        // This can still fail if the ABI or indexing expectations differ.
                        println!("Failed to decode SequencerBatchDelivered event: {:#?}", e);
                    }
                }
            } else {
                // event not in consideration
                // println!("Skipped non-target event with topic0: {:#?}", topic0);
            } 
        } else {
            println!("Received log without topics: {:#?}", log);
        }
    }

    Ok(())
}