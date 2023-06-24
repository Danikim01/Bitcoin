use super::{
    table::{GtkTable, GtkTableData},
    utils::append_to_limited_container,
};
use gtk::prelude::{BuilderExtManual, Cast};

fn widget_from_data(builder: gtk::Builder, data: GtkTableData) -> gtk::Widget {
    let foo = gtk::Label::new(Some("foo"));
    foo.upcast()
}

pub fn add_data_to_headers_table(builder: gtk::Builder, table: GtkTable, data: GtkTableData) {
    // println!("add data to headers table");
    let table_box: gtk::Box = builder.object("headers_table").unwrap();

    let widget: gtk::Widget = widget_from_data(builder, data);
    append_to_limited_container(&table_box, &widget, 100)
}
