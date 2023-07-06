use crate::interface::ModelRequest;
use std::io;
use std::sync::mpsc::Sender;

use gtk::{
    prelude::{BuilderExtManual, ComboBoxExt, ComboBoxTextExt},
    ComboBoxText,
};

fn register_wallet_entries_change_listener(
    wallet_entries: ComboBoxText,
    sender: Sender<ModelRequest>,
) {
    wallet_entries.connect_changed(move |wallet_entries| {
        if let Some(active_wallet) = wallet_entries.active_text() {
            let wallet = active_wallet.to_string();
            _ = sender.send(ModelRequest::ChangeActiveWallet(wallet))
        }
    });
}

pub fn append_wallet(wallet: String, builder: gtk::Builder) {
    if let Some(wallet_entries) = builder.object::<gtk::ComboBoxText>("wallet_entries") {
        wallet_entries.append_text(wallet.as_str());
    }
}

/// Initializes the wallet switcher component of the interface.
pub fn init(builder: gtk::Builder, sender: Sender<ModelRequest>) -> io::Result<()> {
    if let Some(wallet_entries) = builder.object::<gtk::ComboBoxText>("wallet_entries") {
        register_wallet_entries_change_listener(wallet_entries, sender);
    }
    Ok(())
}
