//! The descriptions of the wiki's data types against the trait `DescribesType` and the commands the core and
//! `requirements.md` name.

use std::collections::BTreeSet;

use fs_types::{BasicType, CommandKind, DescribesType};
use fs_wiki_core::describe::{descriptions, registry, wished_commands, PROGRAM};
use fs_wiki_core::{LinkEntry, Node, RevisionInfo, TextEntry, TrashItem};

fn names(t: &fs_types::TypeDescription) -> BTreeSet<String> {
    t.commands.iter().map(|c| c.name.clone()).collect()
}

#[test]
fn every_type_is_valid_and_owned_by_the_wiki() {
    let all = descriptions().expect("valid");
    let got: Vec<&str> = all.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(got, ["Folder", "Link", "Page", "Revision", "TrashEntry"]);
    for t in &all {
        assert_eq!(t.program, PROGRAM);
        t.validate().expect("valid");
        assert_ne!(t.standard.len(), 0);
    }
}

#[test]
fn the_trait_gives_the_same_description_as_the_registry() {
    let reg = registry().unwrap();
    assert_eq!(reg.get("Page"), Some(&TextEntry::describe()));
    assert_eq!(reg.get("Folder"), Some(&Node::describe()));
    assert_eq!(reg.get("Revision"), Some(&RevisionInfo::describe()));
    assert_eq!(reg.get("TrashEntry"), Some(&TrashItem::describe()));
    assert_eq!(reg.get("Link"), Some(&LinkEntry::describe()));
}

#[test]
fn references_between_our_types_resolve_except_the_other_programs() {
    let open: Vec<_> = registry()
        .unwrap()
        .unresolved_targets()
        .into_iter()
        .map(|(_, _, t)| t)
        .collect();
    assert!(
        open.iter().all(|t| t == "File"),
        "only the files program is outside: {open:?}"
    );
}

#[test]
fn the_core_commands_are_all_described() {
    let page = names(&TextEntry::describe());
    for n in [
        "create",
        "import",
        "createFromTemplate",
        "change",
        "move",
        "setTemplate",
        "setRedirect",
        "trash",
        "delete",
        "find",
        "search",
        "resolve",
        "export",
    ] {
        assert!(page.contains(n), "Page.{n}");
    }
    let folder = names(&Node::describe());
    for n in ["find", "create", "rename", "move", "reorder", "delete"] {
        assert!(folder.contains(n), "Folder.{n}");
    }
    assert!(names(&RevisionInfo::describe())
        .is_superset(&["find", "get", "restore", "purge"].map(String::from).into()));
    assert!(names(&TrashItem::describe())
        .is_superset(&["find", "restore", "purge"].map(String::from).into()));
    assert!(names(&LinkEntry::describe()).contains("find"));
}

#[test]
fn every_row_of_requirements_is_a_command() {
    let page = names(&TextEntry::describe());
    // history/restore, trash/restore, sharing, folders/order, templates/redirects, search, comments, attachments,
    // labels, watch, then the optional ideas
    for n in [
        "share",
        "comment",
        "openTalkPage",
        "attach",
        "setLabels",
        "watch",
        "createDatabase",
        "joinSession",
        "recordGuide",
        "recordCommandGuide",
        "replayGuide",
        "embed",
        "applyTag",
        "tagTable",
        "referenceBlock",
        "createCanvas",
        "importCanvas",
        "exportCanvas",
    ] {
        assert!(page.contains(n), "Page.{n}");
    }
    assert!(names(&LinkEntry::describe()).contains("unlinkedMentions"));
    for (ty, n) in wished_commands() {
        let d = registry().unwrap().get(ty).cloned().unwrap();
        assert!(
            names(&d).contains(n),
            "{ty}.{n} is listed as wished but not described"
        );
    }
}

#[test]
fn a_change_or_delete_names_its_record() {
    // The generated frontend offers a change or delete only when its input refers to the type itself. The commands that
    // took `change` before fs-types v0.0.16 are `Other` now; one that takes an `id` names its own record just the same.
    for t in descriptions().unwrap() {
        for c in t.commands.iter().filter(|c| {
            matches!(c.kind, CommandKind::Change | CommandKind::Delete)
                || (c.kind == CommandKind::Other && c.input.iter().any(|f| f.name == "id"))
        }) {
            assert!(
                c.input
                    .iter()
                    .any(|f| f.basic == BasicType::Reference && f.target == t.name),
                "{}.{} has no reference to {}",
                t.name,
                c.name,
                t.name
            );
        }
    }
}

#[test]
fn labels_are_translation_keys_and_unique_per_type() {
    for t in descriptions().unwrap() {
        let mut seen = BTreeSet::new();
        for c in &t.commands {
            assert!(seen.insert(c.label.clone()), "{} twice", c.label);
            assert!(c.label.starts_with("wiki."));
        }
    }
}

#[test]
fn unsupported_is_exactly_the_wished_commands_of_requirements() {
    use fs_wiki_core::describe::{is_supported, unsupported, SUPPORTED};
    for (ty, name) in wished_commands() {
        assert!(
            !is_supported(ty, name),
            "{ty}.{name} is wished, so it cannot be supported yet"
        );
    }
    assert!(
        is_supported("Page", "create")
            && is_supported("Link", "find")
            && !is_supported("Page", "nope")
    );
    let reg = registry().unwrap();
    for (ty, name) in SUPPORTED {
        assert!(
            reg.get(ty).and_then(|d| d.find_command(name)).is_some(),
            "{ty}.{name} not described"
        );
    }
    let open: BTreeSet<_> = unsupported().into_iter().collect();
    let wished: BTreeSet<_> = wished_commands()
        .into_iter()
        .map(|(a, b)| (a.to_owned(), b.to_owned()))
        .collect();
    assert_eq!(open, wished);
}

/// Every method of the trait `Texts` is either a command of a description or a reading/bookkeeping method named here.
#[test]
fn every_method_of_texts_is_a_described_command_or_a_named_reading() {
    // method -> (type, command); the methods that read one record or report the provider are in `reading`
    let described = [
        ("create", "Page", "create"),
        ("edit", "Page", "change"),
        ("edit_if", "Page", "change"),
        ("edit_with", "Page", "change"),
        ("move_to", "Page", "move"),
        ("move_folder", "Folder", "move"),
        ("delete", "Page", "delete"),
        ("find", "Page", "find"),
        ("list", "Page", "find"),
        ("page", "Page", "find"),
        ("search", "Page", "search"),
        ("links", "Link", "find"),
        ("backlinks", "Link", "find"),
        ("import_text", "Page", "import"),
        ("export_text", "Page", "export"),
        ("history", "Revision", "find"),
        ("get_revision", "Revision", "get"),
        ("restore_revision", "Revision", "restore"),
        ("purge_revision", "Revision", "purge"),
        ("trash", "Page", "trash"),
        ("list_trash", "TrashEntry", "find"),
        ("restore_from_trash", "TrashEntry", "restore"),
        ("purge", "TrashEntry", "purge"),
        ("create_folder", "Folder", "create"),
        ("rename_folder", "Folder", "rename"),
        ("delete_folder", "Folder", "delete"),
        ("children", "Folder", "find"),
        ("reorder", "Folder", "reorder"),
        ("set_template", "Page", "setTemplate"),
        ("create_from_template", "Page", "createFromTemplate"),
        ("set_redirect", "Page", "setRedirect"),
        ("resolve", "Page", "resolve"),
    ];
    // a record read (the detail of a list), and what the provider reports about itself
    let reading = [
        "get",
        "get_by_path",
        "get_folder",
        "capabilities",
        "identity",
    ];
    let reg = registry().unwrap();
    let source = include_str!("../src/texts.rs");
    let methods: BTreeSet<&str> = source
        .lines()
        .filter_map(|l| {
            l.trim_start()
                .strip_prefix("async fn ")
                .or_else(|| l.trim_start().strip_prefix("fn "))
        })
        .filter_map(|l| l.split('(').next())
        .collect();
    assert!(methods.len() > 30, "the trait was read: {methods:?}");
    for m in &methods {
        let known = reading.contains(m) || described.iter().any(|(n, _, _)| n == m);
        assert!(
            known,
            "Texts::{m} is neither described nor named as a reading"
        );
    }
    for (m, ty, cmd) in described {
        assert!(methods.contains(m), "{m} is not a method of Texts any more");
        assert!(
            reg.get(ty).and_then(|t| t.find_command(cmd)).is_some(),
            "{ty}.{cmd} for {m}"
        );
    }
    // a page can be moved among its siblings as well as a folder
    assert!(reg.get("Page").unwrap().find_command("reorder").is_some());
}
