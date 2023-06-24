use gtk::prelude::BuilderExtManual;
use gtk::prelude::ImageExt;
use gtk::prelude::LabelExt;
use std::io;

use crate::network_controller::TransactionDisplayInfo;
use crate::raw_transaction::TransactionOrigin;

use super::utils::append_to_limited_container;

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
    let amount: f64 = (transaction.amount as f64 / 100000000.0) as f64;
    amount_label.set_text(format!("{:.8} tBTC", amount).as_str());

    let timestamp_label: gtk::Label = inner_builder
        .object("overview_transaction_template_timestamp")
        .unwrap();
    timestamp_label.set_text(&transaction.date.to_string());

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

    let tx_widget = get_transaction_widget(transaction, origin);

    append_to_limited_container(&overview_transaction_container, &tx_widget, 10);
}

/// Initializes the overview component.
pub fn init(_builder: gtk::Builder) -> io::Result<()> {
    Ok(())
}
