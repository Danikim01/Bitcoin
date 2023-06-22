use std::io;

use crate::interface::ModelRequest;
use crate::interface::RecipientDetails;
use gtk::prelude::BuilderExtManual;
use gtk::prelude::ButtonExt;
use gtk::traits::BoxExt;
use gtk::traits::ContainerExt;
use gtk::traits::EntryExt;
use std::sync::mpsc::Sender;

use gtk::prelude::Cast;

#[derive(Debug, Clone)]

/// Struct with transaction info (recipients and fee)
pub struct TransactionInfo {
    pub recipients: Vec<RecipientDetails>,
    pub fee: u64,
}

fn transaction_details_from_entries(entries: Vec<gtk::Entry>) -> RecipientDetails {
    let float_value: f64 = entries[2].text().parse::<f64>().unwrap_or(0.0);

    let value: u64 = (float_value * 100000000.0) as u64;

    (
        entries[0].text().to_string(),
        entries[1].text().to_string(),
        value,
    )
}

fn get_recipients(builder: gtk::Builder) -> Vec<RecipientDetails> {
    let mut recipients_details: Vec<RecipientDetails> = Vec::new();
    let recipients: gtk::Box = builder.object("transaction_recipients_info").unwrap(); // handle error

    // iterate over all recipients
    recipients.foreach(|r: &gtk::Widget| {
        // cast recipient to gtk::Box
        if let Ok(recipient) = r.clone().downcast::<gtk::Box>() {
            // iterate over all enries boxes in recipient
            let mut entries: Vec<gtk::Entry> = Vec::new();
            recipient.foreach(|e: &gtk::Widget| {
                if let Ok(entry_box) = e.clone().downcast::<gtk::Box>() {
                    // get entry from entry box
                    entry_box.foreach(|entry: &gtk::Widget| {
                        if let Ok(e) = entry.clone().downcast::<gtk::Entry>() {
                            entries.push(e);
                        }
                    });
                }
            });
            recipients_details.push(transaction_details_from_entries(entries));
        }
    });

    recipients_details
}

fn connect_send_btn(builder: gtk::Builder, sender: Sender<ModelRequest>) -> io::Result<()> {
    let transaction_send_btn: gtk::Button = builder
        .object("transaction_send_btn")
        .expect("could not find transaction send btn");

    transaction_send_btn.connect_clicked(move |_| {
        let recipients: Vec<RecipientDetails> = get_recipients(builder.clone());

        // get fee
        let fee: u64 = match builder.object::<gtk::Entry>("transaction_fee_entry") {
            Some(f) => {
                let float_value = f.text().parse::<f64>().unwrap_or(0.0);
                (float_value * 100000000.0) as u64
            }
            _ => 0,
        };

        let transaction_info = TransactionInfo { recipients, fee };

        match sender.send(ModelRequest::GenerateTransaction(transaction_info)) {
            Ok(_) => (),
            Err(_) => println!("could not send transaction details to model"),
        }
    });
    Ok(())
}

fn connect_clear_all_btn(builder: gtk::Builder) -> io::Result<()> {
    let transaction_clear_all_btn: gtk::Button = builder
        .object("transaction_clear_btn")
        .expect("could not find transaction clear all btn");

    let transaction_recipients_info: gtk::Box = builder
        .object("transaction_recipients_info")
        .expect("could not find transaction recipients info");

    transaction_clear_all_btn.connect_clicked(move |_| {
        // clear all recipients
        transaction_recipients_info.foreach(|widget| {
            transaction_recipients_info.remove(widget);
        });

        // add one empty recipient
        let glade_src = include_str!("../res/ui.glade");
        let inner_builder = gtk::Builder::from_string(glade_src);
        let new_recipient: gtk::Box = inner_builder
            .object("transaction_info_template")
            .expect("could not find transaction recipient template");

        transaction_recipients_info.pack_start(&new_recipient, false, false, 0);
    });

    Ok(())
}

fn connect_append_btn(builder: gtk::Builder) -> io::Result<()> {
    let transaction_append_btn: gtk::Button = builder
        .object("transaction_add_recipient_btn")
        .expect("could not find transaction append btn");

    // get box where recipients are appended
    let transaction_recipients_info: gtk::Box = builder
        .object("transaction_recipients_info")
        .expect("could not find transaction recipients info");

    transaction_append_btn.connect_clicked(move |_| {
        let glade_src = include_str!("../res/ui.glade");
        let inner_builder = gtk::Builder::from_string(glade_src);

        // get recipient template from builder
        let new_recipient: gtk::Box = inner_builder
            .object("transaction_info_template")
            .expect("could not find transaction recipient template");

        transaction_recipients_info.pack_start(&new_recipient, true, true, 0);
    });

    Ok(())
}

/// Initialize send panel components 
pub fn init(builder: gtk::Builder, sender: Sender<ModelRequest>) -> io::Result<()> {
    connect_send_btn(builder.clone(), sender)?;
    connect_clear_all_btn(builder.clone())?;
    connect_append_btn(builder)?;

    Ok(())
}
