use std::io;

use gtk::prelude::BuilderExtManual;
use gtk::prelude::ButtonExt;
use gtk::traits::EntryExt;

use crate::interface::ModelRequest;
use crate::interface::TransactionDetails;
use std::sync::mpsc::Sender;

pub fn init(builder: gtk::Builder, sender: Sender<ModelRequest>) -> io::Result<()> {
    let transaction_send_btn: gtk::Button = builder.object("transaction_send_btn").unwrap(); // handle error

    transaction_send_btn.connect_clicked(move |_| {
        // get entries from all recipients
        let address: gtk::Entry = builder.object("transaction_address_0_entry").unwrap();
        let label: gtk::Entry = builder.object("transaction_label_0_entry").unwrap();
        let amount: gtk::Entry = builder.object("transaction_amount_0_entry").unwrap();
        println!("address: {}", address.text());
        println!("label: {}", label.text());
        println!("amount: {}", amount.text());

        let value: u64 = match amount.text().parse::<u64>() {
            Ok(v) => v,
            Err(_) => 0,
        };

        let details: TransactionDetails = (address.text().to_string(), label.text().to_string(), value);
        sender
            .send(ModelRequest::GenerateTransaction(details))
            .unwrap();
    });

    Ok(())
}
