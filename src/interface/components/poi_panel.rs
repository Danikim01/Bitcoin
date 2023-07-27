use std::io;

use crate::interface::ModelRequest;
use gtk::prelude::BuilderExtManual;
use gtk::prelude::ButtonExt;
use gtk::prelude::Cast;
use gtk::prelude::EntryExt;
use gtk::traits::ContainerExt;
use std::sync::mpsc::Sender;

fn get_poi_inputs(builder: gtk::Builder) -> io::Result<(String, String)> {
    let mut block_hash: String = String::default();
    let mut tx_hash: String = String::default();

    let input_form: gtk::Grid = builder
        .object("poi_panel")
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "could not find poi inputs"))?;

    input_form.foreach(|w: &gtk::Widget| {
        if let Ok(entry) = w.clone().downcast::<gtk::Entry>() {
            if tx_hash == String::default() {
                tx_hash = entry.text().to_string();
            } else {
                block_hash = entry.text().to_string();
            }
        }
    });

    Ok((block_hash, tx_hash))
}

fn connect_get_poi_btn(builder: gtk::Builder, sender: Sender<ModelRequest>) -> io::Result<()> {
    let get_poi_btn: gtk::Button = builder
        .object("get_poi_btn")
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "could not find get poi btn"))?;

    get_poi_btn.connect_clicked(move |_| {
        let (block_hash, tx_hash) = get_poi_inputs(builder.clone()).unwrap_or_default();
        if sender
            .send(ModelRequest::GetPoi(block_hash, tx_hash))
            .is_err()
        {
            println!("could not send poi details to model");
        }
    });

    Ok(())
}

pub fn init(builder: gtk::Builder, sender: Sender<ModelRequest>) -> io::Result<()> {
    connect_get_poi_btn(builder, sender)?;
    Ok(())
}
