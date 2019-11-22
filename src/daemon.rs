//! Contains the Lucky RPC implementaiton used for client->daemon communication.

#[allow(clippy::all)]
#[allow(bare_trait_objects)]
/// The varlink RPC code ( generated by build.rs from `rpc/lucky.rpc.varlink` )
pub(crate) mod lucky_rpc;
pub(crate) use lucky_rpc as rpc;

use crate::config;
use crate::types::{ScriptState, ScriptStatus};

use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, RwLock,
};

#[derive(Default)]
/// The Lucky Daemon RPC service
struct LuckyDaemon {
    /// Used to indicate that the server should stop listening.
    /// This will be set to true to indicate that the server should stop.
    stop_listening: Arc<AtomicBool>,
    script_statuses: Arc<RwLock<HashMap<String, ScriptStatus>>>,
}

impl LuckyDaemon {
    /// Create a new daemon instance
    ///
    /// stop_listening will be set to `true` by the daemon if it recieves a StopDaemon RPC. The
    /// actual stopping of the server itself is not handled by the daemon.
    fn new(stop_listening: Arc<AtomicBool>) -> Self {
        LuckyDaemon {
            stop_listening,
            ..Default::default()
        }
    }

    /// Consolidate script statuses into one status that can be used as the global Juju Status
    fn get_juju_status(&self) -> ScriptStatus {
        // The resulting Juju state
        let mut juju_state = ScriptState::default();
        // The resulting Juju status message
        let mut juju_message = None;

        for status in self.script_statuses.read().unwrap().values() {
            // If this script state has a higher precedence
            if status.state > juju_state {
                // Set the Juju state to the more precedent state
                juju_state = status.state;
            }

            // If there is a message with the status
            if let Some(message) = &status.message {
                if let Some(current) = juju_message {
                    // Add this message to Juju message
                    juju_message = Some([current, message.clone()].join(", "));
                } else {
                    // Set Juju message to this message
                    juju_message = Some(message.clone());
                }
            }
        }

        // Return Juju status
        ScriptStatus {
            state: juju_state,
            message: juju_message,
        }
    }
}

impl rpc::VarlinkInterface for LuckyDaemon {
    /// Trigger a Juju hook
    fn trigger_hook(
        &self,
        call: &mut dyn rpc::Call_TriggerHook,
        hook_name: String,
    ) -> varlink::Result<()> {
        log::info!("Triggering hook: {}", hook_name);

        let charm_dir = match config::get_charm_dir() {
            Ok(charm_dir) => charm_dir,
            Err(e) => {
                log::error!("{}\n    Did not trigger hook: \"{}\"", e, hook_name);
                call.reply_os_error(e.to_string())?;
                return Ok(())
            }
        };
        
        println!("{:?}", charm_dir);

        // Reply and exit
        call.set_continues(true);
        call.reply(Some("Hello fello!".into()))?;
        call.reply(Some("Goodbye dude!".into()))?;
        call.set_continues(false);
        call.reply(None)?;
        Ok(())
    }

    /// Stop the Lucky daemon
    fn stop_daemon(&self, call: &mut dyn rpc::Call_StopDaemon) -> varlink::Result<()> {
        log::info!("Shutting down server");
        // Set the stop_listening=true.
        self.stop_listening.store(true, Ordering::SeqCst);

        // Reply and exit
        call.reply()?;
        Ok(())
    }

    /// Set a script's status
    fn set_status(
        &self,
        call: &mut dyn rpc::Call_SetStatus,
        script_id: String,
        status: rpc::ScriptStatus,
    ) -> varlink::Result<()> {
        // Add status to script statuses
        let status: ScriptStatus = status.into();
        log::info!(r#"Setting status for script "{}": {}"#, script_id, status);
        self.script_statuses
            .write()
            .unwrap()
            .insert(script_id, status);

        // Set the Juju status to the consolidated script statuses
        crate::juju::set_status(self.get_juju_status())
            .or_else(|e| call.reply_os_error(e.to_string()))?;

        // Reply
        call.reply()?;
        Ok(())
    }
}

//
// Helpers
//

/// Get the server service
pub(crate) fn get_service(stop_listening: Arc<AtomicBool>) -> varlink::VarlinkService {
    // Create a new daemon instance
    let daemon_instance = LuckyDaemon::new(stop_listening);

    // Return the varlink service
    varlink::VarlinkService::new(
        "lucky.rpc",
        "lucky daemon",
        clap::crate_version!(),
        "https://github.com/katharostech/lucky",
        vec![Box::new(lucky_rpc::new(Box::new(daemon_instance)))],
    )
}

/// Get the client
pub(crate) fn get_client(connection: Arc<RwLock<varlink::Connection>>) -> rpc::VarlinkClient {
    // Return the varlink client
    rpc::VarlinkClient::new(connection)
}
