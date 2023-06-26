use chrono::Utc;

use crate::messages::{Block, BlockHeader, Hashable};
use crate::raw_transaction::RawTransaction;

use super::blocks_panel::add_data_to_blocks_table;
use super::headers_panel::add_data_to_headers_table;
use super::transactions_panel::add_data_to_transactions_table;

use crate::messages::utility::date_from_timestamp;
use std::io;

#[derive(Clone)]
/// Enum with the different tables in the interface
pub enum GtkTable {
    Transactions,
    Blocks,
    Headers,
}

/// Enum with the different types of data that can be added to a table
pub enum GtkTableData {
    /// height, date, hash, tx count
    Blocks(Vec<(String, String, String, String)>),
    Block(String, String, String, String),
    /// height, date, hash (all as String)
    Headers(Vec<(String, String, String)>),
    Header(String, String, String),
    /// date, hash, amount
    Transaction(String, String, String),
}

/// Receive a raw transaction and parse it's data to a RowData::TransactionData
pub fn table_data_from_tx(tx: &RawTransaction) -> GtkTableData {
    // need date, hash and amount
    let date = Utc::now().format("%d-%m-%Y %H:%M").to_string();
    let hash = tx.get_hash();
    let amount = format!("{:.8}", tx.get_total_output_value() as f64 / 100000000.0);

    GtkTableData::Transaction(date, hash.to_string(), amount)
}

/// Receive a vector of blocks and parse their data to a vector of RowData::BlocksData
pub fn table_data_from_blocks(blocks: Vec<&Block>) -> GtkTableData {
    // need height, date, hash and tx count
    let mut data = Vec::new();

    for block in blocks {
        data.push((
            block.header.height.to_string(),
            date_from_timestamp(block.header.timestamp),
            block.hash().to_string(),
            block.txn_count.to_string()),
        );
    }

    GtkTableData::Blocks(data)
}

/// Receive a vector of headers and parse their data to a vector of RowData::HeadersData
pub fn table_data_from_headers(headers: Vec<&BlockHeader>) -> GtkTableData {
    // need height, date and hash
    let mut data = Vec::new();

    for header in headers {
        data.push((
            header.height.to_string(),
            date_from_timestamp(header.timestamp),
            header.hash().to_string()));
    }

    GtkTableData::Headers(data)
}

pub fn table_append_data(
    builder: gtk::Builder,
    table: GtkTable,
    data: GtkTableData,
) -> io::Result<()> {
    match table {
        GtkTable::Transactions => add_data_to_transactions_table(builder, data),
        GtkTable::Blocks => add_data_to_blocks_table(builder, data),
        GtkTable::Headers => add_data_to_headers_table(builder, data),
    }
}
