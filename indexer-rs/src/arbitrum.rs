use clap::Parser;
use serde::Serialize;
use eyre::{Result, WrapErr};

// alloy for versioned hash computation and typed 48-byte arrays
use alloy::{
    consensus::Bytes48,
    eips::eip4844::kzg_to_versioned_hash,
    primitives::B256,
};

fn to_0x_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(2 + bytes.len() * 2);
    s.push_str("0x");
    for b in bytes {
        use std::fmt::Write;
        write!(&mut s, "{:02x}", b).unwrap();
    }
    s
}

fn process_blob(blob: &[u8], preview_words: usize) -> (usize, usize, bool, Vec<String>) {
    // Split into 32-byte words
    let total_words = (blob.len() + 31) / 32;

    // Preview first N words
    let mut first_words = Vec::new();
    for i in 0..total_words.min(preview_words) {
        let start = i * 32;
        let end = ((i + 1) * 32).min(blob.len());
        first_words.push(to_0x_hex(&blob[start..end]));
    }

    // EIP-4844 blob size check: 4096 * 32 bytes
    let is_exact_eip4844_size = blob.len() == 4096 * 32;

    (blob.len(), total_words, is_exact_eip4844_size, first_words)
}

/// Optionally compute the versioned hash from a 48-byte KZG commitment (EIP-4844).
fn compute_versioned_hash_from_commitment(commitment: &[u8; 48]) -> String {
    let cmt = Bytes48::from_slice(commitment);
    let vh = kzg_to_versioned_hash(cmt.as_ref());
    format!("{vh:#x}")
}

pub async fn ArbitrumParser(blob: Vec<u8>) -> Result<()> {
    // Replace these with your real data sources.
    // 1) You already have your blob from an API:
    // let blob: Vec<u8> = {
    //     // Example data; replace with your real Vec<u8>
    //     // e.g., let blob = api_client.fetch_blob(...).await?;
    //     vec![0u8; 64] // 2 words of 32 bytes each
    // };

    // 2) Optional fields (provide them if you have them; otherwise keep as None)
    let commitment: Option<[u8; 48]> = None; // e.g., Some([...;48])
    let proof: Option<[u8; 48]> = None;
    let tx_hash: Option<[u8; 32]> = None;
    let index: Option<u32> = None;

    // Process the blob
    let preview_words = 4;
    let (blob_len_bytes, words_32b, is_exact_eip4844_size, first_words) =
        process_blob(&blob, preview_words);

    println!("blob_len_bytes: {}", blob_len_bytes);
    println!("words_32b: {}", words_32b);
    println!("is_exact_eip4844_size: {}", is_exact_eip4844_size);

    for (i, w) in first_words.iter().enumerate() {
        println!("first_words[{i}]: {w}");
    }

    // If you have a commitment, compute versioned hash like in the project
    if let Some(cmt) = commitment.as_ref() {
        let versioned_hash = compute_versioned_hash_from_commitment(cmt);
        println!("commitment: {}", to_0x_hex(cmt));
        println!("versioned_hash: {}", versioned_hash);
    }

    // Optionally print proof
    if let Some(p) = proof.as_ref() {
        println!("proof: {}", to_0x_hex(p));
    }

    // Optionally print tx_hash and index
    if let Some(th) = tx_hash.as_ref() {
        // Use alloy's B256 to format consistently with 0x-prefix
        let b = B256::from_slice(th);
        println!("tx_hash: {b:#x}");
    }

    if let Some(i) = index {
        println!("index: {i}");
    }
    Ok(())
}