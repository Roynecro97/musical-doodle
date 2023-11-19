use std::{path::PathBuf, str::FromStr};

use structopt::{clap::AppSettings, StructOpt};

#[derive(Debug, StructOpt)]
pub enum Music {
    Song {
        #[structopt(required = true, min_values = 1)]
        songs: Vec<String>,
    },
    Playlist {
        playlist: String,
    },
    AllSongs,
}

#[derive(Debug, StructOpt)]
pub struct Play {
    #[structopt(long)]
    pub shuffled: bool,

    #[structopt(long)]
    pub repeat: bool,

    #[structopt(subcommand)]
    pub command: Option<Music>,
}

#[derive(Debug, StructOpt)]
pub struct Queue {
    #[structopt(long)]
    pub shuffled: bool,

    #[structopt(subcommand)]
    pub command: Music,
}

#[derive(Debug, StructOpt)]
pub enum ClientCommand {
    /// TODO: add docs
    Play(Play),

    /// Add to music queue
    Queue(Queue),

    /// Pause currently playing music
    Pause,

    /// Query the server for the currently playing song
    Status,

    /// Tell the server to exit
    Shutdown,
}

#[derive(Debug, StructOpt)]
pub struct Client {
    #[structopt(subcommand)]
    pub command: ClientCommand,
}

#[derive(Debug, StructOpt)]
pub struct Server {
    /// Music library path
    pub path: PathBuf,
}

#[derive(Debug, StructOpt)]
pub enum Command {
    Client(Client),
    Server(Server),
}

#[derive(Debug, Clone, Copy)]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
    Critical,
}

impl FromStr for LogLevel {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "debug" => Ok(Self::Debug),
            "info" => Ok(Self::Info),
            "warning" => Ok(Self::Warning),
            "error" => Ok(Self::Error),
            "critical" => Ok(Self::Critical),
            _ => Err("valid values: debug, info, warning, error, critical"),
        }
    }
}

impl AsRef<str> for LogLevel {
    fn as_ref(&self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
            Self::Critical => "critical",
        }
    }
}

impl From<LogLevel> for log::LevelFilter {
    fn from(value: LogLevel) -> Self {
        match value {
            LogLevel::Debug => Self::Debug,
            LogLevel::Info => Self::Info,
            LogLevel::Warning => Self::Warn,
            LogLevel::Error => Self::Error,
            LogLevel::Critical => Self::Error,
        }
    }
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_ref())
    }
}

#[derive(Debug, StructOpt)]
#[structopt(
    about = "Remote music player",
    global_settings = &[
        AppSettings::ColoredHelp,
        AppSettings::VersionlessSubcommands,
        AppSettings::DeriveDisplayOrder,
        AppSettings::DisableHelpSubcommand,
    ],
)]
pub struct Opt {
    /// The path to the logfile
    #[structopt(long)]
    pub logfile: Option<PathBuf>,

    /// Suppress stdout/stderr log output (logfile is still written to if provided)
    #[structopt(short, long)]
    pub quiet: bool,

    /// Easy override of log level from the command line.
    /// The value values are: debug, info, warning, critical.
    #[structopt(long, default_value = "info")]
    pub log_level: LogLevel,

    /// When running as a server, this is the adddress to listen on.
    /// The server will listen on all interfaces if not specified.
    ///
    /// When running as client, this is the server address of the Coordinator
    /// to connect to.
    #[structopt(short = "a", long, default_value = "0.0.0.0")]
    pub server_address: String,

    /// When running as a server, this is the port to listen to.
    /// Will be picked at random and printed if not specified.
    ///
    /// When running as a client, this is the port to use to connect.
    #[structopt(short = "p", long, default_value = "31415")]
    pub server_port: u16,

    #[structopt(subcommand)]
    pub command: Command,
}
