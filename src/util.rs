#![cfg_attr(not(debug_assertions),allow(non_snake_case,non_upper_case_globals,non_camel_case_types))]
#![cfg_attr(    debug_assertions ,allow(non_snake_case,non_upper_case_globals,non_camel_case_types,unused_imports,unused_mut,unused_variables,dead_code,unused_assignments,unused_macros))]
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, Expr, FnArg, Generics, Ident, LitStr, Meta, MetaNameValue, Pat};

use crate::is_parameters_section;

/// Function to prepend a string to a `#[doc]` attribute.
pub fn prepend_to_doc_attribute(prepend_text: &str, attr: &Attribute) -> proc_macro2::TokenStream {
    // Parse the attribute to see if it's a MetaNameValue (e.g., #[doc = "..."])
    assert!(
        attr.path().is_ident("doc"),
        "function must only be called on doc attributes"
    );
    if let Meta::NameValue(MetaNameValue {
        value: Expr::Lit(ref lit),
        ..
    }) = attr.meta
    {
        let syn::ExprLit {
            attrs: _,
            lit: syn::Lit::Str(doc_string),
        } = lit
        else {
            unreachable!("reached unexpected node while parsing");
        };
        // Prepend the text to the existing doc comment
        let new_doc = format!("{}{}", prepend_text, doc_string.value());

        // Create a new string literal with the modified doc string
        let new_doc_lit = LitStr::new(&new_doc, doc_string.span());

        // Create a new attribute with the modified doc string (enclosed in quotes)
        let new_attr = quote! {
            #[doc = #new_doc_lit]
        };
        new_attr
    } else {
        unreachable!("reached unexpected node while parsing");
    }
}

/// removes the #[doc...] attributes from `attrs` and returns them in
/// it's own vector.
pub fn extract_doc_attrs(attrs: &mut Vec<Attribute>) -> Vec<Attribute> {
    let doc_attrs = attrs
        .iter()
        .filter(|attr| attr.path().is_ident("doc"))
        .cloned()
        .collect();
    attrs.retain(|attr| !attr.path().is_ident("doc"));
    doc_attrs
}

/// function doc attributes split by whether they occur above or below
/// the attribute that indicates where the argument section should be placed.
/// If no such attribute is present, we stick all the doc attributes in the
/// before section
pub struct FunctionDocs {
    pub before_args_section: Vec<Attribute>,
    pub after_args_section: Vec<Attribute>,
}

/// Struct doc attributes split by whether they occur above or below the attribute that indicates where the field section should be placed.
/// If no such attribute is present, we stick all the doc attributes in the before section
pub struct StructDocs {
    pub before_args_section: Vec<Attribute>,
    pub after_args_section: Vec<Attribute>,
}

/// extract the documentation from the doc comments of the function and perform
/// some additional logic
pub fn extract_fn_doc_attrs(attrs: &mut Vec<Attribute>) -> Result<FunctionDocs, syn::Error> {
    let mut before_args_section = Vec::with_capacity(attrs.len());
    let mut after_args_section = Vec::with_capacity(attrs.len());

    // I'm sure this could be done with iterators...
    // I'm just too dumb to do that now
    let mut idx = 0;
    // parse the arguments before the arguments-section attribute
    while idx < attrs.len() {
        let current_attr = attrs.get(idx).unwrap();
        if is_parameters_section(current_attr) {
            idx += 1;
            break;
        }
        if current_attr.path().is_ident("doc") {
            before_args_section.push(current_attr.clone());
        }
        idx += 1;
    }

    while idx < attrs.len() {
        let current_attr = attrs.get(idx).unwrap();
        if is_parameters_section(current_attr) {
            return Err(syn::Error::new_spanned(
                current_attr,
                "Duplicate attribute not allowed.",
            ));
        }
        if current_attr.path().is_ident("doc") {
            after_args_section.push(current_attr.clone());
        }
        idx += 1;
    }

    // delete all doc attributes from the function (and the arguments section attributes that I don't need anymore)
    attrs.retain(|attr| !attr.path().is_ident("doc"));
    Ok(FunctionDocs {
        before_args_section,
        after_args_section,
    })
}

/// extract the documentation from the doc comments of the struct and perform some additional logic
pub fn extract_struct_doc_attrs(attrs: &mut Vec<Attribute>) -> Result<StructDocs, syn::Error> {
    let mut before_args_section = Vec::with_capacity(attrs.len());
    let mut after_args_section = Vec::with_capacity(attrs.len());

    let mut idx = 0;
    while idx < attrs.len() { // parse the arguments before the arguments-section attribute
        let current_attr = attrs.get(idx).unwrap();
        if is_parameters_section(current_attr) {idx += 1;break;}
        if current_attr.path().is_ident("doc") {before_args_section.push(current_attr.clone());}
        idx += 1;
    }
    while idx < attrs.len() {
        let current_attr = attrs.get(idx).unwrap();
        if is_parameters_section(current_attr) {return Err(syn::Error::new_spanned(current_attr,"Duplicate attribute not allowed.",));}
        if current_attr.path().is_ident("doc") {after_args_section.push(current_attr.clone());}
        idx += 1;
    }
    attrs.retain(|attr| !attr.path().is_ident("doc")); // delete all doc attributes from the struct (and the arguments section attributes that I don't need anymore)
    Ok(StructDocs {before_args_section,after_args_section,})
}

/// an identifier (such as a function parameter or a generic type)
/// with doc attributes
pub struct DocumentedIdent<'a> {
    pub ident: &'a Ident,
    /// the doc comments
    pub docs: Vec<Attribute>,
}

impl<'a> DocumentedIdent<'a> {
    pub fn new(ident: &'a Ident, docs: Vec<Attribute>) -> Self {
        Self { ident, docs }
    }
}

/// extract the parameter documentation from an iterator over function arguments.
/// This will also remove all the doc comments from the collection of attributes, but
/// will leave all the other attributes untouched.
pub fn extract_documented_parameters<'a, I>(args: I) -> Result<Vec<DocumentedIdent<'a>>, syn::Error>
where
    I: Iterator<Item = &'a mut FnArg>,
{
    // will contain the docs comments for each documented function parameter
    // together with the identifier of the function parameter.
    let (lower, upper) = args.size_hint();
    let mut documented_params = Vec::<DocumentedIdent>::with_capacity(upper.unwrap_or(lower));

    for arg in args {
        match arg {
            FnArg::Typed(pat_type) => {
                let Pat::Ident(pat_ident) = pat_type.pat.as_ref() else {
                    unreachable!("unexpected node while parsing");
                };
                let ident = &pat_ident.ident;
                let docs = extract_doc_attrs(&mut pat_type.attrs);

                if !docs.is_empty() {
                    documented_params.push(DocumentedIdent::new(ident, docs));
                }
            }
            FnArg::Receiver(_) => {}
        }
    }
    Ok(documented_params)
}

use itertools::{Itertools, Position as IPos};
use syn::ExprLit;
use syn::Lit;
/// Split a doc in 2 parts: before ///! and after
pub fn split_doc_in2(docs_last:Vec::<Attribute>) -> (Vec::<Attribute>,Vec::<Attribute>){
    let mut is_split = false;
    let mut docs2this:Vec::<Attribute> = vec![];
    let mut docs2next:Vec::<Attribute> = vec![];
    for mut attr in docs_last {
        if let Meta::NameValue(MetaNameValue {value: Expr::Lit(ExprLit{lit: Lit::Str(ref mut lit_s),..}),..}) = attr.meta {
            if ! is_split {
                let s = lit_s.value();
                if  s.starts_with('!') { is_split = true;
                    *lit_s = LitStr::new(&s[1..s.len()], lit_s.span());
                        docs2next.push(attr); // assign post ///! doc lines to the last parameter
                } else {docs2this.push(attr);}
            }     else {docs2next.push(attr);} // everything post stplit goes to the last parameter, ignore further ///!
        }
    }
    (docs2this,docs2next)
}
/// Same as extract_documented_parameters, but shifts all docs by -1, returning the 1st parameter's docs separately,
/// so that it can be used as a function comment
/// Also allows splitting the last parameter's docs into 2: belonging to the last parameter (after ///!) and to the previous one
///
/// fn sum_image_rows( /// this comment belongs to the function, not to the next parameter, so will be returned separately
///  image_data : &[f32],/// this comment belongs to the preceding `image_data` parameter, not `nrows`
///  nrows      :   u32 ,/// this part of the comment belongs to `nrows`
///                             ///! but this part — to the last `ncols` parameter
///  ncols      :   u32 ,// it's a syntax error to add doc comments at the end
/// )
pub fn extract_documented_parameters_shift_up<'a,I>(args: I) -> Result<(Option<Vec<Attribute>>,Vec<DocumentedIdent<'a>>), syn::Error>
    where                                        I:Iterator<Item = &'a mut FnArg>,{
    // will contain the docs comments for each documented function parameter together with the identifier of the function parameter
    let (lower, upper) = args.size_hint();
    let mut documented_params = Vec::<DocumentedIdent>::with_capacity(upper.unwrap_or(lower));

    let mut docs0fn   :Option<Vec::<Attribute>> = None;
    let mut ident_prev:Option<     &Ident     > = None;
    let mut ident_last:Option<     &Ident     > = None;
    let mut ident_only:Option<     &Ident     > = None;
    let mut docs_last :       Vec::<Attribute> = vec![];
    for (pos,arg) in args.with_position() {
        match arg {
            FnArg::Typed(pat_type) => {
                let Pat::Ident(pat_ident) = pat_type.pat.as_ref() else {unreachable!("unexpected node while parsing");};
                let ident = &pat_ident.ident;
                let docs = extract_doc_attrs(&mut pat_type.attrs);
                if !docs.is_empty() {
                    match pos {
                        IPos::Only   => {ident_only = Some(ident); docs_last =      docs;break;},// break to avoid wrong ident_prev
                        IPos::First  => {                          docs0fn   = Some(docs);     },// no ///! split needed, pre-par docs go to fn
                        IPos::Middle => {documented_params.push(DocumentedIdent::new(ident_prev.take().expect("saved prev ident"), docs));},
                        IPos::Last   => {documented_params.push(DocumentedIdent::new(ident_prev.take().expect("saved prev ident"), docs.clone()));
                                         ident_last = Some(ident); docs_last =      docs;break;},
                    } // ↓ don't set on last item, break before
                }; ident_prev = Some(ident); // save id even without docs since next docs might need to be split-attached to it
            }
            FnArg::Receiver(_) => {}
        }
    }
    if        let Some(ident_last) = ident_last { // on ///! split the docs between 2 parameters, removing !
        let (docs2prev,docs2last) = split_doc_in2(docs_last);
        if ! docs2last.is_empty() {
            if let Some(mut docum_par_prev) = documented_params.pop() { // replace last-1 item's docs with its pre-///! docs
                docum_par_prev.docs = docs2prev;
                documented_params.push(docum_par_prev);
                documented_params.push(DocumentedIdent::new(ident_last, docs2last));
            } else {                                                    // add     last-i item's docs …
                documented_params.push(DocumentedIdent::new(ident_prev.expect("saved prev ident"), docs2prev));
                documented_params.push(DocumentedIdent::new(ident_last, docs2last));
            }
        }
    } else if let Some(ident_only) = ident_only { // on ///! split the docs between fn and parameter, removing !
        let (docs2fn,docs2par) = split_doc_in2(docs_last);
        if ! docs2fn .is_empty() {                docs0fn = Some(docs2fn);}
        if ! docs2par.is_empty() {
                documented_params.push(DocumentedIdent::new(ident_only, docs2par));}
    }
    Ok((docs0fn,documented_params))
}


/// extract the documented generic parameters from a collection of generics as
/// a collection of documented identifiers
/// This will also remove all the doc comments from the collection of generics, but
/// will leave all the other attributes untouched.
pub fn extract_documented_generics(
    generics: &'_ mut Generics,
) -> Result<Vec<DocumentedIdent<'_>>, syn::Error> {
    let mut documented_generics = Vec::with_capacity(generics.params.len());
    for param in generics.params.iter_mut() {
        let (ident, attrs) = match param {
            syn::GenericParam::Lifetime(lif) => (&lif.lifetime.ident, &mut lif.attrs),
            syn::GenericParam::Type(ty) => (&ty.ident, &mut ty.attrs),
            syn::GenericParam::Const(con) => (&con.ident, &mut con.attrs),
        };
        let docs = extract_doc_attrs(attrs);
        if !docs.is_empty() {
            documented_generics.push(DocumentedIdent::new(ident, docs))
        }
    }

    Ok(documented_generics)
}


/// Same as extract_documented_generics, but shifts all docs by -1
/// Also allows splitting the last generic's doc into 2: belonging to the last generic (after ///!) and to the previous one
/// fn with_lifetimes_pos <
///   'a    ,/// a lifetime
///   S     ,
///   T     ,/// doc for T line 1
///          /// doc for T line 2
///          ///! a const generic
///   const N: usize, // it's a syntax error to add doc comments at the end
/// >(){}
pub fn extract_documented_generics_shift_up(generics: &'_ mut Generics,) -> Result<Vec<DocumentedIdent<'_>>, syn::Error> {
    let mut doc_gen = Vec::with_capacity(generics.params.len());

    let mut id_prev  :Option<     &Ident     > = None;
    let mut id_last  :Option<     &Ident     > = None;
    let mut id_only  :Option<     &Ident     > = None;
    let mut docs_last:       Vec::<Attribute> = vec![];
    let mut i   :usize = 0; // track the iter position for args with non-empty docs to allow merging pre doc→1st and 2nd gen→1st
    for (pos,param) in generics.params.iter_mut().with_position() {
        i += 1;
        let (id, attrs) = match param {
            syn::GenericParam::Lifetime(lif) => (&lif.lifetime.ident, &mut lif.attrs), // id=a attrs=..."doc" ...lit="a lifetime"
            syn::GenericParam::Type    (ty ) => (&ty          .ident, &mut ty .attrs), // id=T
            syn::GenericParam::Const   (con) => (&con         .ident, &mut con.attrs), // id=N
        };
        let mut docs = extract_doc_attrs(attrs);
        if !docs.is_empty() {
            match pos {
                IPos::Only   => {doc_gen.push(DocumentedIdent::new(id, docs));},
                IPos::First  => {doc_gen.push(DocumentedIdent::new(id, docs));}, // pre-gen docs go to the 1st par as in prefixed docs
                IPos::Middle => {let id_prev = id_prev.take().expect("saved prev id");
                    if i==2 {if let Some(mut doc_gen1) = doc_gen.pop() {
                    doc_gen1.docs.append(&mut docs); doc_gen.push(doc_gen1                           )  ;  // gen2 docs append to  gen1
                    } else {                         doc_gen.push(DocumentedIdent::new(id_prev, docs));};  //   … or    become new gen1
                    } else {                         doc_gen.push(DocumentedIdent::new(id_prev, docs));};
                    },// gen3+ as usual
                IPos::Last   => {id_last = Some(id); docs_last = docs;break;},
            }; // ↓ don't set on last item, break before
        }; id_prev = Some(id); // save id even without docs since next docs might need to be split-attached to it
    }
    if let Some(id_last) = id_last { // on ///! split the docs between 2 generics, removing !
        let (docs2prev,docs2last) = split_doc_in2(docs_last);
        if ! docs2prev.is_empty() {doc_gen.push(DocumentedIdent::new(id_prev.expect("saved prev ident"), docs2prev))};
        if ! docs2last.is_empty() {doc_gen.push(DocumentedIdent::new(id_last                           , docs2last))};
    }
    Ok(doc_gen)
}

/// make a documentation block, which is a markdown list of
/// **<caption>**
///
/// * `ident`: doc-comments
/// * `ident2`: doc-comments
/// * ...
///
/// returns an empty token stream if the list of idents is empty
pub fn make_doc_block<S>(
    caption: S,
    documented_idents: Vec<DocumentedIdent<'_>>,
) -> Option<TokenStream>
where
    S: AsRef<str>,
{
    let has_documented_idents = !documented_idents.is_empty();

    let list = documented_idents.into_iter().map(|param| {
        let mut docs_iter = param.docs.iter();
        // we always have at least one doc attribute because otherwise we
        // would not have inserted this pair into the parameter docs in the
        // first place
        let first = docs_iter
            .next()
            .expect("unexpectedly encountered empty doc list");

        let first_line = prepend_to_doc_attribute(&format!(" * `{}`:", param.ident), first);

        // we just need to indent the other lines, if they exist
        let next_lines = docs_iter.map(|attr| prepend_to_doc_attribute("   ", attr));
        quote! {
            #first_line
            #(#next_lines)*
        }
    });

    let caption = format!(" **{}**:", caption.as_ref());

    if has_documented_idents {
        Some(quote! {
        #[doc=""]
        #[doc=#caption]
        #[doc=""]
        #(#list)*
        })
    } else {
        None
    }
}
