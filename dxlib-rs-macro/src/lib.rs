// src/lib.rs
mod dxlib_error;

extern crate proc_macro;

use anyhow::Result;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    Expr, ExprLit, FnArg, GenericArgument, Ident, Lit, LitStr, Meta, MetaNameValue, Pat, PatType,
    PathArguments, ReturnType, Signature, Token, Type, TypeParamBound, TypePath, TypeReference,
    parse::{Parse, ParseStream},
    parse_macro_input, parse_str,
    punctuated::Punctuated,
};

// 属性付き関数
struct FunctionWithAttrs {
    attrs: Vec<syn::Attribute>,
    sig: syn::Signature,
}

impl Parse for FunctionWithAttrs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let attrs = input.call(syn::Attribute::parse_outer)?;
        let sig: Signature = input.parse()?;
        Ok(FunctionWithAttrs { attrs, sig })
    }
}

// マクロ全体
struct DxlibGenInput {
    lib_name: LitStr,
    fns: Punctuated<FunctionWithAttrs, Token![,]>,
}

impl Parse for DxlibGenInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lib_name: LitStr = input.parse()?;
        input.parse::<Token![,]>()?;
        let fns = Punctuated::<FunctionWithAttrs, Token![,]>::parse_terminated(input)?;
        Ok(DxlibGenInput { lib_name, fns })
    }
}

// 型 `a` と `b` が構造的に同じかを判定（再帰）
fn type_eq(a: &Type, b: &Type) -> bool {
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
fn is_impl_trait_with_target_type_path(ty: &Type, trait_name: &str, expected_ty: &Type) -> bool {
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

fn is_impl_trait_with_target_type(ty: &Type, trait_name: &str, type_arg_name: &str) -> bool {
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
fn is_impl_trait_named(ty: &Type, target: &str) -> bool {
    match ty {
        Type::ImplTrait(it) => it
            .bounds
            .iter()
            .any(|bound| matches!(bound, TypeParamBound::Trait(tb) if tb.path.is_ident(target))),
        _ => false,
    }
}

fn is_impl_to_string(ty: &Type) -> bool {
    is_impl_trait_named(ty, "ToString")
}

fn is_impl_display(ty: &Type) -> bool {
    is_impl_trait_named(ty, "Display")
}

fn get_return_type(sig: &Signature) -> Option<&syn::Type> {
    match &sig.output {
        ReturnType::Default => None,
        ReturnType::Type(_, ty) => Some(ty.as_ref()),
    }
}

fn extract_default_expr(attrs: &[syn::Attribute]) -> Option<proc_macro2::TokenStream> {
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
fn is_option(ty: &Type) -> Option<&Type> {
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

fn extract_error_condition(attrs: &[syn::Attribute]) -> Option<proc_macro2::TokenStream> {
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

#[proc_macro]
pub fn dxlib_gen(input: TokenStream) -> TokenStream {
    let DxlibGenInput { lib_name, fns } = parse_macro_input!(input as DxlibGenInput);

    // CString を使うための import
    let mut output = quote! {
        use std::ffi::CString;
    };

    for FunctionWithAttrs { attrs, sig } in fns.iter() {
        let wrapper_name = &sig.ident;
        let extern_name = format_ident!("dx_{}", wrapper_name);
        let output_ty = &sig.output;
        let generics = &sig.generics;

        let mut wrapper_args = Vec::new();
        let mut extern_args = Vec::new();
        let mut convert_stmts = Vec::new();
        let mut call_idents = Vec::new();

        let return_type = get_return_type(&sig).unwrap();

        let error_condition =
            extract_error_condition(attrs).unwrap_or_else(|| quote! { result as i32 == -1i32 });
        for arg in sig.inputs.iter() {
            if let FnArg::Typed(PatType { pat, ty, attrs, .. }) = arg {
                let ident = match &**pat {
                    Pat::Ident(pi) => &pi.ident,
                    _ => panic!("パターン付き引数は未対応です"),
                };

                if let Some(inner_ty) = is_option(&ty) {
                    // Option<T> の場合
                    wrapper_args.push(quote! { #ident: Option<#inner_ty> });

                    let default_expr = extract_default_expr(attrs)
                        .unwrap_or_else(|| quote! { Default::default() });

                    convert_stmts.push(quote! {
                        let #ident = match #ident {
                            Some(value) => value,
                            None => #default_expr,
                        };
                    });

                    extern_args.push(quote! { #ident: #inner_ty });
                    call_idents.push(quote! { #ident });

                    continue;
                }

                if is_impl_to_string(&ty) {
                    wrapper_args.push(quote! { #ident: impl ToString });
                    extern_args.push(quote! { #ident: *const i8 });
                    convert_stmts.push(quote! {
                        let #ident = {
                            let s = #ident.to_string();
                            let c = CString::new(s).expect("CString::new failed");
                            let ptr = c.as_ptr();
                            std::mem::forget(c);
                            ptr
                        };
                    });
                    call_idents.push(quote! { #ident });
                    continue;
                }

                if is_impl_display(&ty) {
                    wrapper_args.push(quote! { #ident: impl Display });
                    extern_args.push(quote! { #ident: *const i8 });
                    convert_stmts.push(quote! {
                        let #ident = {
                            let s = #ident.to_string();
                            let c = CString::new(s).expect("CString::new failed");
                            let ptr = c.as_ptr();
                            std::mem::forget(c);
                            ptr
                        };
                    });
                    call_idents.push(quote! { #ident });
                    continue;
                }
                // impl Into<Vec<u8>>の場合は*mut u8に変換
                if is_impl_trait_with_target_type_path(&ty, "Into", &parse_str("Vec<u8>").unwrap())
                {
                    wrapper_args.push(quote! { #ident: impl Into<Vec<u8>> });
                    extern_args.push(quote! { #ident: *mut u8 });
                    convert_stmts.push(quote! {
                        let #ident = {
                            let v = #ident.into();
                            let ptr = v.as_mut_ptr();
                            std::mem::forget(v); // メモリ管理
                            ptr
                        };
                    });
                    call_idents.push(quote! { #ident });
                    continue;
                }

                // impl Into<Vec<i8>>の場合は*mut i8に変換
                if is_impl_trait_with_target_type_path(&ty, "Into", &parse_str("Vec<i8>").unwrap())
                {
                    wrapper_args.push(quote! { #ident: impl Into<Vec<i8>> });
                    extern_args.push(quote! { #ident: *mut i8 });
                    convert_stmts.push(quote! {
                        let #ident = {
                            let mut v = #ident.into();
                            let ptr = v.as_mut_ptr();
                            std::mem::forget(v); // メモリ管理
                            ptr
                        };
                    });
                    call_idents.push(quote! { #ident });
                    continue;
                }

                // &str の場合は CString に変換
                if let Type::Reference(TypeReference { elem, .. }) = &**ty {
                    if let Type::Path(TypePath { path, .. }) = &**elem {
                        if path.is_ident("str") {
                            wrapper_args.push(quote! { #ident: &str });
                            extern_args.push(quote! { #ident: *const i8 });
                            convert_stmts.push(quote! {
                                let #ident = {
                                    let c = CString::new(#ident).unwrap();
                                    let ptr = c.as_ptr();
                                    std::mem::forget(c);
                                    ptr
                                };
                            });
                            call_idents.push(quote! { #ident });
                            continue;
                        }
                    }
                }

                // それ以外はそのまま
                wrapper_args.push(quote! { #ident: #ty });
                extern_args.push(quote! { #ident: #ty });
                call_idents.push(quote! { #ident });
            }
        }

        let extern_block = quote! {
            #[link(name = #lib_name)]
            unsafe extern "stdcall" {
                fn #extern_name(#(#extern_args),*) #output_ty;
            }
        };

        output.extend(extern_block);
        // DxLib_Init と DxLib_End 用の処理
        if wrapper_name == "DxLib_Init" {
            let wrapper_fn = quote! {
              pub fn #wrapper_name #generics( #(#wrapper_args),* ) -> anyhow::Result<#return_type, DxLibError> {
                #(#convert_stmts)*

                unsafe {
                    let result: #return_type = #extern_name(#(#call_idents),*);
                                    if #error_condition {
                        return Err(DxLibError::Other(anyhow::anyhow!("Error in {}", stringify!(#wrapper_name))));
                    } else {
                        return Ok(result);
                    }
                }
                }
            };
            output.extend(wrapper_fn);
            continue; // 次の関数の処理に進む
        }
        if wrapper_name == "DxLib_End" {
            let wrapper_fn = quote! {
              pub fn #wrapper_name #generics( #(#wrapper_args),* ) -> anyhow::Result<#return_type, DxLibError> {
                #(#convert_stmts)*

                unsafe {
                    let result: #return_type = #extern_name(#(#call_idents),*);

                 if #error_condition {
                        return Err(DxLibError::Other(anyhow::anyhow!("Error in {}", stringify!(#wrapper_name))));
                    } else {
                        return Ok(result);
                    }

                 }
                }
            };
            output.extend(wrapper_fn);
            continue; // 次の関数の処理に進む
        }

        // wrapper 関数の生成
        let wrapper_fn = quote! {
            pub fn #wrapper_name #generics( #(#wrapper_args),* ) -> anyhow::Result<#return_type, DxLibError> {
                #(#convert_stmts)*

                unsafe {
                    let result: #return_type = #extern_name(#(#call_idents),*);
                    if #error_condition {
                        return Err(DxLibError::Other(anyhow::anyhow!("Error in {}", stringify!(#wrapper_name))));
                    } else {
                        return Ok(result);
                    }
                }
            }
        };

        output.extend(wrapper_fn);
    }

    TokenStream::from(output)
}

/*
#[proc_macro]
pub fn dxlib_gen(input: TokenStream) -> TokenStream {
    let DxlibGenInput { lib_name, fns } = parse_macro_input!(input as DxlibGenInput);

    // CString を使うための import
    let mut output = quote! {
        use std::ffi::CString;
    };

    for sig in fns.iter() {
        let wrapper_name = &sig.ident;
        let extern_name = format_ident!("dx_{}", wrapper_name);
        let output_ty = &sig.output;
        let generics = &sig.generics;

        let mut wrapper_args = Vec::new();
        let mut extern_args = Vec::new();
        let mut convert_stmts = Vec::new();
        let mut call_idents = Vec::new();

        let return_type = get_return_type(&sig).unwrap();

        for arg in sig.inputs.iter() {
            if let FnArg::Typed(PatType { pat, ty, attrs, .. }) = arg {
                let ident = match &**pat {
                    Pat::Ident(pi) => &pi.ident,
                    _ => panic!("パターン付き引数は未対応です"),
                };


                // エラー条件を引数の属性から取得
                let error_condition = extract_error_condition(attrs)
                    .unwrap_or_else(|| quote! { result == -1i32 });

                if let Some(inner_ty) = is_option(&ty) {
                    // Option<T> の場合
                    wrapper_args.push(quote! { #ident: Option<#inner_ty> });

                    let default_expr = extract_default_expr(attrs)
                        .unwrap_or_else(|| quote! { Default::default() });

                    convert_stmts.push(quote! {
                        let #ident = match #ident {
                            Some(value) => value,
                            None => #default_expr,
                        };
                    });

                    extern_args.push(quote! { #ident: #inner_ty });
                    call_idents.push(quote! { #ident });

                    // エラーチェック部分
                    convert_stmts.push(quote! {
                        if #error_condition {
                            return Err(DxLibError::Other(anyhow::anyhow!("Error in {}", stringify!(#wrapper_name))));
                        }
                    });
                    continue;
                }
                // &str の場合は CString に変換
                if let Type::Reference(TypeReference { elem, .. }) = &**ty {
                    if let Type::Path(TypePath { path, .. }) = &**elem {
                        if path.is_ident("str") {
                            wrapper_args.push(quote! { #ident: &str });
                            extern_args.push(quote! { #ident: *const i8 });
                            convert_stmts.push(quote! {
                                let #ident = {
                                    let c = CString::new(#ident).unwrap();
                                    let ptr = c.as_ptr();
                                    std::mem::forget(c);
                                    ptr
                                };
                            });
                            call_idents.push(quote! { #ident });
                            continue;
                        }
                    }
                }

                // それ以外はそのまま
                wrapper_args.push(quote! { #ident: #ty });
                extern_args.push(quote! { #ident: #ty });
                call_idents.push(quote! { #ident });
            }
        }

        let extern_block = quote! {
            #[link(name = #lib_name)]
            unsafe extern "stdcall" {
                fn #extern_name(#(#extern_args),*) #output_ty;
            }
        };

        output.extend(extern_block);
        // DxLib_Init と DxLib_End 用の処理
        if wrapper_name == "DxLib_Init" {
            let wrapper_fn = quote! {
              pub fn #wrapper_name #generics( #(#wrapper_args),* ) -> anyhow::Result<#return_type, DxLibError> {
                #(#convert_stmts)*

                unsafe {
                    let result: #return_type = #extern_name(#(#call_idents),*);
                                    if #error_condition {
                        return Err(DxLibError::Other(anyhow::anyhow!("Error in {}", stringify!(#wrapper_name))));
                    } else {
                        return Ok(result);
                    }
                }
                }
            };
            output.extend(wrapper_fn);
            continue; // 次の関数の処理に進む
        }
        if wrapper_name == "DxLib_End" {
            let wrapper_fn = quote! {
              pub fn #wrapper_name #generics( #(#wrapper_args),* ) -> anyhow::Result<#return_type, DxLibError> {
                #(#convert_stmts)*

                unsafe {
                    let result: #return_type = #extern_name(#(#call_idents),*);

                 if #error_condition {
                        return Err(DxLibError::Other(anyhow::anyhow!("Error in {}", stringify!(#wrapper_name))));
                    } else {
                        return Ok(result);
                    }

                 }
                }
            };
            output.extend(wrapper_fn);
            continue; // 次の関数の処理に進む
        }

        let wrapper_fn = quote! {
            pub fn #wrapper_name #generics( #(#wrapper_args),* ) -> anyhow::Result<#return_type, DxLibError> {
                #(#convert_stmts)*

                unsafe {
                    let result: #return_type = #extern_name(#(#call_idents),*);
                    if result == -1i32 {
                        return Err(DxLibError::Other(anyhow::anyhow!("Unknown error failed to {}", stringify!(#extern_name))));
                    } else {
                        return Ok(result);
                    }

                    if #error_condition {
                        return Err(DxLibError::Other(anyhow::anyhow!("Error in {}", stringify!(#wrapper_name))));
                    } else {
                        return Ok(result);
                    }

                 }

            }
        };

        output.extend(wrapper_fn);
    }

    TokenStream::from(output)
}
*/
