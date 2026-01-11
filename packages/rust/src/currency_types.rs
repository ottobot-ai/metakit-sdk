// ! Currency transaction types for metagraph token transfers

use serde::{Deserialize, Deserializer, Serialize};

use crate::types::Signed;

/// Custom deserializer for salt field that accepts both number and string
fn deserialize_salt<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrNumber {
        String(String),
        Number(i64),
    }

    match StringOrNumber::deserialize(deserializer)? {
        StringOrNumber::String(s) => Ok(s),
        StringOrNumber::Number(n) => Ok(n.to_string()),
    }
}

/// Token decimals constant (1e-8)
/// Same as DAG_DECIMALS from dag4.js
pub const TOKEN_DECIMALS: f64 = 1e-8;

/// Reference to a previous transaction for chaining
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransactionReference {
    /// Transaction hash (64-character hex string)
    pub hash: String,
    /// Transaction ordinal number
    pub ordinal: i64,
}

/// Currency transaction value structure (v2)
/// Contains the actual transaction data before signing
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CurrencyTransactionValue {
    /// Source DAG address
    pub source: String,
    /// Destination DAG address
    pub destination: String,
    /// Amount in smallest units (1e-8)
    pub amount: i64,
    /// Fee in smallest units (1e-8)
    pub fee: i64,
    /// Reference to parent transaction
    pub parent: TransactionReference,
    /// Random salt for uniqueness (as string)
    #[serde(deserialize_with = "deserialize_salt")]
    pub salt: String,
}

/// Currency transaction structure (v2)
/// A signed currency transaction value
/// Used for metagraph token transfers
pub type CurrencyTransaction = Signed<CurrencyTransactionValue>;

/// Parameters for creating a token transfer
#[derive(Debug, Clone)]
pub struct TransferParams {
    /// Destination DAG address
    pub destination: String,
    /// Amount in token units (e.g., 100.5 tokens)
    pub amount: f64,
    /// Fee in token units (defaults to 0)
    pub fee: f64,
}
