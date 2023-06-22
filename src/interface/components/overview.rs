use std::io;
use crate::network_controller::TransactionDisplayInfo;
use crate::raw_transaction::TransactionOrigin;


/// Initializes the overview component.
pub fn init(_builder: gtk::Builder) -> io::Result<()> {
    Ok(())
}


/// Updates the overview component with recent transactions and the origin of the transaction.
pub fn update_overview(
    builder: gtk::Builder,
    transactions: Vec<TransactionDisplayInfo>,
    origin: TransactionOrigin,
) {

}