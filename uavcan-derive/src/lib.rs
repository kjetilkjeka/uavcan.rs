#![recursion_limit="128"]

extern crate regex;
extern crate proc_macro;
extern crate syn;
#[macro_use]
extern crate quote;

use regex::Regex;
use proc_macro::TokenStream;
use syn::Body;
use syn::Ident;
use quote::Tokens;

enum UavcanType {
    PrimitiveType,
    DynamicArray,
    StaticArray,
    Struct,
}


#[proc_macro_derive(UavcanStruct, attributes(DSDLSignature, DataTypeSignature, UavcanCrateName))]
pub fn uavcan_sized(input: TokenStream) -> TokenStream {
    let s = input.to_string();
    let ast = syn::parse_macro_input(&s).unwrap();
    let gen = impl_uavcan_struct(&ast);
    gen.parse().unwrap()
}

fn impl_uavcan_struct(ast: &syn::DeriveInput) -> quote::Tokens {
    let name = &ast.ident;

    // first handle the attributes
    let mut dsdl_signature = quote!{0x00};
    let mut data_type_signature = quote!{0x00};
    let mut crate_name = quote!{uavcan};
    
    for attr in &ast.attrs {
        if let syn::MetaItem::NameValue(ref ident, ref lit) = attr.value {
            if ident == "DSDLSignature" {
                if let syn::Lit::Str(ref lit_str, _) = *lit {
                    let value = Ident::from(lit_str.clone()); // hack needed since only string literals is supported for attributes
                    dsdl_signature = quote!{#value};
                } else {
                    panic!("DSDLSignature must be on the form \"0x123456789abc\"");
                }
            } else if ident == "DataTypeSignature" {
                if let syn::Lit::Str(ref lit_str, _) = *lit {
                    let value = Ident::from(lit_str.clone()); // hack needed since only string literals is supported for attributes
                    data_type_signature = quote!{#value};
                } else {
                    panic!("Data type signature must be on the form \"0x123456789abc\"");
                }
            } else if ident == "UavcanCrateName" {
                if let syn::Lit::Str(ref lit_str, _) = *lit {
                    let value = Ident::from(lit_str.clone()); // hack needed since only string literals is supported for attributes
                    crate_name = quote!{#value};
                } else {
                    panic!("Crate name must be on the form \"uavcan_alternative\"");
                }
            }

        }
    }

    let mut flattened_fields = Tokens::new();
    let mut serialize_body = Tokens::new();
    let mut deserialize_body = Tokens::new();
    
    match ast.body {
        Body::Enum(ref variants) => {
            flattened_fields.append(quote!{0});
            
            for variant in variants {
                if variant.data.fields().len() != 1 {
                    panic!("Enum variants must have exactly one field");
                } else if let Some(field) = variant.data.fields().first() {
                    let field_type = &field.ty;
                    
                    match classify_type(field_type) {
                        UavcanType::PrimitiveType | UavcanType::DynamicArray | UavcanType::StaticArray => flattened_fields.append(quote!{ + 1}),
                        UavcanType::Struct => flattened_fields.append(quote!{ + #field_type::FLATTENED_FIELDS_NUMBER}),
                    }
                }
            }

            serialize_body = quote!(unimplemented!("Serialization is not implemented for enum yet"));
            deserialize_body = quote!(unimplemented!("Serialization is not implemented for enum yet"));

        },
        Body::Struct(syn::VariantData::Struct(ref fields)) => {
            let mut field_index = Tokens::new();
            
            flattened_fields.append(quote!{0});
            field_index.append(quote!{0});
            
            for (i, field) in fields.iter().enumerate() {
                let field_ident = &field.ident;
                let field_type = &field.ty;
                
                match classify_type(field_type) {
                    UavcanType::PrimitiveType => flattened_fields.append(quote!{ + 1}),
                    UavcanType::StaticArray => flattened_fields.append(quote!{ + <#field_type as ::#crate_name::SerializableStatic>::FLATTENED_FIELDS_NUMBER}),
                    UavcanType::DynamicArray => {
                        let array_type = array_from_dynamic(field_type);
                        flattened_fields.append(quote!{ + <::#crate_name::types::Dynamic<#array_type> as ::#crate_name::SerializableStatic>::FLATTENED_FIELDS_NUMBER});
                    },
                    UavcanType::Struct => flattened_fields.append(quote!{ + <#field_type as ::#crate_name::SerializableStatic>::FLATTENED_FIELDS_NUMBER}),
                }
            

                if i != 0 { serialize_body.append(quote!{ else });}
                if i != 0 { deserialize_body.append(quote!{ else });}
                
                let field_length = match classify_type(field_type) {
                    UavcanType::PrimitiveType => quote!(1),
                    UavcanType::StaticArray => quote!(<#field_type as ::#crate_name::SerializableStatic>::FLATTENED_FIELDS_NUMBER),
                    UavcanType::DynamicArray => {
                        let array_type = array_from_dynamic(field_type);
                        quote!{<::#crate_name::types::Dynamic<#array_type> as ::#crate_name::SerializableStatic>::FLATTENED_FIELDS_NUMBER}
                    },
                    UavcanType::Struct => quote!{<#field_type as ::#crate_name::SerializableStatic>::FLATTENED_FIELDS_NUMBER},
                };
                
                serialize_body.append(quote!{if cursor.field >= (#field_index) && cursor.field < (#field_index) + #field_length {
                    cursor.field -= (#field_index);
                    if ::#crate_name::Serializable::partial_serialize(&self.#field_ident, cursor, buffer) == ::#crate_name::SerializationResult::Finished {
                        cursor.field += (#field_index);
                        cursor.bit = 0;
                    } else {
                        cursor.field += (#field_index);
                        return ::#crate_name::SerializationResult::BufferFull;
                    }
                }});

                deserialize_body.append(quote!{if cursor.field >= (#field_index) && cursor.field < (#field_index) + #field_length {
                    cursor.field -= (#field_index);
                    if ::#crate_name::Serializable::partial_deserialize(&mut self.#field_ident, cursor, buffer) == ::#crate_name::DeserializationResult::Finished {
                        cursor.field += (#field_index);
                        cursor.bit = 0;
                    } else {
                        cursor.field += (#field_index);
                        return ::#crate_name::DeserializationResult::BufferInsufficient;
                    }
                }});
                
                field_index.append(quote!{ + #field_length});
            }
        },
        Body::Struct(syn::VariantData::Unit) => {
            flattened_fields = quote!(0);
            serialize_body = quote!{
                assert_eq!(cursor.field, 0);
                cursor.bit = 0;
                cursor.field = 1;
            };
            deserialize_body = quote!{
                assert_eq!(cursor.field, 0);
                cursor.bit = 0;
                cursor.field = 1;
            };

        },
        _ => panic!("UavcanStruct is only derivable for enums and named structs"),
    };

    
    quote!{
        impl ::#crate_name::Struct for #name {
            const DSDL_SIGNATURE: u64 = #dsdl_signature;
            const DATA_TYPE_SIGNATURE: u64 = #data_type_signature;
        }

        impl ::#crate_name::SerializableStatic for #name {
            const FLATTENED_FIELDS_NUMBER: usize = #flattened_fields;
        }

        impl ::#crate_name::Serializable for #name {
            #[allow(unused_comparisons)]
            #[allow(unused_variables)]
            fn partial_serialize(&self, cursor: &mut ::#crate_name::Cursor, buffer: &mut ::#crate_name::SerializationBuffer) -> ::#crate_name::SerializationResult {
                assert!(cursor.field < Self::FLATTENED_FIELDS_NUMBER);
                while cursor.field != Self::FLATTENED_FIELDS_NUMBER{
                    assert!(cursor.field < Self::FLATTENED_FIELDS_NUMBER);
                    #serialize_body
                }
                ::#crate_name::SerializationResult::Finished
            }

            #[allow(unused_comparisons)]
            #[allow(unused_variables)]
            fn partial_deserialize(&mut self, cursor: &mut ::#crate_name::Cursor, buffer: &mut ::#crate_name::DeserializationBuffer) -> ::#crate_name::DeserializationResult {
                assert!(cursor.field < Self::FLATTENED_FIELDS_NUMBER);
                while cursor.field != Self::FLATTENED_FIELDS_NUMBER{
                    assert!(cursor.field < Self::FLATTENED_FIELDS_NUMBER);
                    #deserialize_body
                }
                ::#crate_name::DeserializationResult::Finished
            }


       }

    }
}

fn classify_type(ty: &syn::Ty) -> UavcanType {
    if is_primitive_type(ty) {
        UavcanType::PrimitiveType
    } else if is_dynamic_array(ty) {
        UavcanType::DynamicArray
    } else if is_static_array(ty) {
        UavcanType::StaticArray
    } else {
        UavcanType::Struct
    }
}

fn is_primitive_type(ty: &syn::Ty) -> bool {
    is_unsigned_primitive_type(ty) || is_signed_primitive_type(ty) || is_void_primitive_type(ty) || is_float_primitive_type(ty) || is_bool_primitive_type(ty)
}

fn is_bool_primitive_type(ty: &syn::Ty) -> bool {
    if let syn::Ty::Path(_, ref path) = *ty {
        let re = Regex::new(r"bool").unwrap();
        re.is_match(path.segments.as_slice().last().unwrap().ident.as_ref())
    } else {
        false
    }
}

fn is_unsigned_primitive_type(ty: &syn::Ty) -> bool {
    if let syn::Ty::Path(_, ref path) = *ty {
        let re = Regex::new(r"u([2-9]|[1-5][0-9]|6[0-4])").unwrap();
        re.is_match(path.segments.as_slice().last().unwrap().ident.as_ref())
    } else {
        false
    }
}

fn is_signed_primitive_type(ty: &syn::Ty) -> bool {
    if let syn::Ty::Path(_, ref path) = *ty {
        let re = Regex::new(r"i([2-9]|[1-5][0-9]|6[0-4])").unwrap();
        re.is_match(path.segments.as_slice().last().unwrap().ident.as_ref())
    } else {
        false
    }
}

fn is_float_primitive_type(ty: &syn::Ty) -> bool {
    if let syn::Ty::Path(_, ref path) = *ty {
        let re = Regex::new(r"f(16)|(32)|64").unwrap();
        re.is_match(path.segments.as_slice().last().unwrap().ident.as_ref())
    } else {
        false
    }
}

fn is_void_primitive_type(ty: &syn::Ty) -> bool {
    if let syn::Ty::Path(_, ref path) = *ty {
        let re = Regex::new(r"void([1-9]|[1-5][0-9]|6[0-4])").unwrap();
        re.is_match(path.segments.as_slice().last().unwrap().ident.as_ref())
    } else {
        false
    }
}

fn is_static_array(ty: &syn::Ty) -> bool {
    if let syn::Ty::Array(_, _) = *ty {
        true
    } else {
        false
    }
}

fn is_dynamic_array(type_name: &syn::Ty) -> bool {
    if let syn::Ty::Path(_, ref path) = *type_name {
        if path.segments.as_slice().last().unwrap().ident == syn::parse::ident("Dynamic").expect("") {
            return true;
        }
    }
    false
}

fn array_from_dynamic(type_name: &syn::Ty) -> Option<syn::Ty> {
    if let syn::Ty::Path(_, ref path) = *type_name {
        if path.segments.as_slice().last().unwrap().ident == syn::Ident::from("Dynamic") {
            if let syn::PathSegment{
                parameters: syn::PathParameters::AngleBracketed(syn::AngleBracketedParameterData{
                    ref types, ..
                }), ..
            } = *path.segments.as_slice().last().unwrap() {
                return Some(types[0].clone());
            }
        }
    }
    None
}
