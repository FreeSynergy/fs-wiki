//! Asking the server which of the recorded texts it has.
//!
//! Pattern: none — one function; the reads run many at once and are reduced to a yes or no.

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use fs_store::ObjectId;
use fs_wiki_core::{FileOrigin, Texts};
use futures::StreamExt;

use crate::source::Failure;
use crate::words::{self, say};

/// How many reads of texts are on their way at once when the records are checked.
const CHECK_IN_FLIGHT: usize = 64;

/// The ids of the records whose texts the server has, and the files whose text could not be asked
/// about. The contract has no cheap way to ask (a listing carries every document and took 5.5 s for
/// 1398 pages), so each record is one `get`, many at once on the one connection; each answer is
/// reduced to a `bool` at once, so no document is kept. Nothing is asked when no record exists (a
/// first start).
///
/// One `get` that fails does not stop the start: that text is taken to be there (as the records say)
/// and its file is reported. The whole check stops with a message when it runs past `limit`, or when
/// every `get` failed (the server is not there).
pub(crate) async fn present_ids<W: Texts + ?Sized>(
    wiki: &W,
    known: &HashMap<String, FileOrigin>,
    limit: Duration,
) -> Result<(HashSet<ObjectId>, Vec<Failure>), String> {
    let asked: Vec<(String, ObjectId)> = known.values().map(|o| (o.file.clone(), o.text)).collect();
    let reads = futures::stream::iter(asked)
        .map(|(file, id)| async move {
            let there = wiki.get(id).await.map(|text| text.is_some());
            (file, id, there)
        })
        .buffer_unordered(CHECK_IN_FLIGHT)
        .collect::<Vec<_>>();
    let Ok(answers) = tokio::time::timeout(limit, reads).await else {
        return Err(say(words::CHECK_TIMEOUT, &[("seconds", &limit.as_secs())]));
    };
    let mut present = HashSet::new();
    let mut unasked = Vec::new();
    let mut last_error = None;
    for (file, id, there) in answers {
        match there {
            Ok(true) => {
                present.insert(id);
            }
            Ok(false) => {}
            Err(e) => {
                present.insert(id);
                unasked.push(Failure {
                    path: file,
                    reason: say(words::CHECK_ONE_FAILED, &[("error", &e)]),
                });
                last_error = Some(e);
            }
        }
    }
    match last_error {
        Some(e) if unasked.len() == known.len() => Err(say(words::CHECK_FAILED, &[("error", &e)])),
        _ => Ok((present, unasked)),
    }
}
