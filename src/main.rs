mod connection;

fn main() -> Result<(), String> {
    connection::connect_to_network()?;
    Ok(())
}
