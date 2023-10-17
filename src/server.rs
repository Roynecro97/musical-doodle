use log::info;

use crate::{cmdline::Server, common::Address};

pub(crate) fn main(command: Server, address: Address) -> color_eyre::eyre::Result<()> {
    info!("running {:?} as server on {}", command, address);
    Ok(())
}
