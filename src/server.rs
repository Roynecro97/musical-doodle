use log::info;

use crate::cmdline::Server;

pub(crate) fn main(command: Server, address: String, port: u16) -> color_eyre::eyre::Result<()> {
    info!("running {:?} as server on {}:{}", command, address, port);
    Ok(())
}
