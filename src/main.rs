use std::io;
mod config;
mod connection;
mod messages;
mod node;
mod raw_transaction;
mod serialized_blocks;
mod utility;
mod merkle_tree;
mod utxoset;
fn main() -> Result<(), io::Error> {
    let (mut nodes, _mpsc_reader) = connection::connect_to_network()?;
    let mut utxo_set = utxoset::UTXOset::new();
    connection::sync(&mut nodes, &mut utxo_set)?;
    println!("UTXOset has {} transactions", utxo_set.utxo_vector.len());
    Ok(())
}
