// src/lib.rs
extern crate proc_macro;
mod dxlib_error;
mod utils;

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
use utils::*;

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

#[proc_macro]
pub fn dxlib_gen(input: TokenStream) -> TokenStream {
    let DxlibGenInput { lib_name, fns } = parse_macro_input!(input as DxlibGenInput);

    // CString を使うための import
    let mut output = quote! {
        use std::ffi::CString;
    };

    for FunctionWithAttrs { attrs, sig } in fns.iter() {
        let wrapper_name = extract_alias_attribute(attrs)
            .map(|alias| format_ident!("{}", alias))
            .unwrap_or_else(|| sig.ident.clone());
        let extern_name = format_ident!("dx_{}", sig.ident.clone());

        let output_ty = &sig.output;
        let generics = &sig.generics;

        let mut wrapper_args = Vec::new();
        let mut extern_args = Vec::new();
        let mut convert_stmts = Vec::new();
        let mut call_idents = Vec::new();

        let return_type = get_return_type(&sig).unwrap();

        let error_condition =
            extract_error_condition(attrs).unwrap_or_else(|| quote! { result as i32 == -1i32 });
        let is_not_result = is_not_result_attribute(&attrs);

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

                    let holder_ident = format_ident!("__{}_holder", ident);
                    convert_stmts.push(quote! {
                        let #holder_ident = CStringHolder::new(#ident.to_string());
                        let #ident = #holder_ident.ptr();
                    });

                    call_idents.push(quote! { #ident });
                    continue;
                }

                if is_impl_display(&ty) {
                    wrapper_args.push(quote! { #ident: impl Display });
                    extern_args.push(quote! { #ident: *const i8 });

                    let holder_ident = format_ident!("__{}_holder", ident);
                    convert_stmts.push(quote! {
                        let #holder_ident = CStringHolder::new(#ident.to_string());
                        let #ident = #holder_ident.ptr();
                    });

                    call_idents.push(quote! { #ident });
                    continue;
                }

                // impl Into<Vec<任意の型>> に対応し、Vec<i8>,Vec<u8> の場合だけ別処理
                if is_impl_trait_into_vec(&ty) {
                    // Vec<T> の T を取得
                    let inner_ty = extract_vec_inner_type_from_impl_trait(&ty);

                    if let Some(inner_ty) = inner_ty {
                        if type_eq(inner_ty, &parse_str("i8").unwrap()) {
                            // 特別扱い: Vec<i8> → *mut i8
                            wrapper_args.push(quote! { #ident: impl Into<Vec<i8>> });
                            extern_args.push(quote! { #ident: *mut i8 });

                            let holder_ident = format_ident!("__{}_holder", ident);
                            convert_stmts.push(quote! {
                                let #holder_ident = VecHolder::new(#ident);
                                let #ident = #holder_ident.ptr();
                            });

                            call_idents.push(quote! { #ident });
                            continue;
                        }
                        if type_eq(inner_ty, &parse_str("u8").unwrap()) {
                            // 特別扱い: Vec<i8> → *mut i8
                            wrapper_args.push(quote! { #ident: impl Into<Vec<u8>> });
                            extern_args.push(quote! { #ident: *mut u8 });

                            let holder_ident = format_ident!("__{}_holder", ident);
                            convert_stmts.push(quote! {
                                let #holder_ident = VecHolder::new(#ident);
                                let #ident = #holder_ident.ptr();
                            });

                            call_idents.push(quote! { #ident });
                            continue;
                        } else {
                            // 汎用対応: Vec<T> → *mut T
                            wrapper_args.push(quote! { #ident: impl Into<Vec<#inner_ty>> });
                            extern_args.push(quote! { #ident: *mut #inner_ty });

                            let holder_ident = format_ident!("__{}_holder", ident);
                            convert_stmts.push(quote! {
                                let #holder_ident = VecHolder::new(#ident);
                                let #ident = #holder_ident.ptr();
                            });

                            call_idents.push(quote! { #ident });
                            continue;
                        }
                    }
                }

                // Vec の場合は *mut T に変換
                if is_vec_type(&ty) {
                    let inner_ty = extract_vec_inner_type(&ty);
                    wrapper_args.push(quote! { #ident: Vec<#inner_ty> });
                    extern_args.push(quote! { #ident: *mut #inner_ty });
                   
                    let holder_ident = format_ident!("__{}_holder", ident);
                    convert_stmts.push(quote! {
                        let #holder_ident = VecHolder::new(#ident);
                        let #ident = #holder_ident.ptr();
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

                            let holder_ident = format_ident!("__{}_holder", ident);
                            convert_stmts.push(quote! {
                                let #holder_ident = CStringHolder::new(#ident.to_string());
                                let #ident = #holder_ident.ptr();
                            });

                            call_idents.push(quote! { #ident });
                            continue;
                        }
                    }
                }

                // String の場合は CString に変換
                if let Type::Path(TypePath { path, .. }) = &**ty {
                    if path.is_ident("String") {
                        wrapper_args.push(quote! { #ident: String });
                        extern_args.push(quote! { #ident: *const i8 });

                        let holder_ident = format_ident!("__{}_holder", ident);
                        convert_stmts.push(quote! {
                            let #holder_ident = CStringHolder::new(#ident.to_string());
                            let #ident = #holder_ident.ptr();
                        });

                        call_idents.push(quote! { #ident });
                        continue;
                    }
                }

                // &String の場合は CString に変換
                if let Type::Reference(TypeReference { elem, .. }) = &**ty {
                    if let Type::Path(TypePath { path, .. }) = &**elem {
                        if path.is_ident("String") {
                            wrapper_args.push(quote! { #ident: &String });
                            extern_args.push(quote! { #ident: *const i8 });
                            let holder_ident = format_ident!("__{}_holder", ident);
                            convert_stmts.push(quote! {
                                let #holder_ident = CStringHolder::new(#ident.to_string());
                                let #ident = #holder_ident.ptr();
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
            if !is_not_result {
                let wrapper_fn = quote! {
                  pub fn #wrapper_name #generics( #(#wrapper_args),* ) -> anyhow::Result<#return_type, DxLibError> {
                    #(#convert_stmts)*
                    unsafe {
                        let result: #return_type = #extern_name(#(#call_idents),*);
                        if #error_condition {
                            return Err(DxLibError::InitializeError);
                        } else {
                            return Ok(result);
                        }
                    }
                    }
                };
                output.extend(wrapper_fn);
            } else {
                let wrapper_fn = quote! {
                  pub fn #wrapper_name #generics( #(#wrapper_args),* ) -> #return_type {
                    #(#convert_stmts)*
                    unsafe {
                        let result: #return_type = #extern_name(#(#call_idents),*);
                        if #error_condition {
                            return -1;
                        } else {
                            return result;
                        }
                    }
                    }
                };
                output.extend(wrapper_fn);
            }
            continue; // 次の関数の処理に進む
        }
        if wrapper_name == "DxLib_End" {
            if !is_not_result {
                let wrapper_fn = quote! {
                  pub fn #wrapper_name #generics( #(#wrapper_args),* ) -> anyhow::Result<#return_type, DxLibError> {
                    #(#convert_stmts)*

                    unsafe {
                        let result: #return_type = #extern_name(#(#call_idents),*);

                        if #error_condition {
                            return Err(DxLibError::FinalizeError);
                        } else {
                            return Ok(result);
                        }

                     }
                    }
                };
                output.extend(wrapper_fn);
            } else {
                let wrapper_fn = quote! {
                  pub fn #wrapper_name #generics( #(#wrapper_args),* ) -> #return_type {
                    #(#convert_stmts)*
                    unsafe {
                        let result: #return_type = #extern_name(#(#call_idents),*);
                        if #error_condition {
                            return -1;
                        } else {
                            return result;
                        }
                    }
                    }
                };
                output.extend(wrapper_fn);
            }

            continue; // 次の関数の処理に進む
        }

        if !is_not_result {
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
        } else {
            // wrapper 関数の生成
            let wrapper_fn = quote! {
                pub fn #wrapper_name #generics( #(#wrapper_args),* ) -> #return_type {
                    #(#convert_stmts)*

                    unsafe {
                        let result: #return_type = #extern_name(#(#call_idents),*);
                        if #error_condition {
                            return -1;
                        } else {
                            return result;
                        }
                    }
                }
            };

            output.extend(wrapper_fn);
        }
    }

    TokenStream::from(output)
}
