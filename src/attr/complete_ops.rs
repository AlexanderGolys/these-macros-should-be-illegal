//! Operators that follow from the ones a type already has.

use crate::helpers::fresh_name::{fresh_name, written_identifiers};
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{
    Error, Expr, Generics, Ident, Item, Lifetime, Token, Type, WherePredicate,
    parse::{Parse, ParseStream},
    parse_quote, parse2,
};

/// Operator completed from an addition and the negation of its right operand.
const SUBTRACTION: &str = "Sub";
/// Operator completed from a scalar multiplication.
const NEGATION: &str = "Neg";
/// Name of the generic right operand a completed operator introduces.
const RIGHT_OPERAND_PARAMETER: &str = "Rhs";
/// Lifetime of the borrowed left operand a completed operator introduces.
const OPERAND_LIFETIME: &str = "operand";

macro_docs! {
    /// Adds the operators that follow from the ones a type already implements.
    ///
    /// `Sub` completes subtraction from addition and negation; `Neg = <scalar>`
    /// completes negation from multiplication by that scalar.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::ops::{Add, AddAssign, Neg};
    /// use these_macros_should_be_illegal::complete_ops;
    ///
    /// #[derive(Debug, PartialEq)]
    /// #[complete_ops(Sub)]
    /// struct Vector(f64);
    ///
    /// impl Add for Vector {
    ///     type Output = Vector;
    ///
    ///     fn add(self, rhs: Vector) -> Vector {
    ///         Vector(self.0 + rhs.0)
    ///     }
    /// }
    ///
    /// impl AddAssign for Vector {
    ///     fn add_assign(&mut self, rhs: Vector) {
    ///         self.0 += rhs.0;
    ///     }
    /// }
    ///
    /// impl Neg for Vector {
    ///     type Output = Vector;
    ///
    ///     fn neg(self) -> Vector {
    ///         Vector(-self.0)
    ///     }
    /// }
    ///
    /// assert_eq!(Vector(5.0) - Vector(2.0), Vector(3.0));
    /// ```
}

/// Adds the operators that follow from the ones a type already implements.
pub fn complete_ops(arguments: TokenStream, item: TokenStream) -> TokenStream {
    let result = parse2::<SelectedCompletions>(arguments)
        .and_then(|selected| parse2::<Item>(item).map(|item| (selected, item)))
        .and_then(|(selected, item)| expand_completions(&selected, &item));

    result.unwrap_or_else(Error::into_compile_error)
}

/// One operator to complete, with whatever the completion needs to be stated.
enum Completion {
    /// `x - v` is `x + (-v)`, for every negatable `v` the type can add.
    Subtraction,
    /// `-x` is `x` scaled by this value.
    Negation(Box<Expr>),
}

impl Completion {
    /// Name of the operator this completion implements.
    fn operator(&self) -> &'static str {
        match self {
            Self::Subtraction => SUBTRACTION,
            Self::Negation(_) => NEGATION,
        }
    }
}

impl Parse for Completion {
    /// Parses one operator name, and the scalar the negation is scaled by.
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let operator = input.parse::<Ident>()?;
        let scalar = if input.peek(Token![=]) {
            input.parse::<Token![=]>()?;
            Some(Box::new(input.parse::<Expr>()?))
        } else {
            None
        };

        match (operator.to_string().as_str(), scalar) {
            (SUBTRACTION, None) => Ok(Self::Subtraction),
            (SUBTRACTION, Some(_)) => Err(Error::new(
                operator.span(),
                format!("`{SUBTRACTION}` follows from the addition and takes no scalar"),
            )),
            (NEGATION, Some(scalar)) => Ok(Self::Negation(scalar)),
            (NEGATION, None) => Err(Error::new(
                operator.span(),
                format!("`{NEGATION}` is completed by scaling, as `{NEGATION} = <scalar>`"),
            )),
            _ => Err(Error::new(
                operator.span(),
                format!("expected `{SUBTRACTION}` or `{NEGATION} = <scalar>`"),
            )),
        }
    }
}

/// The operators one attribute completes, in the order they are emitted.
struct SelectedCompletions {
    /// Completions named by the attribute.
    completions: Vec<Completion>,
}

impl Parse for SelectedCompletions {
    /// Parses the comma-separated operators, each at most once, negation ahead of subtraction.
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.is_empty() {
            return Err(input.error(format!(
                "expected the operators to complete, such as `{SUBTRACTION}`"
            )));
        }

        let mut completions: Vec<Completion> = Vec::new();
        while !input.is_empty() {
            let span = input.span();
            let completion = input.parse::<Completion>()?;
            if completions
                .iter()
                .any(|completed| completed.operator() == completion.operator())
            {
                return Err(Error::new(
                    span,
                    format!("`{}` is already completed", completion.operator()),
                ));
            }
            completions.push(completion);
            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }

        // A completed negation is what a completed subtraction negates through, so
        // it is emitted first when both are asked for at once.
        completions.sort_by_key(|completion| match completion {
            Completion::Negation(_) => 0,
            Completion::Subtraction => 1,
        });

        Ok(Self { completions })
    }
}



/// Emits the attributed item unchanged, followed by the operators it implies.
fn expand_completions(selected: &SelectedCompletions, item: &Item) -> syn::Result<TokenStream> {
    let (name, generics) = declared_type(item)?;
    let (_, type_generics, where_clause) = generics.split_for_impl();
    let space: Type = parse_quote!(#name #type_generics);
    let taken = written_identifiers(quote!(#name #generics #where_clause));

    let completions = selected
        .completions
        .iter()
        .map(|completion| match completion {
            Completion::Subtraction => subtraction(generics, &space, &taken),
            Completion::Negation(scalar) => negation(generics, &space, scalar),
        })
        .collect::<Vec<_>>();

    Ok(quote! {
        #item
        #(#completions)*
    })
}

/// Reads the name and generic parameters of the attributed declaration.
fn declared_type(item: &Item) -> syn::Result<(&Ident, &Generics)> {
    match item {
        Item::Struct(declaration) => Ok((&declaration.ident, &declaration.generics)),
        Item::Enum(declaration) => Ok((&declaration.ident, &declaration.generics)),
        Item::Union(declaration) => Ok((&declaration.ident, &declaration.generics)),
        Item::Type(declaration) => Ok((&declaration.ident, &declaration.generics)),
        other => Err(Error::new_spanned(
            other,
            "operators are completed on a type declaration",
        )),
    }
}

/// States that `x - v` makes sense wherever `x + v` does and `v` lies in a group.
///
/// The right operand stays generic, so one impl covers every group element the type
/// can add rather than one impl per operand type, and the result is the result of
/// that very addition. `Neg<Output = Rhs>` is what makes the operand a group element
/// rather than merely something negatable: an inverse leaves the group it came from.
/// Nothing about the left operand's own structure is assumed, so a space acted on by
/// a group completes exactly as a group acting on itself does.
///
/// `taken` holds every identifier the declaration writes, which the introduced right
/// operand and borrow lifetime are named around.
fn subtraction(generics: &Generics, space: &Type, taken: &[String]) -> TokenStream {
    let rhs = Ident::new(
        &fresh_name(RIGHT_OPERAND_PARAMETER, taken),
        Span::call_site(),
    );
    let group: WherePredicate = parse_quote!(#rhs: ::core::ops::Neg<Output = #rhs>);
    let lifetime = Lifetime::new(
        &format!("'{}", fresh_name(OPERAND_LIFETIME, taken)),
        Span::call_site(),
    );
    let borrowed: Type = parse_quote!(&#lifetime #space);

    let subtracts = completed_generics(
        generics,
        None,
        &rhs,
        &[group.clone(), parse_quote!(#space: ::core::ops::Add<#rhs>)],
    );
    let (subtracts_generics, _, subtracts_where) = subtracts.split_for_impl();

    let subtracts_borrowed = completed_generics(
        generics,
        Some(&lifetime),
        &rhs,
        &[
            group.clone(),
            parse_quote!(#borrowed: ::core::ops::Add<#rhs>),
        ],
    );
    let (borrowed_generics, _, borrowed_where) = subtracts_borrowed.split_for_impl();

    let assigns = completed_generics(
        generics,
        None,
        &rhs,
        &[group, parse_quote!(#space: ::core::ops::AddAssign<#rhs>)],
    );
    let (assigns_generics, _, assigns_where) = assigns.split_for_impl();

    quote! {
        impl #subtracts_generics ::core::ops::Sub<#rhs> for #space #subtracts_where {
            type Output = <#space as ::core::ops::Add<#rhs>>::Output;

            fn sub(self, rhs: #rhs) -> Self::Output {
                self + -rhs
            }
        }

        impl #borrowed_generics ::core::ops::Sub<#rhs> for #borrowed #borrowed_where {
            type Output = <#borrowed as ::core::ops::Add<#rhs>>::Output;

            fn sub(self, rhs: #rhs) -> Self::Output {
                self + -rhs
            }
        }

        impl #assigns_generics ::core::ops::SubAssign<#rhs> for #space #assigns_where {
            fn sub_assign(&mut self, rhs: #rhs) {
                *self += -rhs;
            }
        }
    }
}

/// States that negation is multiplication by one fixed scalar.
///
/// Unlike the subtraction, this states no bound for the operation it performs. The
/// multiplication needs `&Space: Mul<&Scalar, Output = Space>`, and the scalar arrives
/// as an expression, so its type has no spelling available in the generated header. A
/// generic declaration therefore has to carry that bound itself.
fn negation(generics: &Generics, space: &Type, scalar: &Expr) -> TokenStream {
    let (impl_generics, _, where_clause) = generics.split_for_impl();

    quote! {
        impl #impl_generics ::core::ops::Neg for &#space #where_clause {
            type Output = #space;

            fn neg(self) -> #space {
                ::core::ops::Mul::mul(self, &(#scalar))
            }
        }

        impl #impl_generics ::core::ops::Neg for #space #where_clause {
            type Output = #space;

            fn neg(self) -> #space {
                ::core::ops::Mul::mul(&self, &(#scalar))
            }
        }
    }
}

/// Adds the generic right operand, the borrowed left operand's lifetime, and bounds.
fn completed_generics(
    generics: &Generics,
    lifetime: Option<&Lifetime>,
    rhs: &Ident,
    predicates: &[WherePredicate],
) -> Generics {
    let mut completed = generics.clone();
    if let Some(lifetime) = lifetime {
        completed.params.insert(0, parse_quote!(#lifetime));
    }
    completed.params.push(parse_quote!(#rhs));
    completed
        .make_where_clause()
        .predicates
        .extend(predicates.iter().cloned());

    completed
}

#[cfg(test)]
mod tests {
    //! Unit tests for the selected completions and the operators they state.

    use super::*;

    /// Expands one attributed declaration, or returns its rejection message.
    fn expand(arguments: &str, item: &str) -> Result<String, String> {
        let selected = parse2::<SelectedCompletions>(arguments.parse().unwrap())
            .map_err(|error| error.to_string())?;
        let item = parse2::<Item>(item.parse().unwrap()).unwrap();

        expand_completions(&selected, &item)
            .map(|expansion| expansion.to_string())
            .map_err(|error| error.to_string())
    }

    /// Expands one completion on a plain declaration.
    fn expand_on_space(arguments: &str) -> String {
        expand(arguments, "struct X(f64);").unwrap()
    }

    /// States subtraction over a generic right operand rather than one per operand.
    #[test]
    fn states_subtraction_over_a_generic_right_operand() {
        let expansion = expand_on_space("Sub");

        assert!(
            expansion.contains("impl < Rhs > :: core :: ops :: Sub < Rhs > for X"),
            "{expansion}"
        );
        assert!(expansion.contains("self + - rhs"), "{expansion}");
    }

    /// Takes the result of the addition it delegates to, assuming nothing about it.
    #[test]
    fn takes_the_result_type_from_the_addition() {
        let expansion = expand_on_space("Sub");

        assert!(
            expansion.contains("type Output = < X as :: core :: ops :: Add < Rhs >> :: Output"),
            "{expansion}"
        );
    }

    /// Asks that the operand lie in a group, and that the type add that group.
    ///
    /// `Neg<Output = Rhs>` is the bound that makes the operand a group element: an
    /// inverse has to stay in the group it came from, so a borrowed operand, whose
    /// negation is an owned value, is deliberately not one.
    #[test]
    fn asks_for_a_group_element_and_the_addition() {
        let expansion = expand_on_space("Sub");

        assert!(
            expansion.contains(
                "where Rhs : :: core :: ops :: Neg < Output = Rhs > , \
                 X : :: core :: ops :: Add < Rhs >"
            ),
            "{expansion}"
        );
    }

    /// Completes the borrowed left operand and the augmented assignment as well.
    #[test]
    fn completes_the_borrowed_operand_and_the_assignment() {
        let expansion = expand_on_space("Sub");

        assert!(
            expansion.contains("Sub < Rhs > for & 'operand X"),
            "{expansion}"
        );
        assert!(expansion.contains("SubAssign < Rhs > for X"), "{expansion}");
        assert!(expansion.contains("* self += - rhs"), "{expansion}");
    }

    /// Emits the completed negation ahead of the subtraction that goes through it.
    #[test]
    fn emits_the_negation_before_the_subtraction() {
        let expansion = expand_on_space("Sub, Neg = -1.0f64");
        let negation = expansion
            .find("Neg for & X")
            .expect("negation is completed");
        let subtraction = expansion
            .find("Sub < Rhs >")
            .expect("subtraction is completed");

        assert!(negation < subtraction, "{expansion}");
    }

    /// Scales by the value as written, so its type selects the multiplication.
    #[test]
    fn scales_the_negation_by_the_value_as_written() {
        let expansion = expand_on_space("Neg = -1.0f64");

        assert!(
            expansion.contains("Mul :: mul (self , & (- 1.0f64))"),
            "{expansion}"
        );
    }

    /// Renames the operand parameter rather than shadowing the declaration's own.
    #[test]
    fn avoids_colliding_with_a_declared_parameter() {
        let expansion = expand("Sub", "struct Wrapper<Rhs>(Rhs);").unwrap();

        assert!(
            expansion.contains("impl < Rhs , Rhs2 > :: core :: ops :: Sub < Rhs2 >"),
            "{expansion}"
        );
    }

    /// Keeps the predicates the declaration already carries.
    #[test]
    fn keeps_the_predicates_of_the_declaration() {
        let expansion = expand("Sub", "struct Pair<T>(T, T) where T: Copy;").unwrap();

        assert!(expansion.contains("where T : Copy"), "{expansion}");
        assert!(
            expansion.contains("Rhs : :: core :: ops :: Neg"),
            "{expansion}"
        );
    }

    /// Avoids a const parameter and the declared type's own name, not only type parameters.
    #[test]
    fn avoids_colliding_with_any_written_name() {
        let const_parameter = expand("Sub", "struct Buffer<const Rhs: usize>([u8; Rhs]);").unwrap();
        let own_name = expand("Sub", "struct Rhs(f64);").unwrap();

        assert!(
            const_parameter.contains("impl < const Rhs : usize , Rhs2 >"),
            "{const_parameter}"
        );
        assert!(
            own_name.contains("impl < Rhs2 > :: core :: ops :: Sub < Rhs2 > for Rhs"),
            "{own_name}"
        );
    }

    /// Rejects an operator named twice rather than emitting conflicting impls.
    #[test]
    fn rejects_a_repeated_operator() {
        let subtraction = expand("Sub, Sub", "struct X(f64);").unwrap_err();
        let negation = expand("Neg = -1.0, Neg = 2.0", "struct X(f64);").unwrap_err();

        assert!(
            subtraction.contains("`Sub` is already completed"),
            "{subtraction}"
        );
        assert!(
            negation.contains("`Neg` is already completed"),
            "{negation}"
        );
    }

    /// Rejects an attribute naming no completion at all.
    #[test]
    fn rejects_an_empty_selection() {
        let error = expand("", "struct X(f64);").unwrap_err();

        assert!(error.contains(SUBTRACTION), "{error}");
    }

    /// Rejects a negation with no scalar to scale by.
    #[test]
    fn rejects_a_negation_without_a_scalar() {
        let error = expand("Neg", "struct X(f64);").unwrap_err();

        assert!(error.contains("scaling"), "{error}");
    }

    /// Rejects a scalar on the subtraction, which follows from the addition alone.
    #[test]
    fn rejects_a_scalar_on_the_subtraction() {
        let error = expand("Sub = -1.0f64", "struct X(f64);").unwrap_err();

        assert!(error.contains("takes no scalar"), "{error}");
    }

    /// Rejects an operator the completion has no rule for.
    #[test]
    fn rejects_an_unknown_operator() {
        let error = expand("Div", "struct X(f64);").unwrap_err();

        assert!(error.contains(SUBTRACTION), "{error}");
    }

    /// Rejects an item that declares no type to complete the operators on.
    #[test]
    fn rejects_an_item_that_is_not_a_declaration() {
        let error = expand("Sub", "fn value() {}").unwrap_err();

        assert!(error.contains("type declaration"), "{error}");
    }
}
