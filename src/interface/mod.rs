use crate::interface::components::overview_panel::update_overview_transactions;
use crate::interface::components::overview_panel::TransactionDisplayInfo;
use crate::interface::components::send_panel::TransactionInfo;
use crate::interface::components::utils::create_notification_window;
use crate::raw_transaction::TransactionOrigin;
use crate::utility::to_io_err;
use gtk::glib;
use gtk::glib::{Receiver as GtkReceiver, Sender as GtkSender};
use gtk::prelude::*;
use std::io;
use std::sync::mpsc::Sender;

use self::components::table::table_append_data;
use self::components::table::{GtkTable, GtkTableData};
pub mod components;

/// Enum with messages from the model to the interface
pub enum GtkMessage {
    UpdateLabel((String, String)),
    UpdateBalance((u64, u64)),
    UpdateOverviewTransactions((TransactionDisplayInfo, TransactionOrigin)),
    /// type, notification title, notification message
    CreateNotification((gtk::MessageType, String, String)),
    UpdateTable((GtkTable, GtkTableData)),
}

pub type RecipientDetails = (String, String, u64); // (address, label, value)

/// Enum with requests from the interface to the model
pub enum ModelRequest {
    GenerateTransaction(TransactionInfo),
}

/// called from the model, to update the status bar in the ui
pub fn update_ui_status_bar(sender: &GtkSender<GtkMessage>, msg: String) -> io::Result<()> {
    update_ui_label(sender, "status_bar".to_string(), msg)
}

/// called from the model, to update the text of a specific label
pub fn update_ui_label(
    sender: &GtkSender<GtkMessage>,
    label: String,
    text: String,
) -> io::Result<()> {
    sender
        .send(GtkMessage::UpdateLabel((label, text)))
        .map_err(to_io_err)
}

fn update_balance(builder: gtk::Builder, balance: u64, pending: u64) {
    // Format balances as (balance / 100000000.0)
    let balance = balance as f64 / 100000000.0;
    let pending = pending as f64 / 100000000.0;

    // Get balances labels and update them
    if let Some(balance_available_val) = builder.object::<gtk::Label>("balance_available_val") {
        balance_available_val.set_text(format!("{:.8}", balance).as_str());
    }

    if let Some(balance_pending_val) = builder.object::<gtk::Label>("balance_pending_val") {
        balance_pending_val.set_text(format!("{:.8}", pending).as_str());
    }

    if let Some(transaction_balance_label) =
        builder.object::<gtk::Label>("transaction_balance_label")
    {
        transaction_balance_label.set_text(format!("{:.8}", balance).as_str()); // Should it be balance or balance and pending?
    }

    if let Some(balance_total_val) = builder.object::<gtk::Label>("balance_total_val") {
        balance_total_val.set_text(format!("{:.8}", balance + pending).as_str());
    }
}

/// Receiver that listen from messages from the model
fn attach_rcv(receiver: GtkReceiver<GtkMessage>, builder: gtk::Builder) {
    receiver.attach(None, move |msg| {
        let builder_aux = builder.clone();
        match msg {
            GtkMessage::UpdateLabel((label, text)) => {
                if let Some(label) = builder_aux.object::<gtk::Label>(label.as_str()) {
                    label.set_text(text.as_str());
                }
            }
            GtkMessage::UpdateBalance((balance, pending)) => {
                update_balance(builder_aux, balance, pending);
            }
            GtkMessage::UpdateOverviewTransactions((transaction, origin)) => {
                update_overview_transactions(builder_aux, transaction, origin);
            }
            GtkMessage::CreateNotification((t, title, msg)) => {
                create_notification_window(t, &title, &msg);
            }
            GtkMessage::UpdateTable((table, data)) => {
                let _res = table_append_data(builder_aux, table, data);
            }
        }

        // Returning false here would close the receiver
        // and have senders fail
        glib::Continue(true)
    });
}

/// Initializes the GTK interface
pub fn init(receiver: GtkReceiver<GtkMessage>, sender: Sender<ModelRequest>) -> io::Result<()> {
    if gtk::init().is_err() {
        println!("Failed to initialize GTK.");
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "GTK init failed.",
        ));
    }

    let glade_src = include_str!("./res/ui.glade");
    let builder = gtk::Builder::from_string(glade_src);

    attach_rcv(receiver, builder.clone());

    let window: gtk::Window = components::init(builder, sender)?;
    window.show_all();
    gtk::main();
    Ok(())
}
