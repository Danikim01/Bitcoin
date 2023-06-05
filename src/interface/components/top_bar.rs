use std::io;

use gtk::prelude::BuilderExtManual;
use gtk::prelude::ButtonExt;
use gtk::prelude::ContainerExt;
use gtk::traits::BoxExt;

// add err handling
fn register_btn_panel_changer(builder: gtk::Builder, button: gtk::Button, panel_id: &str) {
    let panel: gtk::Box = builder.object("panel").unwrap(); // handle error
    let desired_panel: gtk::Grid = builder.object(panel_id).unwrap(); // handle error
    button.connect_clicked(move |_| {
        // remove all widgets from panel_id
        panel.foreach(|widget| {
            panel.remove(widget);
        });

        panel.pack_start(&desired_panel, true, true, 2);
    });
}

pub fn init(builder: gtk::Builder) -> io::Result<()> {
    let overview_btn: gtk::Button = builder.object("overview_btn").unwrap();
    let transactions_btn: gtk::Button = builder.object("transactions_btn").unwrap();

    register_btn_panel_changer(builder.clone(), overview_btn, "overview_panel");
    register_btn_panel_changer(builder, transactions_btn, "transactions_panel");

    Ok(())
}