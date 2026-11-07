//! What the client takes from the root of the texts contract.
//!
//! Pattern: Strategy — the one part of the connection that is the wiki's own: the root capability
//! (`Texts` holds every method itself) and the handshake. The thread, the queue and the deadlines are
//! the shared `Worker` of `fs-interfaces`.

use fs_interfaces::rpc::client::Remote as Contract;
use fs_interfaces::texts_capnp::texts;
use fs_interfaces::Capabilities;

/// The capability of the server a call may use.
#[derive(Clone)]
pub(crate) struct Remote {
    pub(crate) texts: texts::Client,
}

impl Contract for Remote {
    type Root = texts::Client;

    async fn open(root: texts::Client) -> Result<(Self, Capabilities), capnp::Error> {
        let report = root.support_report_request().send().promise.await?;
        let capabilities = Capabilities::read(report.get()?.get_report()?)?;
        Ok((Self { texts: root }, capabilities))
    }
}
