use std::io;

use gtk::prelude::BuilderExtManual;
use gtk::prelude::ButtonExt;
use gtk::prelude::ContainerExt;
use gtk::traits::BoxExt;

fn register_btn_panel_changer(
    builder: gtk::Builder,
    button: gtk::Button,
    panel_id: &str,
) -> Result<(), io::Error> {
    let panel: gtk::Box = builder
        .object("panel")
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Failed to get panel object"))?;

    let desired_panel: gtk::Grid = builder.object(panel_id).ok_or_else(|| {
        io::Error::new(io::ErrorKind::Other, "Failed to get desired panel object")
    })?;

    button.connect_clicked(move |_| {
        panel.foreach(|widget| {
            panel.remove(widget);
        });

        panel.pack_start(&desired_panel, true, true, 2);
    });

    Ok(())
}

fn set_default_panel(builder: gtk::Builder, panel_id: &str) -> Result<(), io::Error> {
    let panel: gtk::Box = builder
        .object("panel")
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Failed to get panel object"))?;

    let desired_panel: gtk::Grid = builder.object(panel_id).ok_or_else(|| {
        io::Error::new(io::ErrorKind::Other, "Failed to get desired panel object")
    })?;

    panel.pack_start(&desired_panel, true, true, 2);

    Ok(())
}

/// Initializes the overview component of the interface.
pub fn init(builder: gtk::Builder) -> io::Result<()> {
    let overview_btn: gtk::Button = builder
        .object("overview_btn")
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Failed to get overview_btn object"))?;

    let send_btn: gtk::Button = builder
        .object("send_btn")
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Failed to get send_btn object"))?;

    let headers_btn: gtk::Button = builder
        .object("headers_btn")
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Failed to get send_btn object"))?;

    let blocks_btn: gtk::Button = builder
        .object("blocks_btn")
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Failed to get send_btn object"))?;

    let transactions_btn: gtk::Button = builder.object("transactions_btn").ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::Other,
            "Failed to get transactions_btn object",
        )
    })?;

    let poi_btn: gtk::Button = builder
        .object("poi_btn")
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Failed to get poi_btn object"))?;

    register_btn_panel_changer(builder.clone(), overview_btn, "overview_panel")?;
    register_btn_panel_changer(builder.clone(), send_btn, "send_panel")?;
    register_btn_panel_changer(builder.clone(), headers_btn, "headers_panel")?;
    register_btn_panel_changer(builder.clone(), blocks_btn, "blocks_panel")?;
    register_btn_panel_changer(builder.clone(), transactions_btn, "transactions_panel")?;
    register_btn_panel_changer(builder.clone(), poi_btn, "poi_panel")?;

    set_default_panel(builder, "overview_panel")?;

    Ok(())
}
