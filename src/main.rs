//! Top level docs.

use std::error::Error;
use std::fmt::{self, Write};
use std::fs;
use std::io::Read;

use proc_macro2::TokenTree;
use pulldown_cmark::{Event, Options, Parser, Tag};
use syn::visit::{self, Visit};
use syn::{AttrStyle, Attribute, ItemFn, ItemMod, Signature};
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};

struct Visitor {
    docs: Vec<Doc>,
}

#[derive(Debug)]
enum Doc {
    FnDoc(FnDoc),
    ModDoc(ModDoc),
}

impl fmt::Display for Doc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Doc::FnDoc(fn_doc) => write!(f, "{}", fn_doc),
            Doc::ModDoc(mod_doc) => write!(f, "{}", mod_doc),
        }
    }
}

/// Does some basic markdown parsing so we can get to the codeblocks.
fn format_markdown(input: &str) -> String {
    let parser = Parser::new_ext(input, Options::empty());

    let mut out = String::new();
    let mut code = String::new();
    let mut is_code = false;

    for event in parser {
        match event {
            Event::Start(tag) => match tag {
                Tag::CodeBlock(_code_block_kind) => is_code = true,
                Tag::Paragraph => {}
                Tag::Heading(level) => {
                    write!(out, "\n\n{:#>1$} ", "", level as usize).unwrap();
                }
                _ => todo!("{:?}", tag),
            },
            Event::End(tag) => {
                match tag {
                    Tag::CodeBlock { .. } => {
                        // Indented
                        // Fenced(CowStr<'a>)
                        is_code = false;

                        // TODO: Highlight!
                        let ps = SyntaxSet::load_defaults_newlines();
                        let ts = ThemeSet::load_defaults();

                        let syntax = ps.find_syntax_by_extension("rs").unwrap();
                        let mut h = HighlightLines::new(syntax, &ts.themes["base16-ocean.dark"]);
                        for line in LinesWithEndings::from(&code) {
                            // LinesWithEndings enables use of newlines mode
                            let ranges: Vec<(Style, &str)> = h.highlight(line, &ps);
                            let escaped = as_24_bit_terminal_escaped(&ranges[..], true);
                            write!(out, "{}", escaped).unwrap();
                        }
                        write!(out, "\x1b[0m\n\n").unwrap();
                        code.clear();
                    }
                    Tag::Heading(_level) => write!(out, "\n\n").unwrap(),
                    Tag::Paragraph => write!(code, "\n\n").unwrap(),
                    _ => todo!("{:?}", tag),
                }
            }
            Event::Text(text) => {
                if is_code {
                    write!(code, "{}", text).unwrap();
                } else {
                    write!(out, "{}", text).unwrap();
                }
            }
            Event::Code(_) => todo!(),
            Event::Html(_) => todo!(),
            Event::FootnoteReference(_) => todo!(),
            Event::SoftBreak => todo!(),
            Event::HardBreak => todo!(),
            Event::Rule => todo!(),
            Event::TaskListMarker(_) => todo!(),
        }
    }
    out
}

#[derive(Debug)]
struct FnDoc {
    signature: String,
    doc: String,
}

impl fmt::Display for FnDoc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let hl = format!("{:-^1$}", "function", 80); // TODO: Useful heading

        write!(
            f,
            "{}\n\n{}{}\n\n",
            hl,
            self.signature,
            format_markdown(&self.doc)
        )
    }
}

#[derive(Debug)]
struct ModDoc {
    ident: String,
    doc: String,
}

impl fmt::Display for ModDoc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let hl = format!("{:-^1$}", format!("module {}", self.ident), 80);
        write!(f, "{}\n\n{}\n\n", hl, format_markdown(&self.doc))
    }
}

/// Formats a syn::Signature into a human readabable signature.
///
/// ## TODO: Output other parts of the sigature including return types, types, and where clauses.
fn format_signature(sig: &Signature) -> String {
    format!("fn {}()\n\n", &sig.ident)
}

fn format_doc(attrs: &[Attribute]) -> String {
    // The compiler transforms doc comments, such as /// comment and /*! comment */, into
    // attributes before macros are expanded. Each comment is expanded into an attribute of the
    // form #[doc = r"comment"].
    //
    // Outer doc comments like /// Example.
    // Inner doc comments like //! Example.

    let mut doc = String::new();
    for attr in attrs {
        if attr.style == AttrStyle::Outer {
            for token in attr.tokens.clone().into_iter() {
                match token {
                    TokenTree::Literal(lit) => {
                        let mut lit = lit.to_string();
                        lit.remove(0); // remove the first `"`
                        lit.remove(0); // assume there is a leading space (TODO: Fix this assumption)
                        if !lit.is_empty() {
                            lit.remove(lit.len() - 1); // remove the last `"`
                        }
                        write!(doc, "{}\n", lit).unwrap();
                    }
                    _ => (),
                }
            }
        }
    }
    doc
}

impl<'ast> Visit<'ast> for Visitor {
    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        let signature = format_signature(&node.sig);
        let doc = format_doc(&node.attrs);

        self.docs.push(Doc::FnDoc(FnDoc { signature, doc }));

        // Delegate to the default impl to visit any nested functions.
        visit::visit_item_fn(self, node);
    }

    fn visit_item_mod(&mut self, node: &'ast ItemMod) {
        let ident = format!("{}", &node.ident);
        let doc = format_doc(&node.attrs);
        self.docs.push(Doc::ModDoc(ModDoc { ident, doc }));

        visit::visit_item_mod(self, node);
    }
}

/// This is just a test module to use for formatting!
mod foo {
    #[allow(dead_code)]

    /// BAR is the answer to the universe and everything.
    ///
    /// ## Examples
    ///
    /// Fenced block example:
    ///
    /// ```should_panic
    /// fn find_answer() -> usize {
    ///     todo!();
    ///     BAR
    /// }
    /// ```
    ///
    /// The answer:
    ///
    /// ```ignore
    /// const THE_ANSWER: usize = BAR;
    /// ```
    const BAR: usize = 42;

    /// Foo
    fn foo() {}
}

/// Hello, this is the main doc!
///
/// ## Examples
///
///     let mut x = {
///         1 + 1
///     };
///
/// #### A level 4 heading (example two):
///
///     mod two {
///         fn foo() -> usize {
///             1 + 1
///         }
///     }
///
fn main() -> Result<(), Box<dyn Error>> {
    let mut file = fs::File::open(
        std::env::args()
            .skip(1)
            .next()
            .expect("no filename provided"),
    )?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;

    let ast = syn::parse_file(&content)?;

    let mut visitor = Visitor { docs: Vec::new() };
    visitor.visit_file(&ast);

    for doc in visitor.docs {
        print!("{}", doc);
    }

    Ok(())
}
