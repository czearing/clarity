//! Count public functions that can stop, and say which already carry a comment.

use quote::ToTokens;
use syn::visit::Visit;

struct Stops {
    found: bool,
}

impl<'a> Visit<'a> for Stops {
    fn visit_macro(&mut self, node: &'a syn::Macro) {
        let name = node
            .path
            .segments
            .last()
            .map(|s| s.ident.to_string())
            .unwrap_or_default();
        if matches!(
            name.as_str(),
            "assert"
                | "assert_eq"
                | "assert_ne"
                | "panic"
                | "unreachable"
                | "todo"
                | "unimplemented"
        ) {
            self.found = true;
        }
        syn::visit::visit_macro(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'a syn::ExprMethodCall) {
        if matches!(node.method.to_string().as_str(), "unwrap" | "expect") {
            self.found = true;
        }
        syn::visit::visit_expr_method_call(self, node);
    }
}

fn public(vis: &syn::Visibility) -> bool {
    matches!(vis, syn::Visibility::Public(_))
}

fn tested(attrs: &[syn::Attribute]) -> bool {
    attrs
        .iter()
        .any(|a| a.path().is_ident("cfg") && a.to_token_stream().to_string().contains("test"))
}

fn documented(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|a| a.path().is_ident("doc"))
}

fn warns(attrs: &[syn::Attribute]) -> bool {
    attrs
        .iter()
        .filter(|a| a.path().is_ident("doc"))
        .any(|a| a.to_token_stream().to_string().contains("Panics"))
}

struct Count {
    undocumented: Vec<String>,
    documented: Vec<String>,
    silent: Vec<String>,
    total: usize,
    file: String,
}

impl Count {
    fn look(
        &mut self,
        sig: &syn::Signature,
        vis: &syn::Visibility,
        attrs: &[syn::Attribute],
        block: &syn::Block,
    ) {
        if !public(vis) {
            return;
        }
        self.total += 1;
        let mut stops = Stops { found: false };
        stops.visit_block(block);
        if !stops.found {
            return;
        }
        let name = format!("{}::{}", self.file, sig.ident);
        if documented(attrs) {
            if warns(attrs) {
                self.documented.push(name);
            } else {
                self.silent.push(name);
            }
        } else {
            self.undocumented.push(name);
        }
    }

    fn items(&mut self, items: &[syn::Item]) {
        for item in items {
            match item {
                syn::Item::Fn(f) if !tested(&f.attrs) => {
                    self.look(&f.sig, &f.vis, &f.attrs, &f.block);
                }
                syn::Item::Impl(i) => {
                    for sub in &i.items {
                        if let syn::ImplItem::Fn(f) = sub {
                            if !tested(&f.attrs) {
                                self.look(&f.sig, &f.vis, &f.attrs, &f.block);
                            }
                        }
                    }
                }
                syn::Item::Mod(m) if !tested(&m.attrs) => {
                    if let Some((_, inner)) = &m.content {
                        self.items(inner);
                    }
                }
                _ => {}
            }
        }
    }
}

fn main() {
    let mut count = Count {
        undocumented: Vec::new(),
        documented: Vec::new(),
        silent: Vec::new(),
        total: 0,
        file: String::new(),
    };
    for path in std::env::args().skip(1) {
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(file) = syn::parse_file(&text) else {
            eprintln!("unparsed {path}");
            continue;
        };
        count.file.clone_from(&path);
        count.items(&file.items);
    }
    println!("public fns        {}", count.total);
    println!("stopping, no doc  {}", count.undocumented.len());
    println!("stopping, warns    {}", count.documented.len());
    println!("stopping, doc silent on the stop {}", count.silent.len());
    for name in &count.undocumented {
        println!("  UNDOCUMENTED {name}");
    }
    for name in &count.silent {
        println!("  DOC SAYS NOTHING ABOUT THE STOP {name}");
    }
}
