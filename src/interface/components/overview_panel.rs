use crate::messages::HashId;
use gtk::prelude::BuilderExtManual;
use gtk::prelude::Cast;
use gtk::prelude::ContainerExt;
use gtk::prelude::ImageExt;
use gtk::prelude::LabelExt;
use std::io;

use crate::raw_transaction::TransactionOrigin;

use super::utils::append_to_limited_container;

pub enum TransactionRole {
    Receiver,
    Sender,
}

/// Struct that holds the information to be displayed in the transaction list in the UI
pub struct TransactionDisplayInfo {
    pub role: TransactionRole,
    pub date: String,
    pub amount: i64,
    pub hash: HashId,
}

fn try_remove_pending_transaction(overview_transaction_container: &gtk::Box, tx_hash: &str) {
    overview_transaction_container.foreach(|transaction| {
        if let Some(overview_tx) = transaction.downcast_ref::<gtk::Box>() {
            overview_tx.foreach(|widget: &gtk::Widget| {
                if let Some(inner_box) = widget.downcast_ref::<gtk::Box>() {
                    inner_box.foreach(|widget: &gtk::Widget| {
                        if let Some(hash_label) = widget.downcast_ref::<gtk::Label>() {
                            if hash_label.text() == tx_hash {
                                println!("removing pending transaction from overview");
                                overview_transaction_container.remove(overview_tx);
                            }
                        }
                    });
                }
            });
        }
    });
}

fn already_added(overview_transaction_container: &gtk::Box, tx_hash: &str) -> bool {
    let mut already_added = false;
    overview_transaction_container.foreach(|transaction| {
        if let Some(overview_tx) = transaction.downcast_ref::<gtk::Box>() {
            overview_tx.foreach(|widget: &gtk::Widget| {
                if let Some(inner_box) = widget.downcast_ref::<gtk::Box>() {
                    inner_box.foreach(|widget: &gtk::Widget| {
                        if let Some(hash_label) = widget.downcast_ref::<gtk::Label>() {
                            if hash_label.text() == tx_hash {
                                already_added = true;
                            }
                        }
                    });
                }
            });
        }
    });
    already_added
}

fn get_transaction_widget(
    transaction: TransactionDisplayInfo,
    origin: TransactionOrigin,
) -> gtk::Widget {
    let glade_src = include_str!("../res/ui.glade");
    let inner_builder = gtk::Builder::from_string(glade_src);

    let transaction_widget: gtk::Widget = inner_builder
        .object("overview_transaction_template")
        .expect("could not find transaction template");

    let hash_label: gtk::Label = inner_builder
        .object("overview_transaction_template_hash")
        .unwrap();
    hash_label.set_text(&transaction.hash.to_string());

    let amount_label: gtk::Label = inner_builder
        .object("overview_transaction_template_amount")
        .unwrap();
    let amount: f64 = transaction.amount as f64 / 100000000.0;
    amount_label.set_text(format!("{:.8} tBTC", amount).as_str());

    let timestamp_label: gtk::Label = inner_builder
        .object("overview_transaction_template_timestamp")
        .unwrap();
    timestamp_label.set_text(&transaction.date);

    let origin_img: gtk::Image = inner_builder
        .object("overview_transaction_template_img")
        .unwrap();

    match origin {
        TransactionOrigin::Block => {
            origin_img.set_file(Some("./src/interface/res/mined.png"));
        }
        TransactionOrigin::Pending => {
            origin_img.set_file(Some("./src/interface/res/pending.png"));
        }
    }

    transaction_widget
}

/// Updates the overview component with recent transactions and the origin of the transaction.
pub fn update_overview_transactions(
    builder: gtk::Builder,
    transaction: TransactionDisplayInfo,
    origin: TransactionOrigin,
) {
    let overview_transaction_container: gtk::Box =
        builder.object("overview_transactions_container").unwrap();

    if origin == TransactionOrigin::Block {
        try_remove_pending_transaction(
            &overview_transaction_container,
            &transaction.hash.to_string(),
        );
    }

    if already_added(
        &overview_transaction_container,
        &transaction.hash.to_string(),
    ) {
        return;
    }

    let tx_widget = get_transaction_widget(transaction, origin);

    append_to_limited_container(&overview_transaction_container, &tx_widget, 10);
}

/// Initializes the overview component.
pub fn init(_builder: gtk::Builder) -> io::Result<()> {
    Ok(())
}
