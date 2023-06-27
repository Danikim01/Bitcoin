use gtk::prelude::BuilderExtManual;
use gtk::prelude::DialogExt;
use gtk::traits::GtkWindowExt;
use gtk::traits::MessageDialogExt;
use gtk::{prelude::BoxExt, traits::ContainerExt};

/// appends a new gtk widget to a gtk box container limited to N elements
/// removing the last element if the box is full
/// (adds to the beginning of the box expand and fill, padding 0)
pub fn append_to_limited_container(box_container: &gtk::Box, widget: &gtk::Widget, limit: usize) {
    let children = box_container.children();
    let children_len = children.len();
    if children_len >= limit {
        box_container.remove(&children[children_len - 1]);
    }

    box_container.pack_start(widget, false, false, 0);
    box_container.reorder_child(widget, 0);
}

/// redraws a gtk box_container, deleting all previous elements and
/// adding the new ones
/// (adds to the beginning of the box expand and fill, padding 0)
pub fn redraw_container(box_container: &gtk::Box, widgets: Vec<gtk::Widget>) {
    let children = box_container.children();
    for child in children {
        box_container.remove(&child);
    }

    for widget in widgets {
        box_container.pack_start(&widget, false, false, 0);
        box_container.reorder_child(&widget, 0);
    }
}

/// creates a notification window of the specified type with a title and a message
pub fn create_notification_window(
    notification_type: gtk::MessageType,
    title: &str,
    message: &str,
) -> std::io::Result<()> {
    let glade_src = include_str!("../res/ui.glade");
    let builder = gtk::Builder::from_string(glade_src);
    //let parent: gtk::Window = builder.object("main_window");

    if let Some(parent) = builder.object::<gtk::Window>("main_window") {
        let dialog = gtk::MessageDialog::new(
            Some(&parent),
            gtk::DialogFlags::empty(),
            notification_type,
            gtk::ButtonsType::Ok,
            "",
        );
        // centering on parent doesn't work for some reason
        dialog.set_transient_for(Some(&parent));
        dialog.set_position(gtk::WindowPosition::CenterOnParent);
        dialog.set_text(Some(title));
        dialog.set_secondary_text(Some(message));

        dialog.connect_response(|dialog, _| dialog.close());
        dialog.run();
        return Ok(());
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::Other,
        "Unable to build notification window",
    ))
}
