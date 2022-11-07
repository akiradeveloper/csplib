use proc_macro::TokenStream;

mod internals;

#[proc_macro_attribute]
pub fn process(_: TokenStream, item: TokenStream) -> TokenStream {
    internals::process(item.into()).into()
}
