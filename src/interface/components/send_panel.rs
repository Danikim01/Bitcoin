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

/// Struct with transaction info (recipients and fee)
#[derive(Debug, Clone)]
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

fn get_recipients(builder: gtk::Builder) -> io::Result<Vec<RecipientDetails>> {
    let mut recipients_details: Vec<RecipientDetails> = Vec::new();
    let recipients: gtk::Box = builder
        .object("transaction_recipients_info")
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "could not find transaction recipients info",
            )
        })?;

    recipients.foreach(|r: &gtk::Widget| {
        if let Ok(recipient) = r.clone().downcast::<gtk::Box>() {
            let mut entries: Vec<gtk::Entry> = Vec::new();
            recipient.foreach(|e: &gtk::Widget| {
                if let Ok(entry_box) = e.clone().downcast::<gtk::Box>() {
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

    Ok(recipients_details)
}

fn connect_send_btn(builder: gtk::Builder, sender: Sender<ModelRequest>) -> io::Result<()> {
    let transaction_send_btn: gtk::Button =
        builder.object("transaction_send_btn").ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "could not find transaction send btn",
            )
        })?;

    transaction_send_btn.connect_clicked(move |_| match get_recipients(builder.clone()) {
        Ok(recipients) => {
            let fee: u64 = match builder.object::<gtk::Entry>("transaction_fee_entry") {
                Some(f) => {
                    let float_value = f.text().parse::<f64>().unwrap_or(0.0);
                    (float_value * 100000000.0) as u64
                }
                None => 0,
            };

            let transaction_info = TransactionInfo { recipients, fee };

            if let Err(_) = sender.send(ModelRequest::GenerateTransaction(transaction_info)) {
                println!("could not send transaction details to model");
            }
        }
        Err(e) => {
            println!("could not get recipients: {}", e);
        }
    });

    Ok(())
}

fn connect_clear_all_btn(builder: gtk::Builder) -> io::Result<()> {
    let transaction_clear_all_btn: gtk::Button =
        builder.object("transaction_clear_btn").ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "could not find transaction clear all btn",
            )
        })?;
    let transaction_recipients_info: gtk::Box = builder
        .object("transaction_recipients_info")
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "could not find transaction recipients info",
            )
        })?;

    transaction_clear_all_btn.connect_clicked(move |_| {
        transaction_recipients_info.foreach(|widget| {
            transaction_recipients_info.remove(widget);
        });

        let glade_src = include_str!("../res/ui.glade");
        let inner_builder = gtk::Builder::from_string(glade_src);
        let new_recipient: gtk::Widget =
            if let Some(widget) = inner_builder.object("transaction_info_template") {
                widget
            } else {
                println!("could not find transaction recipient template");
                return;
            };

        transaction_recipients_info.pack_start(&new_recipient, false, false, 0);
    });

    Ok(())
}

fn connect_append_btn(builder: gtk::Builder) -> io::Result<()> {
    let transaction_append_btn: gtk::Button = builder
        .object("transaction_add_recipient_btn")
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "could not find transaction append btn",
            )
        })?;
    let transaction_recipients_info: gtk::Box = builder
        .object("transaction_recipients_info")
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "could not find transaction recipients info",
            )
        })?;

    transaction_append_btn.connect_clicked(move |_| {
        let glade_src = include_str!("../res/ui.glade");
        let inner_builder = gtk::Builder::from_string(glade_src);

        let new_recipient: gtk::Widget =
            if let Some(widget) = inner_builder.object("transaction_info_template") {
                widget
            } else {
                println!("could not find transaction recipient template");
                return;
            };

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
