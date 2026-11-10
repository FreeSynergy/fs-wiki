//! Reading the pages of a list of texts 0.2 and cutting it at a place.
//!
//! Pattern: Iterator — paging by hand (the pages are fetched by async calls), as `pages.rs`; the cursors of the contract
//! are opaque, so a list that continues after a place is read from the start and cut there.

use std::future::Future;

use fs_wiki_core::Result;

/// Items page after page until `size` are there or the cursor is empty; `more` says whether something follows.
pub(crate) async fn collect<T, F, Fut>(size: usize, mut fetch: F) -> Result<(Vec<T>, bool)>
where
    F: FnMut(Vec<u8>) -> Fut,
    Fut: Future<Output = Result<(Vec<T>, Vec<u8>)>>,
{
    let (mut items, mut cursor) = (Vec::new(), Vec::new());
    loop {
        let (page, next) = fetch(cursor).await?;
        let empty = page.is_empty();
        items.extend(page);
        cursor = next;
        if cursor.is_empty() || empty || items.len() > size {
            break;
        }
    }
    let more = items.len() > size || !cursor.is_empty();
    items.truncate(size);
    Ok((items, more))
}

/// The items after the one `is_place` finds (cut at a place, as the contract's cursors do on the server).
pub(crate) fn drop_through<T>(items: Vec<T>, is_place: impl Fn(&T) -> bool) -> Vec<T> {
    match items.iter().position(is_place) {
        Some(i) => items.into_iter().skip(i + 1).collect(),
        None => Vec::new(),
    }
}

/// How many items a list that continues "after" a place reads at most before it cuts there.
pub(crate) const READ_LIMIT: usize = 5000;

/// The items after the place (if one is given) and at most `size` of them; `more` says whether items follow, in this
/// list or because the server had more than [`READ_LIMIT`].
pub(crate) fn cut<T>(
    items: Vec<T>,
    server_more: bool,
    after: Option<impl Fn(&T) -> bool>,
    size: usize,
) -> (Vec<T>, bool) {
    let mut items = match after {
        Some(is_place) => drop_through(items, is_place),
        None => items,
    };
    let more = items.len() > size || server_more;
    items.truncate(size);
    (items, more)
}
