use std::io;

use super::table::GtkTableData;
use gtk::prelude::{BuilderExtManual, Cast, ContainerExt, LabelExt};
use crate::interface::components::utils::redraw_container;

fn widget_from_data(data: GtkTableData) -> io::Result<gtk::Widget> {
    let (height, date, hash, tx_count) = match data {
        GtkTableData::Block(height, date, hash, tx_count) => (height, date, hash, tx_count),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "wrong GtkTableData",
        ))?,
    };

    let glade_src = include_str!("../res/ui.glade");
    let builder = gtk::Builder::from_string(glade_src);

    let row: gtk::Box = builder.object("blocks_table_row_template").unwrap();

    let elemets = row.children();
    if let Some(height_label) = elemets[0].downcast_ref::<gtk::Label>() {
        height_label.set_text(&height);
    }
    if let Some(date_label) = elemets[1].downcast_ref::<gtk::Label>() {
        date_label.set_text(&date);
    }
    if let Some(hash_label) = elemets[2].downcast_ref::<gtk::Label>() {
        hash_label.set_text(&hash);
    }
    if let Some(tx_count_label) = elemets[3].downcast_ref::<gtk::Label>() {
        tx_count_label.set_text(&tx_count);
    }

    Ok(row.upcast())
}

pub fn add_data_to_blocks_table(builder: gtk::Builder, data: GtkTableData) -> io::Result<()> {
    // println!("add data to blocks table");
    let table_box: gtk::Box = builder.object("blocks_table").unwrap();
    let mut widgets = vec![];

    match data {
        GtkTableData::Blocks(vector) => {
            for (height, date, hash, tx_count) in vector{
                let widget: gtk::Widget = widget_from_data(GtkTableData::Block(height, date, hash, tx_count))?;
                widgets.push(widget);
            }
        },
        _ => println!("wrong GtkTableData"),
    }

    redraw_container(&table_box, widgets);
    //append_to_limited_container(&table_box, &widget, 100);
    Ok(())
}
