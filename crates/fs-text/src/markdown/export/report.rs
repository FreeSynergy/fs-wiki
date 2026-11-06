//! What an export could not say exactly, and the proof that the rest is exact.
//!
//! Pattern: Anchored comparison — the written text is read back and laid against the model.
//! Runs of blocks written from their kept source are anchors: they must read back as they are.
//! Between anchors the blocks written anew may read back as anything (that is reported, not
//! spread). If an anchor itself reads differently, the neighbours' meaning would change.

use crate::doc::{same_in_file, why_differs, Block};
use crate::id::Uuid;

/// One block that does not read back exactly as the model says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Inexact {
    /// The block.
    pub block: Uuid,
    /// Why, in words for a person.
    pub why: String,
}

/// What an export could not say exactly. Empty: the text reads back as the model.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ExportReport {
    /// Blocks written in their nearest form because no form reads back exactly (an edge blank
    /// or newline in a paragraph, a link text with markup inside a wiki link, text that an open
    /// HTML block swallows, …). Their untouched neighbours are written as they were.
    pub inexact: Vec<Inexact>,
    /// Untouched blocks that were written anew because their kept text would have read
    /// differently after the change (a removed definition, a block that lost its container).
    pub rewritten: Vec<Uuid>,
}

/// The result of comparing a written pass with the model.
pub(super) struct Analysis {
    /// `(block index, why)` of blocks that do not read back exactly.
    pub inexact: Vec<(usize, String)>,
    /// A block written from its kept source reads differently now: writing the neighbours anew
    /// may fix that.
    pub neighbours_changed: bool,
}

fn same_run(a: &[Block], b: &[Block]) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .all(|(x, y)| x.text == y.text && x.attrs.kind == y.attrs.kind)
        && same_in_file(a, b)
}

/// End of the run of blocks with the same `verbatim` flag that starts at `from`.
fn run_end(verbatim: &[bool], from: usize) -> usize {
    let flag = verbatim[from];
    from + verbatim[from..].iter().take_while(|v| **v == flag).count()
}

fn run_report(model: &[Block], back: &[Block], first: usize, out: &mut Vec<(usize, String)>) {
    if model.len() == back.len() {
        for (k, why) in why_differs(model, back).into_iter().enumerate() {
            if let Some(why) = why {
                out.push((first + k, why.to_string()));
            }
        }
    } else {
        let why = format!(
            "it reads back as {} block(s) instead of {}",
            back.len(),
            model.len()
        );
        out.extend((first..first + model.len()).map(|i| (i, why.clone())));
    }
}

/// The anchor run `model[from..to]` is not where it should be.
fn anchor_lost(out: &mut Analysis, from: usize, to: usize) {
    out.neighbours_changed = true;
    let why = "it reads differently because of the block before it".to_string();
    out.inexact.extend((from..to).map(|at| (at, why.clone())));
}

/// Where in `back[from..]` the anchor run `model[start..stop]` is.
fn find_anchor(
    model: &[Block],
    back: &[Block],
    from: usize,
    (start, stop): (usize, usize),
) -> Option<usize> {
    let len = stop - start;
    (from..=back.len().saturating_sub(len))
        .find(|&at| at + len <= back.len() && same_run(&model[start..stop], &back[at..at + len]))
}

/// Lay the read-back `back` against the model; `verbatim[i]` says block `i` was written from its
/// kept source.
pub(super) fn analyse(model: &[Block], back: &[Block], verbatim: &[bool]) -> Analysis {
    let total = model.len();
    let mut out = Analysis {
        inexact: Vec::new(),
        neighbours_changed: false,
    };
    let (mut at, mut ptr) = (0, 0);
    while at < total {
        let stop = run_end(verbatim, at);
        if verbatim[at] {
            // an anchor: must read back as it is
            let len = stop - at;
            if ptr + len <= back.len() && same_run(&model[at..stop], &back[ptr..ptr + len]) {
                ptr += len;
                at = stop;
                continue;
            }
            anchor_lost(&mut out, at, stop);
            return out;
        }
        // written anew: whatever it reads back as lies up to the next anchor
        let back_end = if stop == total {
            back.len()
        } else {
            let anchor_stop = run_end(verbatim, stop);
            if let Some(found) = find_anchor(model, back, ptr, (stop, anchor_stop)) {
                found
            } else {
                anchor_lost(&mut out, stop, anchor_stop);
                run_report(
                    &model[at..stop],
                    &back[ptr.min(back.len())..],
                    at,
                    &mut out.inexact,
                );
                return out;
            }
        };
        let run = &back[ptr.min(back_end)..back_end];
        if !same_run(&model[at..stop], run) {
            run_report(&model[at..stop], run, at, &mut out.inexact);
        }
        ptr = back_end;
        at = stop;
    }
    // the runs agree one by one; do they also agree as a whole (which list or quote a block is
    // in is a question between neighbours)?
    if out.inexact.is_empty() && back.len() == total && !same_in_file(model, back) {
        for (block, why) in why_differs(model, back).into_iter().enumerate() {
            if let Some(why) = why {
                out.neighbours_changed |= verbatim[block];
                out.inexact.push((block, why.to_string()));
            }
        }
    }
    out
}

/// The report for the blocks.
pub(super) fn report_of(blocks: &[Block], a: &Analysis) -> ExportReport {
    ExportReport {
        inexact: a
            .inexact
            .iter()
            .map(|(i, why)| Inexact {
                block: blocks[*i].id,
                why: why.clone(),
            })
            .collect(),
        rewritten: Vec::new(),
    }
}
