use log::info;

use crate::cmdline::Client;

pub(crate) fn main(command: Client, server_address: String, server_port: u16) -> color_eyre::eyre::Result<()> {
    info!("running {:?} with server {}:{}", command, server_address, server_port);
    Ok(())
}
