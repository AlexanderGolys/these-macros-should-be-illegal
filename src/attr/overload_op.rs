//! Ownership and assignment overloads of an operator implemented for references.

use crate::helpers::arguments::NoArguments;
use crate::helpers::fresh_name::{fresh_name, written_identifiers};
use crate::helpers::ungroup;
use proc_macro2::{TokenStream, TokenTree};
use quote::{format_ident, quote};
use syn::{
    Attribute, BoundLifetimes, Error, GenericArgument, GenericParam, Generics, Ident,
    ImplItem, ImplItemFn, ItemImpl, Lifetime, Path, PathArguments, ReceiverKind, Token, Type,
    TypeParamBound,
    WherePredicate, parse_quote, parse2, punctuated::Punctuated,
};

/// Name of the associated type holding an operator's result type.
const OUTPUT_ASSOCIATED_TYPE: &str = "Output";
/// Suffix naming the augmented-assignment trait of an operator trait.
const ASSIGNMENT_TRAIT_SUFFIX: &str = "Assign";
/// Suffix naming the augmented-assignment method of an operator method.
const ASSIGNMENT_METHOD_SUFFIX: &str = "_assign";
/// Module gathering the names an assignment impl resolves its trait through.
const ASSIGNMENT_SCOPE_MODULE: &str = "__assignment_scope";
/// Lifetime quantifying the borrows in the bound an assignment impl checks itself by.
const OPERAND_LIFETIME: &str = "operand";

/// One operator implemented for references, decomposed into what its overloads reuse.
///
/// Everything here is read off the impl, which is why the attribute takes no
/// arguments. The operands are stored owned, because every generated impl rebuilds
/// the borrows it needs with elided lifetimes rather than repeating the ones written
/// by hand: a lifetime naming only a stripped outer reference has no place left in
/// the generated header, where rustc would reject it as unconstrained.
struct ReferenceOperator {
    /// Outer attributes repeated on every generated impl.
    attrs: Vec<Attribute>,
    /// Generic parameters and predicates of the impl.
    generics: Generics,
    /// Implemented trait with the operand argument removed.
    trait_base: Path,
    /// Name of the operator method.
    method: Ident,
    /// Left operand, without the reference the impl borrows it through.
    lhs: Type,
    /// Right operand without its reference, absent for a unary operator.
    rhs: Option<Type>,
    /// Result type, taken from the impl's own associated type.
    output: Type,
    /// Impl items other than the operator method, repeated on the value overloads.
    associated_items: Vec<ImplItem>,
}

macro_docs! {
    /// Repeats an operator implemented for references across its owned forms.
    ///
    /// From `impl Op<&Rhs> for &Lhs` it generates `Op<&Rhs>` and `Op<Rhs>` for `Lhs`,
    /// and, when the result is `Lhs` itself, both `OpAssign` impls. Every generated
    /// impl delegates to the attributed one; nothing is cloned.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::ops::Add;
    /// use these_macros_should_be_illegal::overload_op;
    ///
    /// #[derive(Debug, PartialEq)]
    /// struct Vector(f64, f64);
    ///
    /// #[overload_op]
    /// impl Add<&Vector> for &Vector {
    ///     type Output = Vector;
    ///
    ///     fn add(self, rhs: &Vector) -> Vector {
    ///         Vector(self.0 + rhs.0, self.1 + rhs.1)
    ///     }
    /// }
    ///
    /// let mut total = Vector(1.0, 2.0) + Vector(3.0, 4.0);
    /// total += &Vector(1.0, 1.0);
    /// assert_eq!(total, Vector(5.0, 7.0));
    /// ```
}

/// Repeats an operator implemented for references across its owned operand forms.
pub fn overload_op(arguments: TokenStream, item: TokenStream) -> TokenStream {
    let result = parse2::<NoArguments>(arguments)
        .and_then(|NoArguments| parse2::<ItemImpl>(item))
        .and_then(|item| expand_overloads(&item));

    result.unwrap_or_else(Error::into_compile_error)
}

/// Emits the attributed impl unchanged, followed by every overload it implies.
fn expand_overloads(item: &ItemImpl) -> syn::Result<TokenStream> {
    let overloads = ReferenceOperator::from_impl(item)?.overloads();

    Ok(quote! {
        #item
        #overloads
    })
}

impl ReferenceOperator {
    /// Decomposes one operator impl whose operands are written as references.
    fn from_impl(item: &ItemImpl) -> syn::Result<Self> {
        // `default impl` and negative impls are not operator implementations to repeat.
        item.modifiers.require_empty()?;
        let (trait_path, _) = item.trait_.as_ref().ok_or_else(|| {
            Error::new_spanned(
                &item.self_ty,
                "an operator overload is generated from a trait impl",
            )
        })?;

        let lhs = shared_referent(&item.self_ty).ok_or_else(|| {
            Error::new_spanned(
                &item.self_ty,
                "the operator must be implemented for a shared reference, as `impl Add<&T> for &T`",
            )
        })?;

        let (trait_base, trait_operand) = split_trait_operand(trait_path)?;
        let (method, associated_items) = split_operator_method(item)?;
        let rhs = right_operand(&method, trait_operand, &lhs, trait_path)?;
        let output = declared_output(&associated_items, trait_path)?;

        Ok(Self {
            attrs: item.attrs.clone(),
            generics: item.generics.clone(),
            trait_base,
            method: method.sig.ident.clone(),
            lhs,
            rhs,
            output,
            associated_items,
        })
    }

    /// Builds every overload the borrowed operands imply.
    fn overloads(&self) -> TokenStream {
        let Some(rhs) = self.rhs.as_ref() else {
            return self.owned_unary();
        };

        let borrowed: Type = parse_quote!(&#rhs);
        let borrowed_operand = self.owned_binary(&borrowed, &quote!(rhs));
        let owned_operand = self.owned_binary(rhs, &quote!(&rhs));
        let mut overloads = quote!(#borrowed_operand #owned_operand);

        if self.assigns_in_place() {
            let borrowed_operand = self.assignment(&borrowed, &quote!(rhs));
            let owned_operand = self.assignment(rhs, &quote!(&rhs));
            overloads.extend(quote!(#borrowed_operand #owned_operand));
        }

        overloads
    }

    /// Reports whether the operator returns its own left operand type.
    ///
    /// The test is syntactic, because a macro cannot resolve paths, so a difference in
    /// leading qualifiers alone is let through: an impl writing `crate::Vector` in one
    /// position and `Vector` in the other usually means one type, and silently skipping
    /// both assignment overloads over that is a failure the caller only meets at the `+=`
    /// call site. When the two spellings are distinct types after all, the bound from
    /// `assignment_bound` keeps the generated impls from applying.
    fn assigns_in_place(&self) -> bool {
        let Self { output, lhs, .. } = self;
        same_written_type(output, lhs)
    }

    /// Fully qualified path to the attributed method, disambiguating the overloads.
    ///
    /// The borrows are rebuilt here with elided lifetimes, so a body never names a
    /// lifetime that its own header dropped.
    fn primitive_call(&self) -> TokenStream {
        let Self {
            trait_base,
            method,
            lhs,
            rhs,
            ..
        } = self;
        let operator = match rhs {
            Some(rhs) => quote!(#trait_base<&#rhs>),
            None => quote!(#trait_base),
        };

        quote!(<&#lhs as #operator>::#method)
    }

    /// Repeats a binary operator for an owned left operand and the given right one.
    fn owned_binary(&self, rhs: &Type, borrow: &TokenStream) -> TokenStream {
        let Self {
            attrs,
            trait_base,
            method,
            lhs,
            output,
            associated_items,
            ..
        } = self;
        let generics = header_generics(&self.generics, &quote!(#trait_base #rhs #lhs #output));
        let (impl_generics, _, where_clause) = generics.split_for_impl();
        let call = self.primitive_call();

        quote! {
            #(#attrs)*
            impl #impl_generics #trait_base<#rhs> for #lhs #where_clause {
                #(#associated_items)*

                fn #method(self, rhs: #rhs) -> #output {
                    #call(&self, #borrow)
                }
            }
        }
    }

    /// Repeats a unary operator for an owned operand.
    fn owned_unary(&self) -> TokenStream {
        let Self {
            attrs,
            trait_base,
            method,
            lhs,
            output,
            associated_items,
            ..
        } = self;
        let generics = header_generics(&self.generics, &quote!(#trait_base #lhs #output));
        let (impl_generics, _, where_clause) = generics.split_for_impl();
        let call = self.primitive_call();

        quote! {
            #(#attrs)*
            impl #impl_generics #trait_base for #lhs #where_clause {
                #(#associated_items)*

                fn #method(self) -> #output {
                    #call(&self)
                }
            }
        }
    }

    /// Derives the augmented assignment of a binary operator as `left = &left op right`.
    fn assignment(&self, rhs: &Type, borrow: &TokenStream) -> TokenStream {
        let Self {
            attrs,
            trait_base,
            method,
            lhs,
            ..
        } = self;
        let mut generics = header_generics(&self.generics, &quote!(#trait_base #rhs #lhs));
        generics
            .make_where_clause()
            .predicates
            .push(self.assignment_bound());
        let (impl_generics, _, where_clause) = generics.split_for_impl();
        let call = self.primitive_call();
        let assigning_trait = suffixed_trait(trait_base);
        let assigning_method = format_ident!(
            "{method}{ASSIGNMENT_METHOD_SUFFIX}",
            span = self.method.span()
        );
        let assignment = |assigning_trait: TokenStream| {
            quote! {
                #(#attrs)*
                impl #impl_generics #assigning_trait<#rhs> for #lhs #where_clause {
                    fn #assigning_method(&mut self, rhs: #rhs) {
                        *self = #call(&*self, #borrow);
                    }
                }
            }
        };

        // A qualified operator trait locates its assignment trait beside itself.
        if trait_base.leading_colon.is_some() || trait_base.segments.len() > 1 {
            return assignment(quote!(#assigning_trait));
        }

        // A bare operator trait was imported under that name, while the assignment trait
        // derived from it need not have been, so that name is supplied by a module
        // gathering both `core::ops` and the caller's own scope. Only the trait is named
        // through it: the impl itself stays in the caller's scope, which is the only place
        // its operand types are certain to resolve, as a module cannot see items declared
        // in a function body. The globs stay inside that module for the same reason in
        // reverse: beside the impl, `core::ops` would shadow operand types such as
        // `Range`, `Bound` or `ControlFlow`.
        let scope = format_ident!("{ASSIGNMENT_SCOPE_MODULE}", span = self.method.span());
        let scoped_impl = assignment(quote!(#scope::#assigning_trait));

        quote! {
            const _: () = {
                #[allow(unused_imports)]
                mod #scope {
                    pub use super::*;
                    pub use ::core::ops::*;
                }

                #scoped_impl
            };
        }
    }

    /// States that the operator returns its left operand type, which assignment relies on.
    ///
    /// `assigns_in_place` compares types as written, and two spellings differing only in
    /// leading qualifiers may still name two distinct types. Bounding the impl by the
    /// operator itself settles that after resolution: for distinct types the bound fails
    /// and the assignment impl never applies, instead of failing to compile.
    fn assignment_bound(&self) -> WherePredicate {
        let Self {
            generics,
            trait_base,
            lhs,
            rhs,
            ..
        } = self;
        let taken = written_identifiers(quote!(#generics));
        let lifetime = Lifetime::new(
            &format!("'{}", fresh_name(OPERAND_LIFETIME, &taken)),
            self.method.span(),
        );

        parse_quote! {
            for<#lifetime> &#lifetime #lhs: #trait_base<&#lifetime #rhs, Output = #lhs>
        }
    }
}

/// Separates the implemented trait from the right operand it names.
fn split_trait_operand(path: &Path) -> syn::Result<(Path, Option<Type>)> {
    let mut base = path.clone();
    let segment = base
        .segments
        .last_mut()
        .ok_or_else(|| Error::new_spanned(path, "expected a named operator trait"))?;

    let operand = match std::mem::replace(&mut segment.arguments, PathArguments::None) {
        PathArguments::None => None,
        PathArguments::AngleBracketed(arguments) => {
            let mut operands = arguments.args.iter().filter_map(|argument| match argument {
                GenericArgument::Type(operand) => Some(operand.clone()),
                _ => None,
            });
            let operand = operands
                .next()
                .ok_or_else(|| Error::new_spanned(&arguments, "expected the right operand type"))?;
            if operands.next().is_some() {
                return Err(Error::new_spanned(
                    &arguments,
                    "expected exactly one right operand type",
                ));
            }
            Some(operand)
        }
        PathArguments::Parenthesized(arguments) => {
            return Err(Error::new_spanned(
                arguments,
                "expected an operator trait rather than a callable bound",
            ));
        }
    };

    Ok((base, operand))
}

/// Separates the single operator method from the impl's other items.
fn split_operator_method(item: &ItemImpl) -> syn::Result<(ImplItemFn, Vec<ImplItem>)> {
    let mut methods = item.items.iter().filter_map(|entry| match entry {
        ImplItem::Fn(method) => Some(method.clone()),
        _ => None,
    });
    let method = methods
        .next()
        .ok_or_else(|| Error::new_spanned(&item.self_ty, "expected the operator method"))?;
    if let Some(extra) = methods.next() {
        return Err(Error::new_spanned(
            extra.sig.ident,
            "expected exactly one operator method in the impl",
        ));
    }
    if !matches!(method.sig.receiver(), Some(receiver)
        if matches!(receiver.kind, ReceiverKind::Value))
    {
        return Err(Error::new_spanned(
            &method.sig,
            "an operator on a borrowed operand takes `self`, which is that reference",
        ));
    }

    let associated_items = item
        .items
        .iter()
        .filter(|entry| !matches!(entry, ImplItem::Fn(_)))
        .cloned()
        .collect();

    Ok((method, associated_items))
}

/// Reads the right operand, which the method's arity tells apart from a unary operator.
///
/// A trait naming no operand still has one when its method takes an argument, because
/// the right operand then defaults to `Self`, which is the borrowed left operand.
fn right_operand(
    method: &ImplItemFn,
    trait_operand: Option<Type>,
    lhs: &Type,
    trait_path: &Path,
) -> syn::Result<Option<Type>> {
    match (method.sig.inputs.len() - 1, trait_operand) {
        (0, None) => Ok(None),
        (0, Some(operand)) => Err(Error::new_spanned(
            operand,
            "the operator method takes no right operand",
        )),
        (1, None) => Ok(Some(lhs.clone())),
        (1, Some(operand)) => shared_referent(&operand).map(Some).ok_or_else(|| {
            Error::new_spanned(
                operand,
                "the right operand must be written as a shared reference",
            )
        }),
        _ => Err(Error::new_spanned(
            trait_path,
            "an operator method takes at most one right operand",
        )),
    }
}

/// Reads the type behind a shared reference, the only borrow every overload can re-create.
///
/// A mutable borrow is not one of them: the generated bodies borrow their owned operands
/// with `&`, which an impl for `&mut T` does not accept.
fn shared_referent(operand: &Type) -> Option<Type> {
    match ungroup::ty(operand) {
        Type::Reference(borrowed) if borrowed.mutability.is_none() => {
            Some((*borrowed.elem).clone())
        }
        _ => None,
    }
}

/// Reads the result type from the impl's own associated type.
fn declared_output(associated_items: &[ImplItem], trait_path: &Path) -> syn::Result<Type> {
    associated_items
        .iter()
        .find_map(|entry| match entry {
            ImplItem::Type(associated) if associated.ident == OUTPUT_ASSOCIATED_TYPE => {
                Some(associated.ty.clone())
            }
            _ => None,
        })
        .ok_or_else(|| {
            Error::new_spanned(
                trait_path,
                format!("expected the impl to declare `type {OUTPUT_ASSOCIATED_TYPE}`"),
            )
        })
}

/// Names the augmented-assignment trait of an operator trait.
fn suffixed_trait(trait_base: &Path) -> Path {
    let mut suffixed = trait_base.clone();
    if let Some(segment) = suffixed.segments.last_mut() {
        segment.ident = format_ident!(
            "{}{ASSIGNMENT_TRAIT_SUFFIX}",
            segment.ident,
            span = segment.ident.span()
        );
    }
    suffixed
}

/// Drops the lifetime parameters a generated header no longer names, keeping its bounds.
///
/// A bound mentioning a dropped lifetime is rewritten rather than deleted. The generated
/// impls still need it, only for the fresh borrow their bodies create rather than for the
/// lifetime that was written, so the bound becomes higher-ranked over that lifetime, which
/// is exactly what the body requires. Outlives bounds are the exception: `for<'a> T: 'a`
/// would demand `'static` instead of the bound that was written, so they are dropped with
/// the lifetime that stated them.
fn header_generics(generics: &Generics, header: &TokenStream) -> Generics {
    let dropped: Vec<Ident> = generics
        .lifetimes()
        .map(|parameter| parameter.lifetime.ident.clone())
        .filter(|lifetime| !names_lifetime(header, lifetime))
        .collect();
    if dropped.is_empty() {
        return generics.clone();
    }

    let mut pruned = generics.clone();
    let mut rehomed: Vec<WherePredicate> = Vec::new();
    pruned.params = generics
        .params
        .iter()
        .filter_map(|parameter| match parameter {
            GenericParam::Lifetime(parameter) => (!dropped.contains(&parameter.lifetime.ident))
                .then(|| GenericParam::Lifetime(parameter.clone())),
            GenericParam::Type(parameter) => {
                let mut parameter = parameter.clone();
                let (kept, quantified) = split_bounds(&parameter.bounds, &dropped);
                parameter.bounds = kept;
                if parameter.bounds.is_empty() {
                    parameter.colon_token = None;
                }
                if !quantified.is_empty() {
                    let ident = &parameter.ident;
                    let binder = binder_for(&dropped, &quote!(#(#quantified)+*));
                    rehomed.push(parse_quote!(#binder #ident: #(#quantified)+*));
                }
                Some(GenericParam::Type(parameter))
            }
            other => Some(other.clone()),
        })
        .collect();

    if let Some(where_clause) = pruned.where_clause.as_mut() {
        where_clause.predicates = where_clause
            .predicates
            .iter()
            .filter_map(|predicate| requantified(predicate, &dropped))
            .collect();
    }
    if !rehomed.is_empty() {
        pruned.make_where_clause().predicates.extend(rehomed);
    }
    if pruned
        .where_clause
        .as_ref()
        .is_some_and(|where_clause| where_clause.predicates.is_empty())
    {
        pruned.where_clause = None;
    }

    pruned
}

/// Separates bounds a pruned header keeps as written from those needing a binder.
///
/// An outlives bound on a dropped lifetime appears in neither: it constrained the
/// lifetime that is going away, and has nothing left to say about the generated header.
fn split_bounds(
    bounds: &Punctuated<TypeParamBound, Token![+]>,
    dropped: &[Ident],
) -> (Punctuated<TypeParamBound, Token![+]>, Vec<TypeParamBound>) {
    let mut kept = Punctuated::new();
    let mut quantified = Vec::new();

    for bound in bounds {
        if !names_any_lifetime(&quote!(#bound), dropped) {
            kept.push(bound.clone());
        } else if matches!(bound, TypeParamBound::Trait(_)) {
            quantified.push(bound.clone());
        }
    }

    (kept, quantified)
}

/// Rewrites one predicate so it survives the loss of the lifetimes it names.
fn requantified(predicate: &WherePredicate, dropped: &[Ident]) -> Option<WherePredicate> {
    match predicate {
        WherePredicate::Type(predicate) => {
            let mut predicate = predicate.clone();
            let (kept, quantified) = split_bounds(&predicate.bounds, dropped);
            predicate.bounds = kept;
            predicate.bounds.extend(quantified);
            if predicate.bounds.is_empty() {
                return None;
            }
            if let Some(binder) = binder_for(dropped, &quote!(#predicate)) {
                predicate.lifetimes = Some(match predicate.lifetimes.take() {
                    Some(mut existing) => {
                        existing.lifetimes.extend(binder.lifetimes);
                        existing
                    }
                    None => binder,
                });
            }
            Some(WherePredicate::Type(predicate))
        }
        other => (!names_any_lifetime(&quote!(#other), dropped)).then(|| other.clone()),
    }
}

/// Builds the `for<...>` binder naming every dropped lifetime a bound still mentions.
fn binder_for(dropped: &[Ident], tokens: &TokenStream) -> Option<BoundLifetimes> {
    let named: Vec<Lifetime> = dropped
        .iter()
        .filter(|lifetime| names_lifetime(tokens, lifetime))
        .map(|lifetime| Lifetime::new(&format!("'{lifetime}"), lifetime.span()))
        .collect();

    (!named.is_empty()).then(|| parse_quote!(for<#(#named),*>))
}

/// Reports whether a token stream names any of the given lifetimes.
fn names_any_lifetime(tokens: &TokenStream, lifetimes: &[Ident]) -> bool {
    lifetimes
        .iter()
        .any(|lifetime| names_lifetime(tokens, lifetime))
}

/// Compares two written types, tolerating a difference in leading path qualifiers.
///
/// One path being the other with extra leading segments is the only difference treated
/// as immaterial. Two equally long paths that differ anywhere stay distinct, so
/// `a::Vector` and `b::Vector` are still two types.
fn same_written_type(left: &Type, right: &Type) -> bool {
    if quote!(#left).to_string() == quote!(#right).to_string() {
        return true;
    }

    match (ungroup::ty(left), ungroup::ty(right)) {
        (Type::Path(left), Type::Path(right)) if left.qself.is_none() && right.qself.is_none() => {
            shares_path_suffix(&left.path, &right.path)
        }
        _ => false,
    }
}

/// Reports whether one path is the other written with extra leading segments.
fn shares_path_suffix(left: &Path, right: &Path) -> bool {
    let rendered = |path: &Path| -> Vec<String> {
        path.segments
            .iter()
            .map(|segment| quote!(#segment).to_string())
            .collect()
    };
    let left = rendered(left);
    let right = rendered(right);
    let (shorter, longer) = if left.len() <= right.len() {
        (&left, &right)
    } else {
        (&right, &left)
    };

    shorter.len() < longer.len() && longer.ends_with(shorter)
}

/// Reports whether a token stream names one lifetime.
fn names_lifetime(tokens: &TokenStream, lifetime: &Ident) -> bool {
    let mut quoted = false;
    for tree in tokens.clone() {
        match tree {
            TokenTree::Group(group) => {
                if names_lifetime(&group.stream(), lifetime) {
                    return true;
                }
                quoted = false;
            }
            TokenTree::Punct(punct) => quoted = punct.as_char() == '\'',
            TokenTree::Ident(ident) => {
                if quoted && &ident == lifetime {
                    return true;
                }
                quoted = false;
            }
            TokenTree::Literal(_) => quoted = false,
        }
    }

    false
}

#[cfg(test)]
mod tests {
    //! Unit tests for the operator read off the impl and the shapes it rejects.

    use super::*;

    /// Expands one attributed impl, or returns its rejection message.
    fn expand(item: &str) -> Result<String, String> {
        let item = parse2::<ItemImpl>(item.parse().unwrap()).unwrap();

        expand_overloads(&item)
            .map(|expansion| expansion.to_string())
            .map_err(|error| error.to_string())
    }

    /// Expands the ordinary binary operator on references.
    fn expand_addition() -> String {
        expand("impl Add<&V> for &V { type Output = V; fn add(self, rhs: &V) -> V { todo!() } }")
            .unwrap()
    }

    /// Repeats a binary operator without introducing the borrowed left operand.
    #[test]
    fn generates_two_ownership_overloads_of_a_binary_operator() {
        let expansion = expand_addition();

        assert!(expansion.contains("impl Add < & V > for V"), "{expansion}");
        assert!(expansion.contains("impl Add < V > for V"), "{expansion}");
    }

    /// Emits the attributed impl unchanged ahead of the overloads.
    #[test]
    fn emits_the_attributed_impl_unchanged() {
        let expansion = expand_addition();

        assert!(
            expansion.starts_with("impl Add < & V > for & V { type Output = V ;"),
            "{expansion}"
        );
    }

    /// Derives both augmented assignments from an operator returning its own operand.
    #[test]
    fn derives_augmented_assignment_overloads() {
        let expansion = expand_addition();

        assert!(
            expansion.contains("impl __assignment_scope :: AddAssign < & V > for V"),
            "{expansion}"
        );
        assert!(
            expansion.contains("impl __assignment_scope :: AddAssign < V > for V"),
            "{expansion}"
        );
        assert!(
            expansion.contains("* self = < & V as Add < & V > > :: add (& * self , rhs)"),
            "{expansion}"
        );
    }

    /// Bounds the assignment by the operator returning exactly the left operand type.
    #[test]
    fn bounds_the_assignment_by_its_own_operator() {
        let expansion = expand_addition();

        assert!(
            expansion.contains(
                "where for < 'operand > & 'operand V : Add < & 'operand V , Output = V >"
            ),
            "{expansion}"
        );
    }

    /// Names the bound's lifetime around the lifetimes the impl already declares.
    #[test]
    fn names_the_bound_lifetime_around_declared_ones() {
        let expansion = expand(
            "impl<'operand> Add<&'operand V> for &'operand V { type Output = V; fn add(self, rhs: &'operand V) -> V { todo!() } }",
        )
        .unwrap();

        assert!(expansion.contains("for < 'operand2 >"), "{expansion}");
    }

    /// Names the assignment trait beside a qualified operator trait, without a scope.
    #[test]
    fn locates_the_assignment_trait_beside_a_qualified_operator() {
        let expansion = expand(
            "impl ::core::ops::Add<&V> for &V { type Output = V; fn add(self, rhs: &V) -> V { todo!() } }",
        )
        .unwrap();

        assert!(
            expansion.contains("impl :: core :: ops :: AddAssign < & V > for V"),
            "{expansion}"
        );
        assert!(!expansion.contains(ASSIGNMENT_SCOPE_MODULE), "{expansion}");
    }

    /// Leaves assignment alone when the operator does not return its left operand.
    #[test]
    fn skips_assignment_for_an_operator_changing_type() {
        let expansion = expand(
            "impl Sub<&P> for &P { type Output = V; fn sub(self, rhs: &P) -> V { todo!() } }",
        )
        .unwrap();

        assert!(!expansion.contains("SubAssign"), "{expansion}");
    }

    /// Takes the right operand from `Self` when the trait names none.
    #[test]
    fn takes_an_elided_right_operand_from_the_left_one() {
        let expansion =
            expand("impl Add for &V { type Output = V; fn add(self, rhs: &V) -> V { todo!() } }")
                .unwrap();

        assert!(expansion.contains("impl Add < & V > for V"), "{expansion}");
        assert!(expansion.contains("impl Add < V > for V"), "{expansion}");
    }

    /// Generates the single owned form of a unary operator.
    #[test]
    fn generates_the_owned_form_of_a_unary_operator() {
        let expansion =
            expand("impl Neg for &V { type Output = V; fn neg(self) -> V { todo!() } }").unwrap();

        assert!(expansion.contains("impl Neg for V"), "{expansion}");
        assert!(
            expansion.contains("< & V as Neg > :: neg (& self)"),
            "{expansion}"
        );
        assert!(!expansion.contains("NegAssign"), "{expansion}");
    }

    /// Drops a lifetime naming only the operand reference that was removed.
    #[test]
    fn drops_the_lifetime_of_a_removed_operand_reference() {
        let expansion = expand(
            "impl<'a> Add<&'a V> for &'a V { type Output = V; fn add(self, rhs: &'a V) -> V { todo!() } }",
        )
        .unwrap();

        assert!(expansion.contains("impl Add < V > for V"), "{expansion}");
        assert!(
            !expansion.contains("impl < 'a > Add < V > for V"),
            "{expansion}"
        );
    }

    /// Repeats the generic parameters and predicates of the attributed impl.
    #[test]
    fn retains_generic_parameters_and_predicates() {
        let expansion = expand(
            "impl<T> Add<&Pair<T>> for &Pair<T> where T: Copy { type Output = Pair<T>; fn add(self, rhs: &Pair<T>) -> Pair<T> { todo!() } }",
        )
        .unwrap();

        assert!(
            expansion.contains("impl < T > Add < & Pair < T > > for Pair < T > where T : Copy"),
            "{expansion}"
        );
    }

    /// Rejects an inherent impl, which names no operator to overload.
    #[test]
    fn rejects_an_inherent_impl() {
        let error = expand("impl V { fn add(self, rhs: &V) -> V { todo!() } }").unwrap_err();

        assert!(error.contains("trait impl"), "{error}");
    }

    /// Checks the left operand is borrowed rather than assuming it.
    #[test]
    fn rejects_an_owned_left_operand() {
        let error = expand(
            "impl Add<&V> for V { type Output = V; fn add(self, rhs: &V) -> V { todo!() } }",
        )
        .unwrap_err();

        assert!(
            error.contains("implemented for a shared reference"),
            "{error}"
        );
    }

    /// Rejects a mutable left operand, which the generated shared borrows cannot reach.
    #[test]
    fn rejects_a_mutable_left_operand() {
        let error = expand(
            "impl Add<&V> for &mut V { type Output = V; fn add(self, rhs: &V) -> V { todo!() } }",
        )
        .unwrap_err();

        assert!(
            error.contains("implemented for a shared reference"),
            "{error}"
        );
    }

    /// Checks the right operand is borrowed rather than assuming it.
    #[test]
    fn rejects_an_owned_right_operand() {
        let error =
            expand("impl Add<V> for &V { type Output = V; fn add(self, rhs: V) -> V { todo!() } }")
                .unwrap_err();

        assert!(error.contains("written as a shared reference"), "{error}");
    }

    /// Rejects a mutable right operand, which the generated shared borrows cannot reach.
    #[test]
    fn rejects_a_mutable_right_operand() {
        let error = expand(
            "impl Add<&mut V> for &V { type Output = V; fn add(self, rhs: &mut V) -> V { todo!() } }",
        )
        .unwrap_err();

        assert!(error.contains("written as a shared reference"), "{error}");
    }

    /// Rejects the operators that read through a borrow rather than consuming one.
    ///
    /// `Index` and `Deref` take `&self` and return a reference into it, so they have
    /// no owned operand form to generate and no augmented assignment to name: the
    /// standard library pairs them with `IndexMut` and `DerefMut` rather than with an
    /// `Assign` trait. The receiver check turns them away before either question.
    #[test]
    fn rejects_an_operator_reading_through_a_borrow() {
        let error = expand(
            "impl Index<&Key> for &V { type Output = Slot; fn index(&self, key: &Key) -> &Slot { todo!() } }",
        )
        .unwrap_err();

        assert!(error.contains("takes `self`"), "{error}");
    }

    /// Rejects a receiver that is not the borrowed left operand itself.
    #[test]
    fn rejects_a_borrowed_receiver() {
        let error = expand(
            "impl Add<&V> for &V { type Output = V; fn add(&self, rhs: &V) -> V { todo!() } }",
        )
        .unwrap_err();

        assert!(error.contains("takes `self`"), "{error}");
    }

    /// Rejects an impl carrying more than the single operator method.
    #[test]
    fn rejects_more_than_one_method() {
        let error = expand(
            "impl Add<&V> for &V { type Output = V; fn add(self, rhs: &V) -> V { todo!() } fn extra(self) {} }",
        )
        .unwrap_err();

        assert!(error.contains("exactly one operator method"), "{error}");
    }

    /// Rejects an impl that never says what the operator returns.
    #[test]
    fn rejects_a_missing_result_type() {
        let error =
            expand("impl Add<&V> for &V { fn add(self, rhs: &V) -> V { todo!() } }").unwrap_err();

        assert!(error.contains(OUTPUT_ASSOCIATED_TYPE), "{error}");
    }

    /// Rejects arguments, since the impl already says everything.
    #[test]
    fn rejects_arguments() {
        let error = overload_op(quote!(bin = Add), quote!()).to_string();

        assert!(error.contains("takes no arguments"), "{error}");
    }

    /// Negative and `default` impls carry modifiers an overload cannot repeat.
    #[test]
    fn rejects_impl_modifiers() {
        let error = overload_op(quote!(), quote!(impl !Add<&V> for &V {})).to_string();

        assert!(error.contains("compile_error"), "{error}");
    }
}
