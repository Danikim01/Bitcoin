/// Enum with the different tables in the interface
pub enum GtkTable {
    Transactions,
    Blocks,
    Headers,
}

pub enum RowData {
    TransactionData(String),
    BlocksData(String),
    HeadersData(String),
}
