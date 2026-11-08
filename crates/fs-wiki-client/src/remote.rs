//! What the client takes from the root of the texts contract.
//!
//! Pattern: Strategy — the one part of the connection that is the wiki's own: the root capability
//! (`Texts` holds every method itself) and the handshake. The thread, the queue and the deadlines are
//! the shared `Worker` of `fs-interfaces`.

use fs_interfaces::rpc::client::Remote as Contract;
use fs_interfaces::rpc::read_uuid;
use fs_interfaces::texts_capnp::texts;
use fs_interfaces::Capabilities;
use fs_objects::StoreId;

/// The capability of the server a call may use.
#[derive(Clone)]
pub(crate) struct Remote {
    pub(crate) texts: texts::Client,
    /// The id of the store behind the server, as its report said (none: it has no store).
    pub(crate) identity: Option<StoreId>,
}

impl Contract for Remote {
    type Root = texts::Client;

    async fn open(root: texts::Client) -> Result<(Self, Capabilities), capnp::Error> {
        let report = root.support_report_request().send().promise.await?;
        let report = report.get()?.get_report()?;
        let capabilities = Capabilities::read(report)?;
        // What is not a store id (not a UUIDv4) is no identity: better none than a wrong one.
        let identity = read_uuid(report.get_instance()?).and_then(|u| u.to_string().parse().ok());
        Ok((
            Self {
                texts: root,
                identity,
            },
            capabilities,
        ))
    }
}
