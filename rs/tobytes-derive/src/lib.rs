use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

#[proc_macro_derive(ToBytesDict)]
pub fn derive_to_bytes_dict(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let expanded = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => {
                let field_count = fields.named.len();
                let field_encodings = fields.named.iter().map(|f| {
                    let field_name = &f.ident;
                    let field_name_str = field_name.as_ref().unwrap().to_string();
                    quote! {
                        #field_name_str.to_bytes(wr)?;
                        self.#field_name.to_bytes(wr)?;
                    }
                });

                quote! {
                    impl ToBytes for #name {
                        fn to_bytes<W: std::io::Write>(&self, wr: &mut W) -> ToBytesResult<()> {
                            rmp::encode::write_map_len(wr, #field_count as u32)?;
                            #(#field_encodings)*
                            Ok(())
                        }
                    }
                }
            }
            Fields::Unnamed(fields) => {
                let field_count = fields.unnamed.len();
                let field_encodings = (0..field_count).map(|i| {
                    let index = syn::Index::from(i);
                    quote! {
                        self.#index.to_bytes(wr)?;
                    }
                });

                quote! {
                    impl ToBytes for #name {
                        fn to_bytes<W: std::io::Write>(&self, wr: &mut W) -> ToBytesResult<()> {
                            rmp::encode::write_array_len(wr, #field_count as u32)?;
                            #(#field_encodings)*
                            Ok(())
                        }
                    }
                }
            }
            Fields::Unit => {
                quote! {
                    impl ToBytes for #name {
                        fn to_bytes<W: std::io::Write>(&self, wr: &mut W) -> ToBytesResult<()> {
                            rmp::encode::write_array_len(wr, 0)?;
                            Ok(())
                        }
                    }
                }
            }
        },
        Data::Enum(_) => {
            return syn::Error::new_spanned(
                &input,
                "ToBytes derive macro does not support enums yet",
            )
            .to_compile_error()
            .into();
        }
        Data::Union(_) => {
            return syn::Error::new_spanned(
                &input,
                "ToBytes derive macro does not support unions",
            )
            .to_compile_error()
            .into();
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_derive(FromBytesDict)]
pub fn derive_from_bytes_dict(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let expanded = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => {
                let field_decodings = fields.named.iter().map(|f| {
                    let field_name = &f.ident;
                    let field_name_str = field_name.as_ref().unwrap().to_string();
                    let field_type = &f.ty;
                    quote! {
                        let #field_name = {
                            let val = map.remove(#field_name_str)
                                .ok_or_else(|| {
                                    use std::io;
                                    io::Error::new(
                                        io::ErrorKind::InvalidData,
                                        format!("Missing field: {}", #field_name_str)
                                    )
                                })?;
                            <#field_type>::from_value(val)?
                        };
                    }
                });

                let field_names = fields.named.iter().map(|f| &f.ident);

                quote! {
                    impl FromBytes for #name {
                        type Output = Self;

                        fn from_value(value: rmpv::Value) -> ToBytesResult<Self::Output> {
                            let pairs = Vec::<(rmpv::Value, rmpv::Value)>::try_from(value)?;
                            let mut map = std::collections::HashMap::new();

                            for (key, val) in pairs {
                                let key_str = String::try_from(key)?;
                                map.insert(key_str, val);
                            }

                            #(#field_decodings)*

                            Ok(Self {
                                #(#field_names),*
                            })
                        }
                    }
                }
            }
            Fields::Unnamed(fields) => {
                let field_count = fields.unnamed.len();
                let field_decodings = fields.unnamed.iter().enumerate().map(|(i, f)| {
                    let field_type = &f.ty;
                    let var_name = quote::format_ident!("field_{}", i);
                    quote! {
                        let #var_name = {
                            let val = items.get(#i)
                                .ok_or_else(|| {
                                    use std::io;
                                    io::Error::new(
                                        io::ErrorKind::InvalidData,
                                        format!("Missing field at index {}", #i)
                                    )
                                })?
                                .clone();
                            <#field_type>::from_value(val)?
                        };
                    }
                });

                let field_vars = (0..field_count).map(|i| {
                    quote::format_ident!("field_{}", i)
                });

                quote! {
                    impl FromBytes for #name {
                        type Output = Self;

                        fn from_value(value: rmpv::Value) -> ToBytesResult<Self::Output> {
                            let items = Vec::<rmpv::Value>::try_from(value)?;

                            #(#field_decodings)*

                            Ok(Self(#(#field_vars),*))
                        }
                    }
                }
            }
            Fields::Unit => {
                quote! {
                    impl FromBytes for #name {
                        type Output = Self;

                        fn from_value(value: rmpv::Value) -> ToBytesResult<Self::Output> {
                            let items = Vec::<rmpv::Value>::try_from(value)?;
                            if !items.is_empty() {
                                use std::io;
                                return Err(io::Error::new(
                                    io::ErrorKind::InvalidData,
                                    "Expected empty array for unit struct"
                                ).into());
                            }
                            Ok(Self)
                        }
                    }
                }
            }
        },
        Data::Enum(_) => {
            return syn::Error::new_spanned(
                &input,
                "FromBytes derive macro does not support enums yet",
            )
            .to_compile_error()
            .into();
        }
        Data::Union(_) => {
            return syn::Error::new_spanned(
                &input,
                "FromBytes derive macro does not support unions",
            )
            .to_compile_error()
            .into();
        }
    };

    TokenStream::from(expanded)
}
