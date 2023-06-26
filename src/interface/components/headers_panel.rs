use std::io;

use super::{table::GtkTableData, utils::append_to_limited_container};
use gtk::prelude::{BuilderExtManual, Cast, ContainerExt, LabelExt};
use crate::interface::components::utils::redraw_container;

fn widget_from_data(data: GtkTableData) -> io::Result<gtk::Widget> {
    let (height, date, hash) = match data {
        GtkTableData::Header(height, date, hash) => (height, date, hash),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "wrong GtkTableData",
        ))?,
    };

    let glade_src = include_str!("../res/ui.glade");
    let builder = gtk::Builder::from_string(glade_src);

    let row: gtk::Box = builder.object("headers_table_row_template").unwrap();

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

    Ok(row.upcast())
}

pub fn add_data_to_headers_table(builder: gtk::Builder, data: GtkTableData) -> io::Result<()> {
    // println!("add data to headers table");
    let table_box: gtk::Box = builder.object("headers_table").unwrap();
    let mut widgets = vec![];

    match data {
        GtkTableData::Headers(vector) => {
            for (height, date, hash) in vector{
                let widget: gtk::Widget = widget_from_data(GtkTableData::Header(height,date,hash))?;
                widgets.push(widget);
            }
        },
        _ => println!("wrong GtkTableData"),
    }
    redraw_container(&table_box, widgets);
    //append_to_limited_container(&table_box, &widget, 100);
    Ok(())
}
