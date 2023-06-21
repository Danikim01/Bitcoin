use crate::interface::components::send_panel::TransactionInfo;
use gtk::glib;
use gtk::glib::{Receiver as GtkReceiver, Sender as GtkSender};
use gtk::prelude::*;
use std::io;
use std::sync::mpsc::Sender;
use crate::network_controller::TransactionDisplayInfo;
use crate::raw_transaction::TransactionOrigin;

use crate::utility::to_io_err;

pub mod components;

pub enum GtkMessage {
    UpdateLabel((String, String)),
    UpdateBalance((u64, u64)),
    UpdateOverview((Vec<TransactionDisplayInfo>, TransactionOrigin)),
}

pub type RecipientDetails = (String, String, u64); // (address, label, value)

pub enum ModelRequest {
    GenerateTransaction(TransactionInfo),
}

/// called from the model, to update the text of a specific label
pub fn update_ui_label(
    sender: GtkSender<GtkMessage>,
    label: String,
    text: String,
) -> io::Result<()> {
    sender
        .send(GtkMessage::UpdateLabel((label, text)))
        .map_err(to_io_err)
}

fn attach_rcv(receiver: GtkReceiver<GtkMessage>, builder: gtk::Builder) {
    receiver.attach(None, move |msg| {
        match msg {
            GtkMessage::UpdateLabel((label, text)) => {
                let builder_aux = builder.clone();
                let label: gtk::Label = builder_aux.object(label.as_str()).unwrap();
                label.set_text(text.as_str());
            }
            GtkMessage::UpdateBalance((balance, pending)) => {
                let builder_aux = builder.clone();

                // format balances as (balance / 100000000.0)
                let balance = balance as f64 / 100000000.0;
                let pending = pending as f64 / 100000000.0;

                // get balances labels and update them
                let balance_available_val: gtk::Label =
                    builder_aux.object("balance_available_val").unwrap();
                balance_available_val.set_text(format!("{:.8}", balance).as_str());

                let balance_pending_val: gtk::Label =
                    builder_aux.object("balance_pending_val").unwrap();
                balance_pending_val.set_text(format!("{:.8}", pending).as_str());

                let transaction_balance_label: gtk::Label =
                    builder_aux.object("transaction_balance_label").unwrap();
                transaction_balance_label.set_text(format!("{:.8}", balance).as_str()); // should it be balance or balance and pending?

                let balance_total_val: gtk::Label =
                    builder_aux.object("balance_total_val").unwrap();
                balance_total_val.set_text(format!("{:.8}", balance + pending).as_str());
            }
            GtkMessage::UpdateOverview((transactions, origin)) => {
                //update_overview(builder.clone(), transactions, origin);
            }
        }

        // Returning false here would close the receiver
        // and have senders fail
        glib::Continue(true)
    });
}

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
