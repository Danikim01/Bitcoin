use chrono::Local;

use crate::messages::{Block, Headers};
use crate::raw_transaction::RawTransaction;

use super::blocks_panel::add_data_to_blocks_table;
use super::headers_panel::add_data_to_headers_table;
use super::transactions_panel::add_data_to_transactions_table;

/// Enum with the different tables in the interface
pub enum GtkTable {
    Transactions,
    Blocks,
    Headers,
}

/// Enum with the different types of data that can be added to a table
pub enum GtkTableData {
    BlocksData(String),
    HeadersData(String),
    TransactionData(String),
}

/// Receive a raw transaction and parse it's data to a RowData::TransactionData
pub fn table_data_from_tx(tx: &RawTransaction) -> GtkTableData {
    // need date, hash and amount
    let date = Local::now().format("%d-%m-%Y %H:%M").to_string();

    GtkTableData::TransactionData(date)
}

/// Receive a block and parse it's data to a RowData::BlocksData
pub fn table_data_from_block(block: &Block) -> GtkTableData {
    // need height, date, hash and tx count

    GtkTableData::BlocksData("foo".to_string())
}

/// Receive a header and parse it's data to a RowData::HeadersData
pub fn table_data_from_headers(headers: &Headers) -> GtkTableData {
    // need height, date and hash
    
    GtkTableData::HeadersData("foo".to_string())
}

pub fn table_append_data(builder: gtk::Builder, table: GtkTable, data: GtkTableData) {
    match table {
        GtkTable::Transactions => add_data_to_transactions_table(builder, table, data),
        GtkTable::Blocks => add_data_to_blocks_table(builder, table, data),
        GtkTable::Headers => add_data_to_headers_table(builder, table, data),
    }
}
