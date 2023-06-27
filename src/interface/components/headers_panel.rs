use std::io;

use super::table::GtkTableData;
use crate::interface::components::utils::redraw_container;
use gtk::prelude::{BuilderExtManual, Cast, ContainerExt, LabelExt};

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
    if let Some(row) = builder.object::<gtk::Box>("headers_table_row_template") {
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
        return Ok(row.upcast());
    }
    Err(io::Error::new(
        io::ErrorKind::Other,
        "Unable to build headers table row template",
    ))
}

pub fn add_data_to_headers_table(builder: gtk::Builder, data: GtkTableData) -> io::Result<()> {
    if let Some(table_box) = builder.object::<gtk::Box>("headers_table") {
        let mut widgets = vec![];
        match data {
            GtkTableData::Headers(vector) => {
                for (height, date, hash) in vector {
                    let widget: gtk::Widget =
                        widget_from_data(GtkTableData::Header(height, date, hash))?;
                    widgets.push(widget);
                }
            }
            _ => println!("wrong GtkTableData"),
        }
        redraw_container(&table_box, widgets);
        return Ok(());
    }
    Err(io::Error::new(
        io::ErrorKind::Other,
        "Unable to build headers table",
    ))
}
