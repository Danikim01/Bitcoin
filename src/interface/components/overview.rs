use std::io;

use gtk::prelude::BuilderExtManual;
use gtk::prelude::ButtonExt;
use gtk::prelude::LabelExt;

pub fn init(builder: gtk::Builder) -> io::Result<()> {
    let get_balance_btn: gtk::Button = builder.object("get_balance_btn").unwrap(); // handle error

    get_balance_btn.connect_clicked(move |_| {
        let balance_available_val: gtk::Label = builder.object("balance_available_val").unwrap(); // add err handling

        // call to model

        balance_available_val.set_text("Getting balance...");
    });

    Ok(())
}
