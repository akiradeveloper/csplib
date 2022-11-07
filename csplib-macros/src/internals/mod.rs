use proc_macro2::TokenStream;
use quote::quote;
use std::str::FromStr;
use syn::ItemStruct;

mod generate;

#[derive(Clone, Debug)]
struct Var {
    name: String,
    typ: String,
}

#[derive(Debug)]
pub struct ProcessDef {
    name: String,
    fields: Vec<FieldDef>,
}

#[derive(Debug)]
struct FieldDef {
    dir: Dir,
    var: Var,
}

#[derive(Debug)]
enum Dir {
    Input,
    Output,
}

fn parse_fields(t: &syn::Fields) -> Vec<FieldDef> {
    let mut out = vec![];
    match t {
        syn::Fields::Named(fields_named) => {
            for field in &fields_named.named {
                let name = {
                    let x = &field.ident;
                    quote!(#x).to_string()
                };
                let typ = {
                    let ty = &field.ty;
                    quote!(#ty).to_string()
                };
                let dir = {
                    let attr = &field.attrs[0];
                    let x = &attr.path;
                    quote!(#x).to_string()
                };
                let dir = match dir.as_str() {
                    "output" => Dir::Output,
                    "input" => Dir::Input,
                    _ => unreachable!(),
                };
                out.push(FieldDef {
                    dir,
                    var: Var { name, typ },
                });
            }
        }
        _ => unreachable!(),
    }
    out
}

fn parse_process_def(tok: TokenStream) -> ProcessDef {
    let t = syn::parse2::<ItemStruct>(tok).unwrap();
    let p_name = {
        let x = &t.ident;
        quote!(#x).to_string()
    };
    let fields = parse_fields(&t.fields);
    ProcessDef {
        name: p_name,
        fields,
    }
}

pub fn process(item: TokenStream) -> TokenStream {
    let p = parse_process_def(item);
    let code = generate::generate(p);
    TokenStream::from_str(&code).unwrap()
}

#[test]
fn test_parse() {
    let tok = quote!(
        #[csplib::process]
        struct Add {
            #[input]
            a: bool,
            #[input]
            b: bool,
            #[output]
            c: bool,
        }
    );
    let parsed = parse_process_def(tok);
    dbg!(&parsed);
    dbg!(generate::generate(parsed));
}
