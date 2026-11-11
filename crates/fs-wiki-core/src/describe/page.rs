//! `Page`: a text of the wiki (`TextEntry`).
//!
//! Pattern: Value object (the description), `DescribesType` is the Registry entry. Standard: W3C `ActivityStreams` 2.0
//! `Article`, because the core stores every text in an AS2 `Article` envelope (texts.capnp `TextEntry.envelope`, ADR 0031);
//! the content travels as Markdown (the fs-text document is the truth, Markdown the readable form, ADR 0042).
//! Templates and redirects are states of a page (`kind`, `redirectTo`), not types of their own, as in the contract.

use fs_types::{
    BasicType as B, CommandKind as K, DescribesType, FieldDescription, TypeDescription,
};

use super::{command, field, optional_reference, page_wish, reference, PROGRAM};
use crate::TextEntry;

const T: &str = "page";

fn fields(d: TypeDescription) -> TypeDescription {
    // What the program sets itself is read-only: the form of `change` offers the rest (path, content, kind, redirect).
    d.field(reference("id", "Page", "wiki.page.id").read_only())
        .field(field(T, "path", B::Text).required())
        .field(field(T, "name", B::Text).read_only())
        .field(field(T, "content", B::MultilineText))
        .field(field(T, "kind", B::Choice).choices(&["page", "template"]))
        .field(field(T, "redirectTo", B::Text))
        .field(field(T, "updatedBy", B::Text).read_only())
        .field(field(T, "revision", B::Text).read_only())
}

fn writing(d: TypeDescription) -> TypeDescription {
    let id = || reference("id", "Page", "wiki.page.id");
    d.command(
        command(T, "create", K::Create)
            .input(field(T, "path", B::Text).required())
            .input(field(T, "content", B::MultilineText)),
    )
    .command(
        command(T, "import", K::Create)
            .input(field(T, "path", B::Text).required())
            .input(field(T, "mediaType", B::Text).required())
            .input(field(T, "text", B::MultilineText).required()),
    )
    .command(
        command(T, "createFromTemplate", K::Create)
            .input(reference("template", "Page", "wiki.page.template"))
            .input(field(T, "path", B::Text).required()),
    )
    // A typed edit (`Edit`, fs-types): a field that is not sent stays, one with no values is cleared. No input here: the
    // form is made of the fields of the type.
    .command(command(T, "change", K::Change))
    .command(
        command(T, "move", K::Other)
            .input(id())
            .input(field(T, "path", B::Text).required()),
    )
    .command(
        command(T, "setTemplate", K::Other).input(id()).input(
            field(T, "kind", B::Choice)
                .choices(&["page", "template"])
                .required(),
        ),
    )
    .command(
        command(T, "setRedirect", K::Other)
            .input(id())
            .input(field(T, "redirectTo", B::Text)),
    )
    .command(
        command(T, "reorder", K::Other)
            .input(id())
            .input(
                field(T, "placement", B::Choice)
                    .choices(&["first", "last", "before", "after"])
                    .required(),
            )
            .input(FieldDescription::new("anchor", B::Reference).label("wiki.page.anchor")),
    )
    .command(command(T, "trash", K::Delete).input(id()))
    .command(command(T, "delete", K::Delete).input(id()))
}

fn reading(d: TypeDescription) -> TypeDescription {
    d.command(command(T, "find", K::Find).input(field(T, "term", B::Text)))
        .command(
            command(T, "search", K::Find)
                .input(field(T, "term", B::Text))
                .input(field(T, "prefix", B::Choice).choices(&["no", "yes"]))
                .input(field(T, "pathPrefix", B::Text))
                .input(field(T, "templates", B::Choice).choices(&["any", "only", "exclude"]))
                .input(field(T, "redirects", B::Choice).choices(&["any", "only", "exclude"]))
                .input(field(T, "author", B::Text))
                .input(
                    FieldDescription::new("space", B::Reference)
                        .target("Folder")
                        .label("wiki.page.space"),
                )
                .input(field(T, "updatedFrom", B::DateTime))
                .input(field(T, "updatedTo", B::DateTime)),
        )
        .command(
            command(T, "resolve", K::Other)
                .input(field(T, "path", B::Text).required())
                .result(field(T, "outcome", B::Choice).choices(&[
                    "resolved",
                    "not-found",
                    "loop",
                    "too-many-hops",
                ]))
                .result(field(T, "hops", B::Text).multiple())
                .result(optional_reference("id", "Page", "wiki.page.id")),
        )
        .command(
            command(T, "export", K::Other)
                .input(reference("id", "Page", "wiki.page.id"))
                .input(field(T, "mediaType", B::Text).required())
                .result(field(T, "text", B::MultilineText)),
        )
}

fn describe() -> TypeDescription {
    let d = TypeDescription::new(
        "Page",
        PROGRAM,
        "W3C ActivityStreams 2.0 Article",
        "wiki.page",
    );
    page_wish::add(reading(writing(fields(d))))
}

impl DescribesType for TextEntry {
    fn describe() -> TypeDescription {
        describe()
    }
}
