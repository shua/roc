use bumpalo::{collections::Vec, Bump};
use roc_parse::ast::{CommentOrNewline, Pattern, TypeAnnotation};

use crate::{
    annotation::{Formattable, Newlines, Parens},
    collection::Braces,
    spaces::{fmt_comments_only, fmt_spaces, fmt_spaces_no_blank_lines, NewlineAt, INDENT},
    Buf,
};

#[derive(Copy, Clone, Debug)]
pub struct Sp<'a> {
    pub default_space: bool, // if true and comments is empty, use a space (' ')
    pub force_newline: bool, // if true, force a newline (irrespectively of comments)
    pub comments: &'a [CommentOrNewline<'a>],
}

impl<'a> Sp<'a> {
    pub fn empty() -> Sp<'a> {
        Sp {
            force_newline: false,
            default_space: false,
            comments: &[],
        }
    }
    pub fn space() -> Sp<'a> {
        Sp {
            force_newline: false,
            default_space: true,
            comments: &[],
        }
    }

    pub fn with_space(sp: &'a [CommentOrNewline<'a>]) -> Self {
        Sp {
            force_newline: false,
            default_space: true,
            comments: sp,
        }
    }

    pub fn maybe_with_space(space: bool, sp: &'a [CommentOrNewline<'a>]) -> Sp<'a> {
        Sp {
            force_newline: false,
            default_space: space,
            comments: sp,
        }
    }

    pub fn force_newline(sp: &'a [CommentOrNewline<'a>]) -> Self {
        Sp {
            force_newline: true,
            default_space: false,
            comments: sp,
        }
    }

    pub fn is_multiline(&self) -> bool {
        self.force_newline || !self.comments.is_empty()
    }
}

impl<'a> From<&'a [CommentOrNewline<'a>]> for Sp<'a> {
    fn from(comments: &'a [CommentOrNewline<'a>]) -> Self {
        Sp {
            force_newline: false,
            default_space: false,
            comments,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum Node<'a> {
    Literal(&'a str),
    Sequence {
        first: &'a Node<'a>,
        extra_indent_for_rest: bool,
        rest: &'a [(Sp<'a>, Node<'a>)],
    },
    DelimitedSequence {
        braces: Braces,
        indent_items: bool,
        items: &'a [DelimitedItem<'a>],
        after: Sp<'a>,
    },
    CommaSequence {
        allow_blank_lines: bool,
        allow_newlines: bool,
        indent_rest: bool,
        first: &'a Node<'a>,
        rest: &'a [Item<'a>],
    },

    // Temporary! TODO: translate these into proper Node elements
    TypeAnnotation(TypeAnnotation<'a>),
    Pattern(Pattern<'a>),
}

#[derive(Copy, Clone, Debug)]
pub struct DelimitedItem<'a> {
    pub before: &'a [CommentOrNewline<'a>],
    pub newline: bool,
    pub space: bool,
    pub node: Node<'a>,
    pub comma_after: bool,
}

impl<'a> DelimitedItem<'a> {
    fn is_multiline(&self) -> bool {
        self.newline || !self.before.is_empty() || self.node.is_multiline()
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Item<'a> {
    pub comma_before: bool,
    pub before: &'a [CommentOrNewline<'a>],
    pub newline: bool,
    pub space: bool,
    pub node: Node<'a>,
}

impl<'a> Item<'a> {
    fn is_multiline(&self) -> bool {
        self.newline || !self.before.is_empty() || self.node.is_multiline()
    }
}

impl<'a> Node<'a> {
    pub fn space_seq_3(
        arena: &'a Bump,
        a: Node<'a>,
        b_sp: &'a [CommentOrNewline<'a>],
        c: Node<'a>,
        d_sp: &'a [CommentOrNewline<'a>],
        e: Node<'a>,
    ) -> Node<'a> {
        Node::Sequence {
            first: arena.alloc(a),
            extra_indent_for_rest: true,
            rest: arena.alloc_slice_copy(&[(Sp::with_space(b_sp), c), (Sp::with_space(d_sp), e)]),
        }
    }
}

pub fn parens_around_node<'b, 'a: 'b>(
    arena: &'b Bump,
    item: NodeInfo<'a>,
    allow_space_before: bool,
) -> NodeInfo<'b> {
    NodeInfo {
        before: if allow_space_before { item.before } else { &[] },
        node: Node::DelimitedSequence {
            braces: Braces::Round,
            indent_items: true,
            items: arena.alloc_slice_copy(&[DelimitedItem {
                before: if allow_space_before { &[] } else { item.before },
                node: item.node,
                newline: false,
                space: false,
                comma_after: false,
            }]),
            after: Sp::empty(),
        },
        // We move the comments/newlines to the outer scope, since they tend to migrate there when re-parsed
        after: item.after,
        needs_indent: true, // Maybe want to make parens outdentable?
        prec: Prec::Term,
    }
}

#[derive(Copy, Clone, Debug)]
pub struct NodeInfo<'b> {
    pub before: &'b [CommentOrNewline<'b>],
    pub node: Node<'b>,
    pub after: &'b [CommentOrNewline<'b>],
    pub needs_indent: bool,
    pub prec: Prec,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Prec {
    Term,
    Apply,
    AsType,
    FunctionType,
    Outer,
}

impl From<Parens> for Prec {
    fn from(parens: Parens) -> Self {
        match parens {
            Parens::NotNeeded => Prec::Outer,
            Parens::InClosurePattern => Prec::Outer,
            Parens::InApply => Prec::Apply,
            Parens::InApplyLastArg => Prec::Apply,
            Parens::InCollection => Prec::FunctionType,
            Parens::InFunctionType => Prec::FunctionType,
            Parens::InOperator => Prec::FunctionType,
            Parens::InAsPattern => Prec::AsType,
        }
    }
}

impl<'b> NodeInfo<'b> {
    pub fn item(text: Node<'b>) -> NodeInfo<'b> {
        NodeInfo {
            before: &[],
            node: text,
            after: &[],
            needs_indent: true,
            prec: Prec::Term,
        }
    }

    pub fn add_parens<'a>(&self, arena: &'a Bump, parens: Parens) -> NodeInfo<'a>
    where
        'b: 'a,
    {
        if self.prec < parens.into() {
            *self
        } else {
            parens_around_node(arena, *self, true)
        }
    }

    pub fn add_ty_ext_parens<'a>(&self, arena: &'a Bump) -> NodeInfo<'a>
    where
        'b: 'a,
    {
        if self.prec <= Prec::Term && self.before.is_empty() {
            *self
        } else {
            parens_around_node(arena, *self, false)
        }
    }
}

pub trait Nodify<'a> {
    fn to_node<'b>(&'a self, arena: &'b Bump) -> NodeInfo<'b>
    where
        'a: 'b;
}

fn fmt_sp(buf: &mut Buf, sp: Sp<'_>, indent: u16) {
    if !sp.comments.is_empty() {
        fmt_spaces(buf, sp.comments.iter(), indent);
    } else if sp.force_newline {
        buf.ensure_ends_with_newline();
    } else if sp.default_space {
        buf.spaces(1);
    }
}

impl<'a> Formattable for Node<'a> {
    fn is_multiline(&self) -> bool {
        match self {
            Node::Sequence {
                first,
                extra_indent_for_rest: _,
                rest,
            } => {
                first.is_multiline()
                    || rest
                        .iter()
                        .any(|(sp, l)| l.is_multiline() || sp.is_multiline())
            }
            Node::DelimitedSequence {
                braces: _,
                indent_items: _,
                items,
                after,
            } => after.is_multiline() || items.iter().any(|item| item.is_multiline()),
            Node::CommaSequence {
                allow_blank_lines: _,
                allow_newlines: _,
                indent_rest: _,
                first,
                rest,
            } => first.is_multiline() || rest.iter().any(|item| item.is_multiline()),
            Node::Literal(_) => false,
            Node::TypeAnnotation(type_annotation) => type_annotation.is_multiline(),
            Node::Pattern(pat) => pat.is_multiline(),
        }
    }

    fn format_with_options(&self, buf: &mut Buf, parens: Parens, newlines: Newlines, indent: u16) {
        match self {
            Node::DelimitedSequence {
                braces,
                indent_items,
                items: lefts,
                after: right,
            } => {
                buf.indent(indent);
                buf.push(braces.start());

                let inner_indent = if *indent_items {
                    indent + INDENT
                } else {
                    indent
                };

                for item in *lefts {
                    fmt_spaces(buf, item.before.iter(), inner_indent);
                    if item.newline {
                        buf.ensure_ends_with_newline();
                    } else if item.space {
                        buf.ensure_ends_with_whitespace();
                    }
                    item.node
                        .format_with_options(buf, parens, newlines, inner_indent);
                    if item.comma_after {
                        buf.push(',');
                    }
                }
                fmt_sp(buf, *right, inner_indent);

                buf.indent(indent);
                buf.push(braces.end());
            }
            Node::Sequence {
                first,
                extra_indent_for_rest,
                rest,
            } => {
                buf.indent(indent);
                let cur_indent = buf.cur_line_indent();
                first.format_with_options(buf, parens, newlines, indent);
                let next_indent = if *extra_indent_for_rest {
                    cur_indent + INDENT
                } else {
                    indent
                };

                for (sp, l) in *rest {
                    fmt_sp(buf, *sp, next_indent);
                    l.format_with_options(buf, parens, newlines, next_indent);
                }
            }
            Node::CommaSequence {
                allow_blank_lines,
                allow_newlines,
                indent_rest,
                first,
                rest,
            } => {
                buf.indent(indent);
                let inner_indent = if *indent_rest {
                    indent + INDENT
                } else {
                    indent
                };
                first.format_with_options(buf, parens, newlines, indent);

                for item in *rest {
                    if item.comma_before {
                        buf.push(',');
                    }
                    if *allow_blank_lines {
                        fmt_spaces(buf, item.before.iter(), indent);
                    } else if *allow_newlines {
                        fmt_spaces_no_blank_lines(buf, item.before.iter(), inner_indent);
                    } else {
                        fmt_comments_only(buf, item.before.iter(), NewlineAt::Bottom, inner_indent);
                    }
                    if item.newline {
                        buf.ensure_ends_with_newline();
                    } else if item.space {
                        buf.ensure_ends_with_whitespace();
                    }
                    item.node
                        .format_with_options(buf, parens, newlines, inner_indent);
                }
            }
            Node::Literal(text) => {
                buf.indent(indent);
                buf.push_str(text);
            }
            Node::TypeAnnotation(type_annotation) => {
                type_annotation.format_with_options(buf, parens, newlines, indent);
            }
            Node::Pattern(pat) => {
                pat.format_with_options(buf, parens, newlines, indent);
            }
        }
    }
}

pub struct NodeSequenceBuilder<'a> {
    first: Node<'a>,
    extra_indent_for_rest: bool,
    rest: Vec<'a, (Sp<'a>, Node<'a>)>,
}

impl<'a> NodeSequenceBuilder<'a> {
    pub fn new(
        arena: &'a Bump,
        first: Node<'a>,
        capacity: usize,
        extra_indent_for_rest: bool,
    ) -> Self {
        Self {
            first,
            extra_indent_for_rest,
            rest: Vec::with_capacity_in(capacity, arena),
        }
    }

    pub fn push(&mut self, sp: Sp<'a>, literal: Node<'a>) {
        self.rest.push((sp, literal));
    }

    pub fn build(self) -> Node<'a> {
        Node::Sequence {
            first: self.rest.bump().alloc(self.first),
            extra_indent_for_rest: self.extra_indent_for_rest,
            rest: self.rest.into_bump_slice(),
        }
    }
}
