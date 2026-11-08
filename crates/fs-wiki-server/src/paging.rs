//! A page of texts on the wire.
//!
//! Pattern: Adapter — a [`TextPage`] from the provider becomes a `TextPage` of `texts.capnp`, cut by
//! weight so the answer always fits a message, with the cursor of the place the page ends at. The
//! page was cut in the provider's store; only the texts of the window are written as JSON here.

use fs_interfaces::rpc::{Problem, MAX_ITEM_BYTES, MAX_PAGE_BYTES, TOO_LARGE};
use fs_interfaces::texts_capnp::text_page;
use fs_wiki_core::{Place, TextPage, TextQuery, Texts};

use crate::cursor::{decode, encode};
use crate::errors::rpc_error;
use crate::wire::{count, fill, Sent};

/// How many of the items go into this answer: all, or fewer if they weigh more than a message
/// carries. An item that cannot be sent even alone is a `too-large` problem whose `next` is the
/// cursor past it (none if it was the last), so what is behind it stays readable.
fn fitting(items: &[(Sent, Place)], more: bool) -> capnp::Result<usize> {
    let (mut taken, mut bytes) = (0usize, 0usize);
    for (i, (sent, place)) in items.iter().enumerate() {
        let heavy = sent.weight();
        bytes = bytes.saturating_add(heavy);
        let full = bytes > MAX_PAGE_BYTES;
        if full && taken > 0 {
            break;
        }
        if heavy > MAX_ITEM_BYTES {
            let later = i + 1 < items.len() || more;
            let detail = format!("one item of {heavy} bytes, at most {MAX_ITEM_BYTES} fit");
            // With a cursor no extensions: the numbers are in the detail anyway.
            let problem = if later {
                Problem::new(TOO_LARGE, "Too large", detail, 413)
            } else {
                Problem::too_large(detail, heavy as u64, MAX_ITEM_BYTES as u64)
            };
            return Err(problem.with_next(later.then(|| encode(place))).into_error());
        }
        taken += 1;
        if full {
            break;
        }
    }
    Ok(taken)
}

/// The page of `query` that `cursor` asks for, written to `out`.
pub(crate) async fn write_page(
    texts: &dyn Texts,
    query: &TextQuery,
    cursor: &[u8],
    size: usize,
    mut out: text_page::Builder<'_>,
) -> capnp::Result<()> {
    let after = decode(cursor)?;
    let page: TextPage = texts
        .page(query, after.as_ref(), size)
        .await
        .map_err(rpc_error)?;
    let more = page.more;
    let items = page
        .items
        .into_iter()
        .map(|p| Sent::new(p.entry).map(|sent| (sent, p.place)))
        .collect::<capnp::Result<Vec<_>>>()?;
    let taken = fitting(&items, more)?;
    let mut written = out.reborrow().init_items(count(taken)?);
    for (i, (sent, _)) in (0u32..).zip(&items[..taken]) {
        fill(&mut written.reborrow().get(i), sent);
    }
    let next = match items[..taken].last() {
        Some((_, last)) if taken < items.len() || more => encode(last),
        _ => Vec::new(),
    };
    out.init_next().set_opaque(&next);
    Ok(())
}
