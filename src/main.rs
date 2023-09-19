#![allow(unused_imports)]
pub(crate) mod client;
pub(crate) mod cmdline;
pub mod common;
pub(crate) mod server;

use std::{fs::File, io::BufReader};

use rodio::{Decoder, OutputStream, Sink};
use serde_derive::{Deserialize, Serialize};
use structopt::{clap::AppSettings, StructOpt};

#[derive(Debug, Clone, Serialize, Deserialize, StructOpt)]
#[structopt(
    // name = "",  // leaves 2 blank lines at the start
    // no_version,
    about = "Remote music player",
    global_settings = &[AppSettings::ColoredHelp, AppSettings::VersionlessSubcommands, AppSettings::DeriveDisplayOrder],
)]
struct Opt {
    /// The file to play
    soundfile: String,
    /// The amount of times to repeat the file
    #[structopt(default_value = "1")]
    times: u32,
}

fn main() -> color_eyre::eyre::Result<()> {
    color_eyre::install()?;

    #[cfg(not(target_feature="play-single-file"))]
    {
        let opt = cmdline::Opt::from_args();
        println!("{:?}", opt);
        println!("{}", opt.log_level);
    }

    #[cfg(target_feature="play-single-file")]
    {
        let opt = Opt::from_args();
        println!("Hello, world!");
        println!("My configuration is {:?}", opt);

        use rodio::cpal::traits::{DeviceTrait, HostTrait};
        let host = rodio::cpal::default_host();
        println!("default: {:?}", host.default_output_device() .map(|d| d.name().unwrap_or("missing name".to_owned())));
        for (i, device) in host.output_devices()?.enumerate() {
            println!("{}: {}", i, device.name().unwrap_or("missing name".to_owned()));
        }

        let (_stream, stream_handle) = OutputStream::try_default()?;
        let sink = Sink::try_new(&stream_handle)?;
        sink.set_volume(0.1);

        println!("paused: {}", sink.is_paused());
        println!("volume: {}", sink.volume());
        println!("speed: {}", sink.speed());

        for _ in 0..opt.times {
            sink.append(Decoder::new(BufReader::new(File::open(&opt.soundfile)?))?);
            sink.play();
            println!("playing...");
            sink.sleep_until_end();
        }
    }

    Ok(())
}
