use std::io;

mod top_bar;

pub fn init(builder: gtk::Builder) -> io::Result<()> {
    top_bar::init(builder)?;

    Ok(())
}