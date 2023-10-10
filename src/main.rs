pub(crate) mod client;
pub(crate) mod cmdline;
pub mod common;
pub(crate) mod server;

use log::{info, debug};
use structopt::StructOpt;

#[cfg(target_feature = "play-single-file")]
use serde_derive::{Deserialize, Serialize};
#[cfg(target_feature = "play-single-file")]
use structopt::clap::AppSettings;

// #[derive(Debug, Error)]  // Error from thiserror
// enum MainError {
//     InvalidLoggingLevel,
// }

// impl std::fmt::Display for MainError {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         std::fmt::Debug::fmt(self, f)
//     }
// }

#[cfg(target_feature = "play-single-file")]
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

pub fn get_version() -> &'static str {
    const VERSION: &str = git_version::git_version!();

    #[cfg(debug_assertions)]
    const EXTRA: &str = ", DEBUG BUILD";
    #[cfg(not(debug_assertions))]
    const EXTRA: &str = "";

    lazy_static::lazy_static! {
        static ref FULL_VERSION: String = format!("magical-doodle {}{}", VERSION, EXTRA);
    }

    return FULL_VERSION.as_str();
}

pub fn os_string() -> String {
    match (sys_info::os_release().ok(), sys_info::os_type().ok()) {
        (Some(release), Some(os_type)) =>
            format!("{}, kernel-ver {}", os_type, release),
        _ => format!("Unknown"),
    }
}

#[cfg(not(target_feature = "lol"))]
fn logger_init(opt: &cmdline::Opt) -> color_eyre::eyre::Result<()> {
    use std::fs::File;
    use time::macros::format_description;
    use simplelog::{ConfigBuilder, LevelFilter, TermLogger, ThreadLogMode, TerminalMode, ColorChoice, WriteLogger, CombinedLogger, SharedLogger};

    let config = ConfigBuilder::new()
        .set_location_level(LevelFilter::Error)
        .set_target_level(LevelFilter::Error)
        .set_thread_level(LevelFilter::Error)
        .set_thread_mode(ThreadLogMode::Names)
        .set_time_format_custom(
            format_description!("[year]-[month]-[day] [hour]:[minute]:[second].[subsecond]"))  // "%Y-%m-%d %H:%M:%S.%6f"
        .build();

    let log_level = opt.log_level.into();

    let mut loggers: Vec<Box<(dyn SharedLogger + 'static)>> = Vec::with_capacity(2);

    if !opt.quiet {
        loggers.push(
            TermLogger::new(log_level, config.clone(), TerminalMode::Stderr, ColorChoice::Auto)
        )
    }

    if let Some(path) = &opt.logfile {
        loggers.push(
            WriteLogger::new(log_level, config, File::create(path)?)
        )
    }

    Ok(CombinedLogger::init(loggers)?)
}

// #[cfg(not(target_feature="play-single-file"))]
fn main() -> color_eyre::eyre::Result<()> {
    color_eyre::install()?;

    let opt = cmdline::Opt::from_args();

    logger_init(&opt)?;

    match opt.command {
        cmdline::Command::Server(command) => {
            info!("Starting server ({}), PID {}", get_version(), std::process::id());
            info!("Running on OS: {}", os_string());
            server::main(command, opt.server_address, opt.server_port)
        },
        cmdline::Command::Client(command) => {
            debug!("Starting client ({}), PID {}", get_version(), std::process::id());
            client::main(command, opt.server_address, opt.server_port)
        },
    }
}

#[cfg(target_feature="play-single-file")]
fn main() -> color_eyre::eyre::Result<()> {
    use std::{fs::File, io::BufReader};
    use rodio::{Decoder, OutputStream, Sink};

    color_eyre::install()?;

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

    Ok(())
}
