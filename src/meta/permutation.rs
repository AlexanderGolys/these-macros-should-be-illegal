//! Finite permutations embedded into streams of arbitrary greater length.

use std::cmp::max;
use std::collections::HashSet;
use std::ops::Mul;

use proc_macro2::{Delimiter, Group, Punct, Spacing, Span, TokenStream, TokenTree};
use quote::ToTokens;
use syn::{Error, LitInt, parse2};

/// Minimal callable-object operation used by the permutation algebra.
trait Call<Rhs> {
    /// Value returned after applying the object to one argument.
    type Codomain;

    /// Applies the object to `x`.
    fn call(self, x: Rhs) -> Self::Codomain;
}

/// A finite zero-based permutation which fixes every omitted trailing position.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Permutation {
    /// Destination indexed by source, with the trailing fixed points removed.
    map_idx: Vec<usize>,
}

impl Permutation {
    /// Constructs the permutation which fixes every position.
    pub(crate) fn identity() -> Self {
        Self {
            map_idx: Vec::new(),
        }
    }

    /// Constructs a permutation map and removes its redundant fixed suffix.
    pub(crate) fn new(mut map_idx: Vec<usize>) -> Self {
        while map_idx
            .last()
            .is_some_and(|destination| *destination + 1 == map_idx.len())
        {
            map_idx.pop();
        }
        Self { map_idx }
    }

    /// Constructs the permutation represented by one zero-based cycle.
    pub(crate) fn from_cycle(cycle: &[usize]) -> Self {
        let Some(maximum) = cycle.iter().copied().max() else {
            return Self::identity();
        };
        let mut map_idx = (0..=maximum).collect::<Vec<_>>();
        for (source, destination) in cycle
            .iter()
            .zip(cycle.iter().cycle().skip(1))
            .take(cycle.len())
        {
            map_idx[*source] = *destination;
        }
        Self::new(map_idx)
    }

    /// Constructs the restriction of a mapping function to `0..bound`.
    pub(crate) fn from_fn(bound: usize, f: impl Fn(usize) -> usize) -> Self {
        Self::new((0usize..bound).map(f).collect())
    }

    /// Returns the smallest symmetric group containing this permutation.
    pub(crate) fn min_size(&self) -> usize {
        self.map_idx.len()
    }
}

impl Call<&usize> for &Permutation {
    type Codomain = usize;
    fn call(self, x: &usize) -> Self::Codomain {
        *self.map_idx.get(*x).unwrap_or(x)
    }
}

impl Call<&usize> for Permutation {
    type Codomain = usize;
    fn call(self, x: &usize) -> Self::Codomain {
        (&self).call(x)
    }
}

impl Mul for &Permutation {
    type Output = Permutation;
    fn mul(self, rhs: Self) -> Self::Output {
        Permutation::from_fn(max(self.min_size(), rhs.min_size()), |i| {
            self.call(&rhs.call(&i))
        })
    }
}

impl Mul<&Permutation> for Permutation {
    type Output = Permutation;
    fn mul(self, rhs: &Self) -> Self {
        &self * rhs
    }
}

/// One parsed cycle and the span of its delimiters.
struct Cycle {
    /// One-based positions in cyclic order.
    positions: Vec<usize>,
    /// Span used for diagnostics concerning the complete cycle.
    span: Span,
}
/// Parsed permutation and comma-separated token-tree operands.
struct Invocation {
    /// Cycles in conventional written composition order.
    cycles: Vec<Cycle>,
    /// Individual token trees on which the permutation acts.
    elements: Vec<TokenTree>,
    /// Whether the input stream ended in a comma.
    trailing_comma: bool,
}

/// Applies a finite permutation to the leading positions of a token stream.
///
/// Cycles compose from right to left and use one-based positions. Positions
/// beyond the largest mentioned index are fixed, realizing the conventional
/// embedding from `S_n` into every `S_{n+k}`.
pub(crate) fn perm(input: TokenStream) -> TokenStream {
    parse_invocation(input)
        .and_then(apply)
        .unwrap_or_else(Error::into_compile_error)
}

/// Parses one parenthesized cycle product followed by comma-separated trees.
fn parse_invocation(input: TokenStream) -> syn::Result<Invocation> {
    let mut tokens = input.into_iter().peekable();
    let Some(TokenTree::Group(permutation)) = tokens.next() else {
        return Err(Error::new(
            Span::call_site(),
            "expected the permutation in parentheses, for example `((1 4 3))`",
        ));
    };
    if permutation.delimiter() != Delimiter::Parenthesis {
        return Err(Error::new(
            permutation.span(),
            "expected the permutation in parentheses",
        ));
    }
    expect_comma(tokens.next(), permutation.span())?;

    let cycles = parse_cycles(permutation)?;
    let mut elements = Vec::new();
    let mut trailing_comma = false;
    while let Some(element) = tokens.next() {
        if is_comma(&element) {
            return Err(Error::new(
                element.span(),
                "expected a token tree after `,`",
            ));
        }
        elements.push(element);

        match tokens.next() {
            Some(separator) => {
                expect_comma(
                    Some(separator),
                    elements.last().expect("just pushed").span(),
                )?;
                trailing_comma = tokens.peek().is_none();
            }
            None => {
                trailing_comma = false;
                break;
            }
        }
    }

    Ok(Invocation {
        cycles,
        elements,
        trailing_comma,
    })
}

/// Parses every inner parenthesized cycle.
fn parse_cycles(permutation: Group) -> syn::Result<Vec<Cycle>> {
    permutation
        .stream()
        .into_iter()
        .map(|token| {
            let TokenTree::Group(cycle) = token else {
                return Err(Error::new(
                    token.span(),
                    "expected a parenthesized cycle such as `(1 4 3)`",
                ));
            };
            if cycle.delimiter() != Delimiter::Parenthesis {
                return Err(Error::new(cycle.span(), "expected a parenthesized cycle"));
            }
            parse_cycle(cycle)
        })
        .collect()
}

/// Parses one nonempty cycle of distinct positive decimal indices.
fn parse_cycle(cycle: Group) -> syn::Result<Cycle> {
    let span = cycle.span();
    let mut positions = Vec::new();
    let mut seen = HashSet::new();

    for token in cycle.stream() {
        let token_span = token.span();
        let TokenTree::Literal(literal) = token else {
            return Err(Error::new(
                token_span,
                "cycle positions must be positive integer literals separated by spaces",
            ));
        };
        let integer = parse2::<LitInt>(TokenStream::from(TokenTree::Literal(literal)))?;
        if !integer.suffix().is_empty() {
            return Err(Error::new_spanned(
                integer,
                "cycle positions cannot have integer suffixes",
            ));
        }
        let position = integer.base10_parse::<usize>()?;
        if position == 0 {
            return Err(Error::new_spanned(
                integer,
                "cycle positions are one-based and cannot be zero",
            ));
        }
        if !seen.insert(position) {
            return Err(Error::new_spanned(
                integer,
                format!("position `{position}` occurs twice in one cycle"),
            ));
        }
        positions.push(position);
    }

    if positions.is_empty() {
        return Err(Error::new(span, "a cycle cannot be empty"));
    }
    Ok(Cycle { positions, span })
}

/// Computes the product and moves every supplied element to its destination.
fn apply(invocation: Invocation) -> syn::Result<TokenStream> {
    let Invocation {
        cycles,
        elements,
        trailing_comma,
    } = invocation;
    let degree = cycles
        .iter()
        .flat_map(|cycle| cycle.positions.iter().copied())
        .max()
        .unwrap_or(0);
    if degree > elements.len() {
        let span = cycles
            .iter()
            .find(|cycle| cycle.positions.contains(&degree))
            .map_or_else(Span::call_site, |cycle| cycle.span);
        return Err(Error::new(
            span,
            format!(
                "permutation references position `{degree}`, but only {} token trees were supplied",
                elements.len()
            ),
        ));
    }

    let permutation = cycles
        .iter()
        .fold(Permutation::identity(), |product, cycle| {
            let positions = cycle
                .positions
                .iter()
                .map(|position| position - 1)
                .collect::<Vec<_>>();
            product * &Permutation::from_cycle(&positions)
        });

    let mut output = vec![None; elements.len()];
    for (source, element) in elements.into_iter().enumerate() {
        output[(&permutation).call(&source)] = Some(element);
    }
    let mut stream = TokenStream::new();
    let output_len = output.len();
    for (index, element) in output.into_iter().enumerate() {
        element
            .expect("a permutation assigns exactly one source to every destination")
            .to_tokens(&mut stream);
        if index + 1 < output_len || trailing_comma {
            stream.extend([TokenTree::Punct(Punct::new(',', Spacing::Alone))]);
        }
    }
    Ok(stream)
}

/// Requires one comma token at a structural separator.
fn expect_comma(token: Option<TokenTree>, previous: Span) -> syn::Result<()> {
    match token {
        Some(token) if is_comma(&token) => Ok(()),
        Some(token) => Err(Error::new(token.span(), "expected `,`")),
        None => Err(Error::new(previous, "expected `,`")),
    }
}

/// Recognizes a comma punctuation token.
fn is_comma(token: &TokenTree) -> bool {
    matches!(token, TokenTree::Punct(punctuation) if punctuation.as_char() == ',')
}

/// Cycle parsing, composition, embedding, and validation tests.
#[cfg(test)]
mod tests {
    use quote::quote;

    use super::{Call, Permutation, perm};

    /// The empty representation fixes positions of every size.
    #[test]
    fn identity_fixes_every_position() {
        let identity = Permutation::identity();

        assert_eq!(identity.min_size(), 0);
        assert_eq!((&identity).call(&0), 0);
        assert_eq!((&identity).call(&17), 17);
    }

    /// Cycle construction records destinations without shifting map entries.
    #[test]
    fn constructs_a_permutation_from_a_cycle() {
        let permutation = Permutation::from_cycle(&[0, 3, 2]);

        assert_eq!(permutation.min_size(), 4);
        assert_eq!((&permutation).call(&0), 3);
        assert_eq!((&permutation).call(&1), 1);
        assert_eq!((&permutation).call(&2), 0);
        assert_eq!((&permutation).call(&3), 2);
        assert_eq!((&permutation).call(&4), 4);
    }

    /// A map is stored only through its last non-fixed position.
    #[test]
    fn trims_trailing_fixed_positions() {
        assert_eq!(Permutation::new(vec![1, 0, 2, 3]).min_size(), 2);
    }

    /// Multiplication applies the right operand before the left operand.
    #[test]
    fn multiplies_permutations_as_function_composition() {
        let left = Permutation::from_cycle(&[1, 0, 3]);
        let right = Permutation::from_cycle(&[2, 3]);
        let product = &left * &right;

        assert_eq!(
            (0..5)
                .map(|position| (&product).call(&position))
                .collect::<Vec<_>>(),
            vec![3, 0, 1, 2, 4]
        );
    }

    /// Unmentioned trailing positions are fixed by the implicit embedding.
    #[test]
    fn embeds_a_permutation_into_a_longer_stream() {
        assert_eq!(
            perm(quote!(((1 4 3)), a, b, c, d, e)).to_string(),
            quote!(c, b, d, a, e).to_string()
        );
    }

    /// Products of cycles use the conventional right-to-left action.
    #[test]
    fn composes_cycles_from_right_to_left() {
        assert_eq!(
            perm(quote!(((2 1 4)(3 4)), a, b, c, d, e)).to_string(),
            quote!(b, c, d, a, e).to_string()
        );
    }

    /// Token-tree operands may be identifiers, literals, punctuation, or groups.
    #[test]
    fn permutes_heterogeneous_token_trees() {
        assert_eq!(
            perm(quote!(((1 3)), alpha, 7, [inside], +)).to_string(),
            quote!([inside], 7, alpha, +).to_string()
        );
    }

    /// A referenced position must exist in the supplied stream.
    #[test]
    fn rejects_a_degree_larger_than_the_stream() {
        let error = perm(quote!(((1 4)), a, b, c)).to_string();
        assert!(error.contains("references position `4`"));
        assert!(error.contains("only 3 token trees"));
    }

    /// Repeating a point inside one cycle is not valid cycle notation.
    #[test]
    fn rejects_repeated_positions_inside_a_cycle() {
        assert!(
            perm(quote!(((1 2 1)), a, b))
                .to_string()
                .contains("occurs twice in one cycle")
        );
    }
}
