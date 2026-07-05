//! Typed client SDK: a Session and one facade per domain. Front-ends drive the
//! daemon only through here — never by linking the engine.

mod facades;
mod session;
mod spawn;

pub use facades::{Accounts, App, Cache, Config, Daemon, Java, Process};
pub use ipc::errors::IpcError;
pub use session::{job_id, Session};

/// Re-export `proto` so front-ends need only depend on `client`.
pub use proto;

use std::path::Path;

/// A connection to the daemon plus the typed facades over it.
pub struct Client {
    session: Session,
}

impl Client {
    /// Connect to the daemon at the default endpoint, auto-spawning it if it is
    /// not already running and `auto_spawn` is set.
    pub async fn connect(auto_spawn: bool) -> Result<Client, IpcError> {
        let endpoint = ipc::endpoint::default_endpoint();
        match ipc::connect(&endpoint).await {
            Ok(conn) => Ok(Client {
                session: Session::new(conn),
            }),
            Err(_) if auto_spawn => {
                spawn::spawn_daemon()?;
                match spawn::connect_with_retry(&endpoint).await {
                    Some(conn) => Ok(Client {
                        session: Session::new(conn),
                    }),
                    None => Err(IpcError::ConnectionLost),
                }
            }
            Err(e) => Err(e),
        }
    }

    /// Connect to a daemon listening on `endpoint` (no auto-spawn).
    pub async fn connect_to(endpoint: &Path) -> Result<Client, IpcError> {
        let conn = ipc::connect(endpoint).await?;
        Ok(Client {
            session: Session::new(conn),
        })
    }

    pub fn session(&self) -> &Session {
        &self.session
    }

    pub fn app(&self) -> App<'_> {
        App {
            session: &self.session,
        }
    }

    pub fn daemon(&self) -> Daemon<'_> {
        Daemon {
            session: &self.session,
        }
    }

    pub fn config(&self) -> Config<'_> {
        Config {
            session: &self.session,
        }
    }

    pub fn cache(&self) -> Cache<'_> {
        Cache {
            session: &self.session,
        }
    }

    pub fn java(&self) -> Java<'_> {
        Java {
            session: &self.session,
        }
    }

    pub fn accounts(&self) -> Accounts<'_> {
        Accounts {
            session: &self.session,
        }
    }

    pub fn process(&self) -> Process<'_> {
        Process {
            session: &self.session,
        }
    }
}
