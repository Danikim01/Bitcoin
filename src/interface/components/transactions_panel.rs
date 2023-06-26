use super::{table::GtkTableData, utils::append_to_limited_container};
use gtk::{
    prelude::{BuilderExtManual, Cast, LabelExt},
    traits::ContainerExt,
};
use std::io;

fn widget_from_data(data: GtkTableData) -> io::Result<gtk::Widget> {
    let (date, hash, amount) = match data {
        GtkTableData::Transaction(date, hash, amount) => (date, hash, amount),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "wrong GtkTableData",
        ))?,
    };

    let glade_src = include_str!("../res/ui.glade");
    let builder = gtk::Builder::from_string(glade_src);

    //let row: gtk::Box = builder.object("transactions_table_row_template").unwrap();
    if let Some(row) = builder.object::<gtk::Box>("transactions_table_row_template") {
        let elemets = row.children();
        if let Some(date_label) = elemets[0].downcast_ref::<gtk::Label>() {
            date_label.set_text(&date);
        }
        if let Some(hash_label) = elemets[1].downcast_ref::<gtk::Label>() {
            hash_label.set_text(&hash);
        }
        if let Some(amount_label) = elemets[2].downcast_ref::<gtk::Label>() {
            amount_label.set_text(&amount);
        }

        return Ok(row.upcast());
    }

    Err(io::Error::new(
        io::ErrorKind::InvalidInput,
        "failed to create widget from data",
    ))
}

pub fn add_data_to_transactions_table(builder: gtk::Builder, data: GtkTableData) -> io::Result<()> {
    // println!("add data to transactions table");
    //let table_box: gtk::Box = builder.object("transactions_table").unwrap();

    if let Some(table_box) = builder.object::<gtk::Box>("transactions_table") {
        let widget: gtk::Widget = widget_from_data(data)?;
        append_to_limited_container(&table_box, &widget, 100);
        return Ok(());
    }

    Err(io::Error::new(
        io::ErrorKind::InvalidInput,
        "failed to add data to transactions table",
    ))
}
