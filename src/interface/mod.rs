use gtk::glib;
use gtk::glib::{Receiver as GtkReceiver, Sender as GtkSender};
use gtk::prelude::*;
use std::io;
use std::sync::mpsc::Sender;

use crate::utility::to_io_err;

mod components;

pub enum GtkMessage {
    UpdateLabel((String, String)),
}

pub type TransactionDetails = (String, String, u64); // (address, label, value)

pub enum ModelRequest {
    GetWalletBalance,
    GenerateTransaction(TransactionDetails),
}

/// called from the model, to update the text of a specific label
pub fn update_ui_label(sender: GtkSender<GtkMessage>, label: String, text: String) -> io::Result<()> {
    sender.send(GtkMessage::UpdateLabel((label, text))).map_err(to_io_err)
}

fn attach_rcv(receiver: GtkReceiver<GtkMessage>, builder: gtk::Builder) {
    receiver.attach(None, move |msg| {
        match msg {
            GtkMessage::UpdateLabel((label, text)) => {
                let builder_aux = builder.clone();
                let label: gtk::Label = builder_aux.object(label.as_str()).unwrap();
                label.set_text(text.as_str());
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

    // this only for example
    let get_balance_btn = builder.object::<gtk::Button>("get_balance_btn").unwrap();
    let sender_clone = sender.clone();
    get_balance_btn.connect_clicked(move |_| {
        println!("click");
        sender_clone.send(ModelRequest::GetWalletBalance).unwrap();
    });

    let send_tx_btn = builder.object::<gtk::Button>("send_tx_btn").unwrap();
    send_tx_btn.connect_clicked(move |_| {
        println!("click");
        let address = "mnJvq7mbGiPNNhUne4FAqq27Q8xZrAsVun".to_string();
        let value = 10000;
        let details = (address, "label".to_string(), value);
        sender
            .send(ModelRequest::GenerateTransaction(details))
            .unwrap();
    });

    let window: gtk::Window = components::init(builder)?;
    window.show_all();

    gtk::main();
    Ok(())
}
