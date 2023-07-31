use crate::messages::HashId;
use gtk::prelude::BuilderExtManual;
use gtk::prelude::ImageExt;
use gtk::prelude::LabelExt;
use std::io;

use crate::raw_transaction::TransactionOrigin;

use super::utils::redraw_container;

#[derive(Debug, Clone, PartialEq)]
pub enum TransactionRole {
    Receiver,
    Sender,
}

/// Struct that holds the information to be displayed in the transaction list in the UI
#[derive(Debug, Clone, PartialEq)]
pub struct TransactionDisplayInfo {
    pub role: TransactionRole,
    pub origin: TransactionOrigin,
    pub date: String,
    pub amount: i64,
    pub hash: HashId,
}

fn get_transaction_widget(transaction: TransactionDisplayInfo) -> Result<gtk::Widget, String> {
    let glade_src = include_str!("../res/ui.glade");
    let inner_builder = gtk::Builder::from_string(glade_src);

    let transaction_widget: gtk::Widget = inner_builder
        .object("overview_transaction_template")
        .ok_or("Could not find transaction template")?;

    let hash_label: gtk::Label = inner_builder
        .object("overview_transaction_template_hash")
        .ok_or("Could not find hash label")?;
    hash_label.set_text(&transaction.hash.to_string());

    let amount_label: gtk::Label = inner_builder
        .object("overview_transaction_template_amount")
        .ok_or("Could not find amount label")?;
    let amount: f64 = transaction.amount as f64 / 100000000.0;
    amount_label.set_text(format!("{:.8} tBTC", amount).as_str());

    let timestamp_label: gtk::Label = inner_builder
        .object("overview_transaction_template_timestamp")
        .ok_or("Could not find timestamp label")?;
    timestamp_label.set_text(&transaction.date);

    let origin_img: gtk::Image = inner_builder
        .object("overview_transaction_template_img")
        .ok_or("Could not find origin image")?;

    match transaction.origin {
        TransactionOrigin::Block => {
            origin_img.set_file(Some("./src/interface/res/mined.png"));
        }
        TransactionOrigin::Pending => {
            origin_img.set_file(Some("./src/interface/res/pending.png"));
        }
    }

    Ok(transaction_widget)
}

/// Updates the overview component with recent transactions and the origin of the transaction.
pub fn update_overview_transactions(
    builder: gtk::Builder,
    transactions: Vec<TransactionDisplayInfo>,
) -> Result<(), String> {
    let overview_transaction_container: gtk::Box = builder
        .object("overview_transactions_container")
        .ok_or("Could not find overview transaction container")?;

    let mut tx_widgets = Vec::new();
    for tx in transactions {
        let tx_widget = get_transaction_widget(tx)?;
        tx_widgets.push(tx_widget);
    }

    redraw_container(&overview_transaction_container, tx_widgets);
    Ok(())
}

/// Initializes the overview component.
pub fn init(_builder: gtk::Builder) -> io::Result<()> {
    Ok(())
}
