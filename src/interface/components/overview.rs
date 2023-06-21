use std::io;
use crate::network_controller::TransactionDisplayInfo;
use crate::raw_transaction::TransactionOrigin;

pub fn init(_builder: gtk::Builder) -> io::Result<()> {
    Ok(())
}

pub fn update_overview(
    builder: gtk::Builder,
    transactions: Vec<TransactionDisplayInfo>,
    origin: TransactionOrigin,
) {

}