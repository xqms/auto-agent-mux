use clap::Parser;
use glob::glob;
use service_binding::Binding;
use ssh_agent_lib::agent::{listen, Session};
use ssh_agent_lib::client::connect;
use ssh_agent_lib::error::AgentError;
use ssh_agent_lib::proto::{Identity, SignRequest};
use ssh_key::Signature;
use std::path::PathBuf;
use std::os::unix::fs::PermissionsExt;
use tokio::net::UnixListener as Listener;

#[derive(Default, Clone)]
struct MyAgent;

pub fn find_agents() -> Vec<Binding> {
    let potential = glob("/tmp/ssh-*/agent.*").expect("Failed to glob()");

    potential
        .filter_map(|file| Some(Binding::FilePath(file.ok()?)))
        .collect()
}

#[ssh_agent_lib::async_trait]
impl Session for MyAgent {
    async fn request_identities(&mut self) -> Result<Vec<Identity>, AgentError> {
        let mut identities: Vec<Identity> = Vec::new();

        for binding in find_agents() {
            let bind_dbg = format!("{binding:?}");
            let Ok(stream) = binding.try_into() else {
                println!("Could not connect to {bind_dbg}");
                continue;
            };
            let Ok(mut client) = connect(stream) else {
                println!("Could not connect to {bind_dbg}");
                continue;
            };

            for new_identity in client.request_identities().await.unwrap_or(vec![]) {
                if identities
                    .iter()
                    .find(|ident| ident.pubkey == new_identity.pubkey)
                    .is_none()
                {
                    identities.push(new_identity);
                }
            }
        }

        Ok(identities)
    }

    async fn sign(&mut self, request: SignRequest) -> Result<Signature, AgentError> {
        for binding in find_agents() {
            let bind_dbg = format!("{binding:?}");

            println!("Trying to sign using agent {bind_dbg}");

            let Ok(stream) = binding.try_into() else {
                println!("Could not connect to {bind_dbg}");
                continue;
            };
            let Ok(mut client) = connect(stream) else {
                println!("Could not connect to {bind_dbg}");
                continue;
            };

            if let Ok(sig) = client.sign(request.clone()).await {
                return Ok(sig);
            }
        }

        Err(AgentError::Failure)
    }
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(short, long, value_name = "DIR")]
    socket_dir: PathBuf,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let _ = std::fs::create_dir(cli.socket_dir.clone());

    std::fs::set_permissions(cli.socket_dir.clone(), std::fs::Permissions::from_mode(0o700))?;

    let socket = cli.socket_dir.join("agent.sock");

    let _ = std::fs::remove_file(socket.clone()); // remove the socket if exists

    let listener = Listener::bind(socket.clone())?;

    std::fs::set_permissions(listener.local_addr()?.as_pathname().unwrap(), std::fs::Permissions::from_mode(0o600))?;

    tokio::select! {
        _ = listen(listener, MyAgent::default()) => {}
        _ = tokio::signal::ctrl_c() => {}
    }

    let _ = std::fs::remove_file(socket.clone()); // remove the socket if exists

    Ok(())
}
