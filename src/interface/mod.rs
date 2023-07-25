use crate::interface::components::overview_panel::update_overview_transactions;
use crate::interface::components::overview_panel::TransactionDisplayInfo;
use crate::interface::components::send_panel::TransactionInfo;
use crate::interface::components::utils::create_notification_window;
use crate::utility::to_io_err;
use gtk::glib;
use gtk::glib::{Receiver as GtkReceiver, SyncSender as GtkSender};
use gtk::prelude::*;
use std::io;
use std::sync::mpsc::Sender;

use self::components::table::table_append_data;
use self::components::table::{GtkTable, GtkTableData};
use self::components::wallet_switcher::append_wallet;
pub mod components;

/// Enum with messages from the model to the interface
pub enum GtkMessage {
    /// label, text
    UpdateBalance((u64, u64)),
    UpdateOverviewTransactions(Vec<TransactionDisplayInfo>),
    /// type, notification title, notification message
    CreateNotification((gtk::MessageType, String, String)),
    UpdateTable((GtkTable, GtkTableData)),
    /// optional new status, fraction
    UpdateProgressBar((Option<String>, f64)),
    /// wallet address, is main wallet
    AddWalletEntry(String, bool),
}

pub type RecipientDetails = (String, String, u64); // (address, label, value)

/// Enum with requests from the interface to the model
pub enum ModelRequest {
    GenerateTransaction(TransactionInfo),
    ChangeActiveWallet(String), // wallet address
}

/// called from the model, to update the status bar in the ui
pub fn update_ui_progress_bar(
    sender: &GtkSender<GtkMessage>,
    new_status: Option<&str>,
    mut fraction: f64,
) -> io::Result<()> {
    if fraction > 1.0 {
        fraction = 1.0;
    }
    if let Some(new_status) = new_status {
        sender
            .send(GtkMessage::UpdateProgressBar((
                Some(new_status.to_string()),
                fraction,
            )))
            .map_err(to_io_err)
    } else {
        sender
            .send(GtkMessage::UpdateProgressBar((None, fraction)))
            .map_err(to_io_err)
    }
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
            GtkMessage::UpdateBalance((balance, pending)) => {
                update_balance(builder_aux, balance, pending);
            }
            GtkMessage::UpdateOverviewTransactions(transactions) => {
                _ = update_overview_transactions(builder_aux, transactions);
            }
            GtkMessage::CreateNotification((t, title, msg)) => {
                _ = create_notification_window(t, &title, &msg);
            }
            GtkMessage::UpdateTable((table, data)) => {
                let _res = table_append_data(builder_aux, table, data);
            }
            GtkMessage::UpdateProgressBar((new_status, fraction)) => {
                if let Some(progress_bar) = builder_aux.object::<gtk::ProgressBar>("progress_bar") {
                    progress_bar.set_fraction(fraction);
                    if let Some(new_status) = new_status {
                        progress_bar.set_text(Some(new_status.as_str()));
                    }
                }
            }
            GtkMessage::AddWalletEntry(wallet, is_main_wallet) => {
                append_wallet(builder_aux, wallet, is_main_wallet);
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
