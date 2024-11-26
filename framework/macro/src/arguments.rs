use std::collections::HashSet;

#[derive(deluxe::ParseMetaItem)]
pub struct MacroArgs {
    #[deluxe(default)]
    pub debug: bool,

    #[deluxe(default)]
    pub html_tags: HashSet<syn::Ident>,

    #[deluxe(default)]
    pub tag: Option<syn::Ident>,

    #[deluxe(default)]
    pub key: Option<syn::Expr>,
}

impl MacroArgs {
    pub fn parse2(attr: proc_macro2::TokenStream) -> syn::Result<MacroArgs> {
        let mut args = deluxe::parse2::<MacroArgs>(attr)?;
        if args.html_tags.is_empty() {
            args.html_tags.extend(well_known_tags());
        }
        return Ok(args);
    }
}

fn well_known_tags() -> HashSet<syn::Ident> {
    [
        ["tag"].as_slice(),
        &["a", "abbr", "address", "area", "article", "aside", "audio"],
        &["b", "base", "bdi", "bdo", "blockquote", "body", "br"],
        &["button", "canvas", "caption", "cite", "code", "col"],
        &["colgroup", "data", "datalist", "dd", "del", "details"],
        &["dfn", "dialog", "div", "dl", "dt", "em", "embed"],
        &["fieldset", "figcaption", "figure", "footer", "form"],
        &["h1", "h2", "h3", "h4", "h5", "h6", "head", "header"],
        &["hgroup", "hr", "html", "i", "iframe", "img", "input", "ins"],
        &["kbd", "label", "legend", "li", "link", "main", "map"],
        &["mark", "meta", "meter", "nav", "noscript", "object", "ol"],
        &["optgroup", "option", "output", "p", "param", "picture"],
        &["pre", "progress", "q", "rp", "rt", "ruby", "s", "samp"],
        &["script", "section", "select", "small", "source", "span"],
        &["strong", "style", "sub", "summary", "sup", "svg", "table"],
        &["tbody", "td", "template", "textarea", "tfoot", "th"],
        &["thead", "time", "title", "tr", "track", "u", "ul", "var"],
        &["video", "wbr"],
    ]
    .into_iter()
    .flatten()
    .map(|tag| syn::Ident::new(tag, proc_macro2::Span::call_site()))
    .inspect({
        // Check for duplicates
        let mut tags = HashSet::new();
        move |tag| assert!(tags.insert(tag.clone()))
    })
    .collect()
}
