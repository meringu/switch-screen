use std::{fmt, thread::sleep, time::Duration};

use anyhow::{bail, Result};
use clap::{Parser, Subcommand, ValueEnum};
use rumqttc::{Event, MqttOptions, Packet, QoS};
use windows::Win32::Devices::Display::{self, SetDisplayConfig};

#[derive(Debug, Parser)]
#[command(about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    INTERNAL,
    EXTERNAL,
    CLONE,
    EXTEND,
    SUPPLIED,
    /// Subscribe to an MQTT endpoint. Set the target topology as the payload to trigger a switch event.
    MQTT {
        /// Broker host
        #[arg(long, default_value = "localhost")]
        host: String,
        /// Broker port
        #[arg(long, default_value = "1883")]
        port: u16,
        /// Client ID
        #[arg(long, default_value = "screen-switch")]
        id: String,
        /// Topic to subscribe to
        #[arg(long, default_value = "screen-switch")]
        topic: String,
        /// Topic to publish to on successful switch
        #[arg(long, default_value = "screen-switch-status")]
        status_topic: String,

        /// Authentication username
        #[arg(long)]
        username: Option<String>,
        /// Authentication password
        #[arg(long)]
        password: Option<String>,
    },
}

#[derive(Debug, Clone, ValueEnum)]
enum Topology {
    INTERNAL,
    EXTERNAL,
    CLONE,
    EXTEND,
    SUPPLIED,
}

impl Topology {
    fn to_flags(&self) -> Display::SET_DISPLAY_CONFIG_FLAGS {
        match self {
            Self::INTERNAL => Display::SDC_TOPOLOGY_INTERNAL,
            Self::EXTERNAL => Display::SDC_TOPOLOGY_EXTERNAL,
            Self::CLONE => Display::SDC_TOPOLOGY_CLONE,
            Self::EXTEND => Display::SDC_TOPOLOGY_EXTEND,
            Self::SUPPLIED => Display::SDC_TOPOLOGY_SUPPLIED,
        }
    }
}

impl fmt::Display for Topology {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Topology::INTERNAL => write!(f, "internal"),
            Topology::EXTERNAL => write!(f, "external"),
            Topology::CLONE => write!(f, "clone"),
            Topology::EXTEND => write!(f, "extend"),
            Topology::SUPPLIED => write!(f, "supplied"),
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::INTERNAL => switch(&Topology::INTERNAL),
        Commands::EXTERNAL => switch(&Topology::EXTERNAL),
        Commands::CLONE => switch(&Topology::CLONE),
        Commands::EXTEND => switch(&Topology::EXTEND),
        Commands::SUPPLIED => switch(&Topology::SUPPLIED),
        Commands::MQTT {
            host,
            port,
            id,
            username,
            password,
            topic,
            status_topic,
        } => {
            let mut options = rumqttc::MqttOptions::new(id, host, port);
            if let (Some(u), Some(p)) = (username, password) {
                options.set_credentials(u, p);
            }

            subscribe(options, &topic, &status_topic)
        }
    }
}

fn switch(topology: &Topology) -> Result<()> {
    println!("Switching to {}", topology);

    let res = unsafe { SetDisplayConfig(None, None, Display::SDC_APPLY | topology.to_flags()) };

    if res != 0 {
        bail!("An error occcured swtiching displays");
    }

    Ok(())
}

fn subscribe(options: MqttOptions, topic: &str, status_topic: &str) -> Result<()> {
    'outer: loop {
        let (mut client, mut connection) = rumqttc::Client::new(options.clone(), 10);

        println!("Subscribing to {}", topic);
        if let Err(e) = client.subscribe(topic, QoS::AtMostOnce) {
            println!(
                "error connecting to server: {}. reconnecting in 10 seconds",
                e
            );
            sleep(Duration::from_secs(10));
            continue;
        }

        for res in connection.iter() {
            let event = match res {
                Ok(e) => e,
                Err(e) => {
                    println!(
                        "error reading event from server: {}. reconnecting in 10 seconds",
                        e
                    );
                    sleep(Duration::from_secs(10));
                    continue 'outer;
                }
            };

            let payload = if let Event::Incoming(incoming) = event {
                if let Packet::Publish(publish) = incoming {
                    match String::from_utf8(publish.payload.to_vec()) {
                        Ok(s) => s,
                        Err(e) => {
                            println!("failed to decode message: {}", e);
                            continue;
                        }
                    }
                } else {
                    continue;
                }
            } else {
                continue;
            };

            match Topology::from_str(&payload, true) {
                Ok(topology) => {
                    switch(&topology)?;
                    client.publish(status_topic, QoS::AtMostOnce, true, topology.to_string())?
                }
                Err(topology) => println!("unexpected topology: {}", topology),
            };
        }

        println!("disconnected from server. reconnecting in 10 seconds");
        sleep(Duration::from_secs(10));
    }
}
