use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    Expr, ExprLit, FnArg, GenericArgument, Ident, Lit, LitStr, Meta, MetaNameValue, Pat, PatType,
    PathArguments, ReturnType, Signature, Token, Type, TypeArray, TypeImplTrait, TypeParamBound,
    TypePath, TypeReference, TypeSlice,
    parse::{Parse, ParseStream},
    parse_macro_input, parse_str,
    punctuated::Punctuated,
};
// 型 `a` と `b` が構造的に同じかを判定（再帰）
pub fn type_eq(a: &Type, b: &Type) -> bool {
    match (a, b) {
        (Type::Path(a_path), Type::Path(b_path)) => {
            let a_segments = &a_path.path.segments;
            let b_segments = &b_path.path.segments;

            if a_segments.len() != b_segments.len() {
                return false;
            }

            for (a_seg, b_seg) in a_segments.iter().zip(b_segments.iter()) {
                if a_seg.ident != b_seg.ident {
                    return false;
                }

                match (&a_seg.arguments, &b_seg.arguments) {
                    (
                        PathArguments::AngleBracketed(a_args),
                        PathArguments::AngleBracketed(b_args),
                    ) => {
                        let a_generic = &a_args.args;
                        let b_generic = &b_args.args;

                        if a_generic.len() != b_generic.len() {
                            return false;
                        }

                        for (a_arg, b_arg) in a_generic.iter().zip(b_generic.iter()) {
                            match (a_arg, b_arg) {
                                (GenericArgument::Type(a_ty), GenericArgument::Type(b_ty)) => {
                                    if !type_eq(a_ty, b_ty) {
                                        return false;
                                    }
                                }
                                _ => return false, // lifetimesや他の引数には未対応
                            }
                        }
                    }
                    (PathArguments::None, PathArguments::None) => {}
                    _ => return false,
                }
            }

            true
        }
        _ => false,
    }
}

// `impl Trait<SomeType>` において、Trait名と型引数の型構造が一致するかを判定
pub fn is_impl_trait_with_target_type_path(
    ty: &Type,
    trait_name: &str,
    expected_ty: &Type,
) -> bool {
    match ty {
        Type::ImplTrait(it) => it.bounds.iter().any(|bound| {
            if let TypeParamBound::Trait(trait_bound) = bound {
                let path = &trait_bound.path;

                if let Some(last_segment) = path.segments.last() {
                    if last_segment.ident == trait_name {
                        if let PathArguments::AngleBracketed(args) = &last_segment.arguments {
                            return args.args.iter().any(|arg| {
                                if let GenericArgument::Type(inner_ty) = arg {
                                    return type_eq(inner_ty, expected_ty);
                                }
                                false
                            });
                        }
                    }
                }
            }
            false
        }),
        _ => false,
    }
}

pub fn is_impl_trait_with_target_type(ty: &Type, trait_name: &str, type_arg_name: &str) -> bool {
    match ty {
        Type::ImplTrait(it) => it.bounds.iter().any(|bound| {
            if let TypeParamBound::Trait(trait_bound) = bound {
                let path = &trait_bound.path;

                if let Some(last_segment) = path.segments.last() {
                    if last_segment.ident == trait_name {
                        if let PathArguments::AngleBracketed(args) = &last_segment.arguments {
                            return args.args.iter().any(|arg| {
                                if let GenericArgument::Type(Type::Path(type_path)) = arg {
                                    if let Some(ident) = type_path.path.get_ident() {
                                        return ident == type_arg_name;
                                    }
                                }
                                false
                            });
                        }
                    }
                }
            }
            false
        }),
        _ => false,
    }
}
pub fn is_impl(ty: &Type) -> bool {
    match ty {
        Type::ImplTrait(_) => true,
        _ => false,
    }
}
pub fn is_impl_trait_named(ty: &Type, target: &str) -> bool {
    match ty {
        Type::ImplTrait(it) => it
            .bounds
            .iter()
            .any(|bound| matches!(bound, TypeParamBound::Trait(tb) if tb.path.is_ident(target))),
        _ => false,
    }
}

pub fn is_impl_to_string(ty: &Type) -> bool {
    is_impl_trait_named(ty, "ToString")
}

pub fn is_impl_display(ty: &Type) -> bool {
    is_impl_trait_named(ty, "Display")
}

pub fn get_return_type(sig: &Signature) -> Option<&syn::Type> {
    match &sig.output {
        ReturnType::Default => None,
        ReturnType::Type(_, ty) => Some(ty.as_ref()),
    }
}

pub fn extract_default_expr(attrs: &[syn::Attribute]) -> Option<proc_macro2::TokenStream> {
    for attr in attrs {
        if attr.path().is_ident("default") {
            if let Meta::NameValue(MetaNameValue { value, .. }) = &attr.meta {
                if let Expr::Lit(ExprLit {
                    lit: Lit::Str(lit_str),
                    ..
                }) = value
                {
                    let value = lit_str.value();
                    return Some(match value.as_str() {
                        "null" => quote! { std::ptr::null() },
                        "null_mut" => quote! { std::ptr::null_mut() },
                        "default" => quote! { Default::default() },
                        other => {
                            let tokens: proc_macro2::TokenStream =
                                other.parse().expect("Invalid default literal");
                            quote! { #tokens }
                        }
                    });
                }
            }
        }
    }
    None
}
// Option<T>かどうかを判定
pub fn is_option(ty: &Type) -> Option<&Type> {
    if let Type::Path(TypePath { path, .. }) = ty {
        if path.segments.len() == 1 && path.segments[0].ident == "Option" {
            if let PathArguments::AngleBracketed(args) = &path.segments[0].arguments {
                if let Some(GenericArgument::Type(inner_ty)) = args.args.first() {
                    return Some(inner_ty);
                }
            }
        }
    }
    None
}

// AsRef<T>かどうかを判定
pub fn is_impl_as_ref_type(ty: &Type) -> bool {
    // 参照型(&T, &mut T)の場合
    if let Type::Reference(ref_type) = ty {
        // &mutの場合も対象にする
        return is_impl_as_ref_type(&ref_type.elem);
    }

    // 通常のimpl AsRef<T>の場合
    if let Type::ImplTrait(TypeImplTrait { bounds, .. }) = ty {
        for bound in bounds {
            if let syn::TypeParamBound::Trait(trait_bound) = bound {
                let segments = &trait_bound.path.segments;
                if let Some(segment) = segments.last() {
                    if segment.ident == "AsRef" {
                        return true;
                    }
                }
            }
        }
    }
    false
}

// AsRef<T>を取り出して返す
pub fn extract_as_ref_generic(ty: &Type) -> Option<&Type> {
    // 参照型(&T, &mut T)の場合
    if let Type::Reference(ref_type) = ty {
        return extract_as_ref_generic(&ref_type.elem);
    }

    // 通常のimpl AsRef<T>の場合
    if let Type::ImplTrait(TypeImplTrait { bounds, .. }) = ty {
        for bound in bounds {
            if let TypeParamBound::Trait(trait_bound) = bound {
                let segments = &trait_bound.path.segments;
                if let Some(segment) = segments.last() {
                    if segment.ident == "AsRef" {
                        if let PathArguments::AngleBracketed(args) = &segment.arguments {
                            for arg in &args.args {
                                if let GenericArgument::Type(inner_ty) = arg {
                                    return Some(inner_ty);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

// AsMut<T>かどうかを判定
pub fn is_impl_as_mut_type(ty: &Type) -> bool {
    // 参照型(&T, &mut T)の場合
    if let Type::Reference(ref_type) = ty {
        // &mutの場合も対象にする
        return is_impl_as_mut_type(&ref_type.elem);
    }

    // 通常のimpl AsMut<T>の場合
    if let Type::ImplTrait(TypeImplTrait { bounds, .. }) = ty {
        for bound in bounds {
            if let syn::TypeParamBound::Trait(trait_bound) = bound {
                if let Some(segment) = trait_bound.path.segments.last() {
                    if segment.ident == "AsMut" {
                        return true;
                    }
                }
            }
        }
    }
    false
}

pub fn extract_as_mut_generic(ty: &Type) -> Option<&Type> {
    // 参照型(&T, &mut T)の場合
    if let Type::Reference(ref_type) = ty {
        return extract_as_mut_generic(&ref_type.elem);
    }

    // 通常のimpl AsMut<T>の場合
    if let Type::ImplTrait(TypeImplTrait { bounds, .. }) = ty {
        for bound in bounds {
            if let TypeParamBound::Trait(trait_bound) = bound {
                let segments = &trait_bound.path.segments;
                if let Some(segment) = segments.last() {
                    if segment.ident == "AsMut" {
                        if let PathArguments::AngleBracketed(args) = &segment.arguments {
                            for arg in &args.args {
                                if let GenericArgument::Type(inner_ty) = arg {
                                    return Some(inner_ty);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

// 配列[num;T]かどうかを判定
pub fn is_array(ty: &Type) -> bool {
    matches!(*ty, Type::Array(_))
}
pub fn extract_array(ty: &Type) -> Option<(&Type, &Expr)> {
    if let Type::Array(TypeArray { elem, len, .. }) = ty {
        Some((elem.as_ref(), len))
    } else {
        None
    }
}
// 可変配列（&mut [T;N]）かどうかを判定
pub fn is_mut_array(ty: &Type) -> bool {
    if let Type::Reference(TypeReference {
        elem,
        mutability: Some(_),
        ..
    }) = ty
    {
        matches!(**elem, Type::Array(_))
    } else {
        false
    }
}

// `&mut [T; N]` を受け取り、要素型 `T` と長さ `N` を返す
pub fn extract_mut_array(ty: &Type) -> Option<(&Type, &Expr)> {
    if let Type::Reference(TypeReference {
        elem,
        mutability: Some(_),
        ..
    }) = ty
    {
        if let Type::Array(TypeArray {
            elem: array_elem,
            len,
            ..
        }) = &**elem
        {
            Some((array_elem.as_ref(), len))
        } else {
            None
        }
    } else {
        None
    }
}

// 不変スライス（&[T]）かどうかを判定
pub fn is_slice(ty: &Type) -> bool {
    if let Type::Reference(TypeReference {
        elem,
        mutability: None,
        ..
    }) = ty
    {
        matches!(**elem, Type::Slice(_))
    } else {
        false
    }
}

// 不変スライスの要素型を抽出（&[T] → T）
pub fn extract_slice(ty: &Type) -> Option<&Type> {
    if let Type::Reference(TypeReference {
        elem,
        mutability: None,
        ..
    }) = ty
    {
        if let Type::Slice(slice) = &**elem {
            Some(&slice.elem)
        } else {
            None
        }
    } else {
        None
    }
}

// 可変スライス（&mut [T]）かどうかを判定
pub fn is_mut_slice(ty: &Type) -> bool {
    if let Type::Reference(TypeReference {
        elem,
        mutability: Some(_),
        ..
    }) = ty
    {
        matches!(**elem, Type::Slice(_))
    } else {
        false
    }
}

// 可変スライスの要素型を抽出（&mut [T] → T）
pub fn extract_mut_slice(ty: &Type) -> Option<&Type> {
    if let Type::Reference(TypeReference {
        elem,
        mutability: Some(_),
        ..
    }) = ty
    {
        if let Type::Slice(slice) = &**elem {
            Some(&slice.elem)
        } else {
            None
        }
    } else {
        None
    }
}

pub fn extract_error_condition(attrs: &[syn::Attribute]) -> Option<proc_macro2::TokenStream> {
    for attr in attrs {
        if attr.path().is_ident("error_condition") {
            if let Meta::NameValue(MetaNameValue { value, .. }) = &attr.meta {
                if let Expr::Lit(ExprLit {
                    lit: Lit::Str(lit_str),
                    ..
                }) = value
                {
                    let value = lit_str.value();
                    return Some(value.parse().expect("Invalid error condition expression"));
                }
            }
        }
    }
    None
}

pub fn extract_alias_attribute(attrs: &[syn::Attribute]) -> Option<String> {
    for attr in attrs {
        if attr.path().is_ident("alias") {
            if let Meta::NameValue(MetaNameValue { value, .. }) = &attr.meta {
                if let Expr::Lit(ExprLit {
                    lit: Lit::Str(lit_str),
                    ..
                }) = value
                {
                    return Some(lit_str.value());
                }
            }
        }
    }
    None
}
pub fn is_not_result_attribute(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| attr.path().is_ident("not_result"))
}

pub fn is_impl_trait_into_vec(ty: &Type) -> bool {
    match ty {
        Type::ImplTrait(it) => it.bounds.iter().any(|bound| {
            if let TypeParamBound::Trait(trait_bound) = bound {
                let path = &trait_bound.path;

                if let Some(last_segment) = path.segments.last() {
                    if last_segment.ident == "Into" {
                        if let PathArguments::AngleBracketed(args) = &last_segment.arguments {
                            return args.args.iter().any(|arg| {
                                if let GenericArgument::Type(inner_ty) = arg {
                                    return is_vec_type(inner_ty);
                                }
                                false
                            });
                        }
                    }
                }
            }
            false
        }),
        _ => false,
    }
}

pub fn is_vec_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(last_segment) = type_path.path.segments.last() {
            return last_segment.ident == "Vec";
        }
    }
    false
}

pub fn is_mut_vec_type(ty: &Type) -> bool {
    if let Type::Reference(TypeReference {
        mutability: Some(_),
        elem,
        ..
    }) = ty
    {
        if let Type::Path(TypePath { path, .. }) = elem.as_ref() {
            if let Some(last_segment) = path.segments.last() {
                return last_segment.ident == "Vec";
            }
        }
    }
    false
}
pub fn extract_vec_inner_type(ty: &Type) -> Option<&Type> {
    if let Type::Path(type_path) = ty {
        if let Some(last_segment) = type_path.path.segments.last() {
            if last_segment.ident == "Vec" {
                if let PathArguments::AngleBracketed(args) = &last_segment.arguments {
                    for arg in &args.args {
                        if let GenericArgument::Type(inner_ty) = arg {
                            return Some(inner_ty);
                        }
                    }
                }
            }
        }
    }
    // &mut Vec<T> などの参照タイプも対応
    if let Type::Reference(TypeReference { elem, .. }) = ty {
        return extract_vec_inner_type(elem);
    }
    None
}

pub fn extract_vec_inner_type_from_impl_trait(ty: &Type) -> Option<&Type> {
    if let Type::ImplTrait(it) = ty {
        for bound in &it.bounds {
            if let TypeParamBound::Trait(trait_bound) = bound {
                if trait_bound.path.segments.last()?.ident == "Into" {
                    if let PathArguments::AngleBracketed(args) =
                        &trait_bound.path.segments.last()?.arguments
                    {
                        for arg in &args.args {
                            if let GenericArgument::Type(ty) = arg {
                                return extract_vec_inner_type(ty);
                            }
                            // &mut Vec<T> などの参照タイプも対応
                            if let Type::Reference(TypeReference { elem, .. }) = ty {
                                return extract_vec_inner_type(elem);
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

// 不変 Vec<T> の判定（&Vec<T>）
pub fn is_ref_vec_type(ty: &Type) -> Option<()> {
    if let Type::Reference(TypeReference {
        elem, mutability, ..
    }) = ty
    {
        if mutability.is_none() {
            if let Type::Path(TypePath { path, .. }) = elem.as_ref() {
                if let Some(last_segment) = path.segments.last() {
                    if last_segment.ident == "Vec" {
                        return Some(()); // 不変 Vec<T>
                    }
                }
            }
        }
    }
    None
}

// 可変 Vec<T> の判定（&mut Vec<T>）
pub fn is_mut_ref_vec_type(ty: &Type) -> Option<()> {
    if let Type::Reference(TypeReference {
        elem,
        mutability: Some(_),
        ..
    }) = ty
    {
        if let Type::Path(TypePath { path, .. }) = elem.as_ref() {
            if let Some(last_segment) = path.segments.last() {
                if last_segment.ident == "Vec" {
                    return Some(()); // 可変 Vec<T>
                }
            }
        }
    }
    None
}

pub fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut c = part.chars();
            match c.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + c.as_str(),
            }
        })
        .collect()
}

pub fn to_capitalized_snake(s: &str) -> String {
    s.split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut c = part.chars();
            match c.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + c.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join("_")
}
