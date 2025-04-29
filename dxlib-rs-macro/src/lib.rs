// src/lib.rs
mod dxlib_error;

extern crate proc_macro;

use anyhow::Result;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    Expr, ExprLit, FnArg, GenericArgument, Ident, Lit, LitStr, Meta, MetaNameValue, Pat, PatType,
    PathArguments, ReturnType, Signature, Token, Type, TypePath, TypeReference,
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
};

/*
/// マクロ入力全体を受け取る構造体
struct DxlibGenInput {
    lib_name: LitStr,
    fns: Punctuated<Signature, Token![,]>,
}

impl Parse for DxlibGenInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // 1) 文字列リテラルとしてライブラリ名をパース
        let lib_name: LitStr = input.parse()?;
        // 2) カンマをスキップ
        input.parse::<Token![,]>()?;
        // 3) 以降を Signature のカンマ区切りリストとしてパース
        let fns = Punctuated::<Signature, Token![,]>::parse_terminated(input)?;
        Ok(DxlibGenInput { lib_name, fns })
    }
}
*/

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
