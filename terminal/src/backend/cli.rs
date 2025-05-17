use clap::Parser;
use clap::ValueEnum;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Whether to start or stop the terrazzo-terminal daemon.
    #[arg(long, short, value_enum, default_value_t = Action::Run)]
    pub action: Action,

    /// The TCP host to listen to.
    #[arg(long)]
    pub host: Option<String>,

    /// The file to store the config.
    #[arg(long)]
    pub config_file: Option<String>,

    /// The TCP port to listen to.
    #[arg(long)]
    pub port: Option<u16>,

    /// The file to store the pid of the daemon while it is running.
    #[arg(long)]
    pub pidfile: Option<String>,

    /// The file to the store private Root CA.
    #[arg(long)]
    pub private_root_ca: Option<String>,

    /// If using mesh: the Client name.
    #[arg(long)]
    pub client_name: Option<String>,

    /// If using mesh: the Gateway endpoint
    #[arg(long)]
    pub gateway_url: Option<String>,

    /// If using mesh: the Gateway CA
    #[arg(long)]
    pub gateway_pki: Option<String>,

    /// If using mesh: the AuthCode to get a client certificate
    #[arg(long, default_value_t = String::default())]
    pub auth_code: String,

    /// If using mesh: the file to store the client certificate
    #[arg(long)]
    pub client_certificate: Option<String>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum Action {
    /// Run the server in the foreground
    Run,

    /// Run the server in the background as a daemon
    Start,

    /// Stop the daemon
    Stop,

    /// Sets the password
    SetPassword,
}
