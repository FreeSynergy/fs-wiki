//! The order of siblings.
//!
//! Pattern: Repository — one record per parent (type `WikiOrder`, id derived from the parent's id)
//! holds the ids of the children that were placed, in order. A child not in it comes after them, in
//! creation order (ids are time ordered). The order is state of the provider, not of the document.

use fs_objects::{Envelope, ObjectId, StoredObject};
use serde::{Deserialize, Serialize};

use super::handle::WikiRepo;
use super::ids::order_id;
use crate::error::Result;

const ORDER_TYPE: &str = "WikiOrder";
const ORDER_CONTEXT: &str = "wiki-order";

#[derive(Serialize, Deserialize)]
struct Order {
    ids: Vec<ObjectId>,
}

/// `all` in the order of `placed`, then the rest by id.
pub(super) fn arranged(all: Vec<ObjectId>, placed: &[ObjectId]) -> Vec<ObjectId> {
    let mut out: Vec<ObjectId> = placed.iter().copied().filter(|p| all.contains(p)).collect();
    let mut rest: Vec<ObjectId> = all.into_iter().filter(|a| !out.contains(a)).collect();
    rest.sort();
    out.extend(rest);
    out
}

impl WikiRepo {
    /// The placed children of `parent`, in order (none placed: empty).
    pub(super) async fn order_get(&self, parent: Option<ObjectId>) -> Result<Vec<ObjectId>> {
        match self.objects.get(order_id(parent)).await? {
            Some(o) => Ok(serde_json::from_value::<Order>(o.content)?.ids),
            None => Ok(vec![]),
        }
    }

    /// Say that exactly `ids` are placed under `parent`, in this order.
    pub(super) async fn order_put(
        &self,
        parent: Option<ObjectId>,
        ids: Vec<ObjectId>,
    ) -> Result<()> {
        let mut env = Envelope::new(ORDER_TYPE, "order");
        env.id = order_id(parent);
        env.context = Some(ORDER_CONTEXT.into());
        let object = StoredObject::new(
            env,
            "application/json",
            serde_json::to_value(Order { ids })?,
        );
        self.objects.put(object, &self.aux_actor()).await?;
        Ok(())
    }

    /// Forget the order under a folder that is gone.
    pub(super) async fn order_drop(&self, parent: Option<ObjectId>) -> Result<()> {
        if self.objects.get(order_id(parent)).await?.is_some() {
            self.objects
                .delete(order_id(parent), &self.aux_actor())
                .await?;
        }
        Ok(())
    }
}
