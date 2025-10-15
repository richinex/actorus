//! Procedural macros for Actorus tools
//!
//! Provides the #[tool] and #[tool_fn] attribute macros for auto-generating tool metadata

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input, FnArg, Ident, LitBool, LitStr, Result, Token, Type,
};

/// Parse tool attribute arguments
struct ToolArgs {
    name: String,
    description: String,
}

impl Parse for ToolArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut name = String::new();
        let mut description = String::new();

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            let value: LitStr = input.parse()?;

            match key.to_string().as_str() {
                "name" => name = value.value(),
                "description" => description = value.value(),
                _ => {}
            }

            // Parse comma if not at end
            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(ToolArgs { name, description })
    }
}

/// Attribute macro for simple tool metadata generation
///
/// Usage:
/// ```ignore
/// #[tool!(name = "greet", description = "Greets a person")]
/// pub struct GreetTool;
/// ```
#[proc_macro_attribute]
pub fn tool(args: TokenStream, input: TokenStream) -> TokenStream {
    let tool_args = parse_macro_input!(args as ToolArgs);
    let input_item: syn::Item = parse_macro_input!(input);

    // Can work on either struct or impl block
    let struct_name = match &input_item {
        syn::Item::Struct(item_struct) => &item_struct.ident,
        syn::Item::Impl(item_impl) => {
            if let syn::Type::Path(type_path) = &*item_impl.self_ty {
                &type_path.path.segments.last().unwrap().ident
            } else {
                return syn::Error::new_spanned(input_item, "#[tool] requires a simple type")
                    .to_compile_error()
                    .into();
            }
        }
        _ => {
            return syn::Error::new_spanned(input_item, "#[tool] can only be applied to structs or impl blocks")
                .to_compile_error()
                .into();
        }
    };
    let tool_name = &tool_args.name;
    let tool_desc = &tool_args.description;

    // Parse field attributes to extract parameters (only for structs)
    let mut param_definitions = Vec::new();

    if let syn::Item::Struct(struct_item) = &input_item {
        if let syn::Fields::Named(fields) = &struct_item.fields {
            for field in &fields.named {
            let field_name = field.ident.as_ref().unwrap();
            let mut is_param = false;
            let mut param_desc = String::new();
            let mut required = true;

            // Check for #[param] attribute
            for attr in &field.attrs {
                if attr.path().is_ident("param") {
                    is_param = true;

                    // Parse the attribute meta for description and required
                    if let Ok(meta_list) = attr.meta.require_list() {
                        let _ = meta_list.parse_nested_meta(|meta| {
                            if meta.path.is_ident("description") {
                                let lit: LitStr = meta.value()?.parse()?;
                                param_desc = lit.value();
                            } else if meta.path.is_ident("required") {
                                let lit: LitBool = meta.value()?.parse()?;
                                required = lit.value;
                            }
                            Ok(())
                        });
                    }
                }
            }

            if is_param {
                let field_name_str = field_name.to_string();
                let field_type = &field.ty;

                // Determine type based on Rust type
                let type_str = quote!(#field_type).to_string();
                let param_type = if type_str.contains("String") || type_str.contains("str") {
                    "string"
                } else if type_str.contains("i64")
                    || type_str.contains("i32")
                    || type_str.contains("usize")
                    || type_str.contains("f64")
                    || type_str.contains("f32")
                {
                    "number"
                } else if type_str.contains("bool") {
                    "boolean"
                } else {
                    "string" // default
                };

                param_definitions.push(quote! {
                    actorus::tools::ToolParameter {
                        name: #field_name_str.to_string(),
                        param_type: #param_type.to_string(),
                        description: #param_desc.to_string(),
                        required: #required,
                    }
                });
            }
        }
        }
    }

    // Generate the output - add metadata method to the impl block or create new one
    let expanded = if let syn::Item::Impl(impl_block) = &input_item {
        // Extend the existing impl block
        let impl_block_items = &impl_block.items;
        quote! {
            impl #struct_name {
                #(#impl_block_items)*

                /// Auto-generated metadata method
                pub fn tool_metadata() -> actorus::tools::ToolMetadata {
                    actorus::tools::ToolMetadata {
                        name: #tool_name.to_string(),
                        description: #tool_desc.to_string(),
                        parameters: vec![
                            #(#param_definitions),*
                        ],
                    }
                }
            }
        }
    } else {
        // Create a new impl block for struct
        quote! {
            #input_item

            impl #struct_name {
                /// Auto-generated metadata method
                pub fn tool_metadata() -> actorus::tools::ToolMetadata {
                    actorus::tools::ToolMetadata {
                        name: #tool_name.to_string(),
                        description: #tool_desc.to_string(),
                        parameters: vec![
                            #(#param_definitions),*
                        ],
                    }
                }
            }
        }
    };

    TokenStream::from(expanded)
}

/// Function-style tool macro (MCP/Python style)
///
/// Usage:
/// ```ignore
/// #[tool_fn(name = "greet", description = "Greet a person")]
/// async fn greet(name: String, greeting: Option<String>) -> Result<String> {
///     let greeting = greeting.unwrap_or("Hello".to_string());
///     Ok(format!("{}, {}!", greeting, name))
/// }
/// ```
///
/// This generates a struct and Tool implementation from a simple function.
#[proc_macro_attribute]
pub fn tool_fn(args: TokenStream, input: TokenStream) -> TokenStream {
    let tool_args = parse_macro_input!(args as ToolArgs);
    let input_fn = parse_macro_input!(input as syn::ItemFn);

    let fn_name = &input_fn.sig.ident;
    let tool_name = &tool_args.name;
    let tool_desc = &tool_args.description;

    // Generate struct name from function name (e.g., greet -> GreetTool)
    let struct_name_str = format!(
        "{}Tool",
        fn_name
            .to_string()
            .split('_')
            .map(|s| {
                let mut c = s.chars();
                match c.next() {
                    None => String::new(),
                    Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                }
            })
            .collect::<String>()
    );
    let struct_name = syn::Ident::new(&struct_name_str, fn_name.span());

    // Extract parameters from function signature
    let mut param_definitions = Vec::new();
    let mut param_extractions = Vec::new();
    let mut fn_args = Vec::new();

    for arg in &input_fn.sig.inputs {
        if let FnArg::Typed(pat_type) = arg {
            if let syn::Pat::Ident(pat_ident) = &*pat_type.pat {
                let param_name = &pat_ident.ident;
                let param_name_str = param_name.to_string();
                let param_type = &pat_type.ty;

                // Determine if optional and base type
                let (is_optional, base_type_str) = match &**param_type {
                    Type::Path(type_path) => {
                        let type_str = quote!(#type_path).to_string();
                        if type_str.starts_with("Option") {
                            // Extract inner type from Option<T>
                            if let Some(seg) = type_path.path.segments.first() {
                                if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                                    if let Some(syn::GenericArgument::Type(inner_type)) =
                                        args.args.first()
                                    {
                                        (true, quote!(#inner_type).to_string())
                                    } else {
                                        (true, type_str)
                                    }
                                } else {
                                    (false, type_str)
                                }
                            } else {
                                (true, type_str)
                            }
                        } else {
                            (false, type_str)
                        }
                    }
                    _ => (false, quote!(#param_type).to_string()),
                };

                // Map Rust type to tool parameter type
                let (param_type_name, is_struct) = if base_type_str.contains("String") || base_type_str.contains("str") {
                    ("string", false)
                } else if base_type_str.contains("i64")
                    || base_type_str.contains("i32")
                    || base_type_str.contains("f64")
                    || base_type_str.contains("f32")
                    || base_type_str.contains("usize")
                {
                    ("number", false)
                } else if base_type_str.contains("bool") {
                    ("boolean", false)
                } else {
                    // Assume it's a custom struct/type that needs JSON deserialization
                    ("object", true)
                };

                // Generate parameter metadata
                let is_required = !is_optional;
                param_definitions.push(quote! {
                    actorus::tools::ToolParameter {
                        name: #param_name_str.to_string(),
                        param_type: #param_type_name.to_string(),
                        description: format!("Parameter: {}", #param_name_str),
                        required: #is_required,
                    }
                });

                // Generate parameter extraction logic
                if is_optional {
                    if param_type_name == "string" {
                        param_extractions.push(quote! {
                            let #param_name = args.get(#param_name_str)
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string());
                        });
                    } else if param_type_name == "number" {
                        // For Option<number>, we extract as the original Rust type
                        param_extractions.push(quote! {
                            let #param_name = args.get(#param_name_str)
                                .and_then(|v| v.as_i64())
                                .map(|n| n as #param_type);
                        });
                    } else if param_type_name == "boolean" {
                        param_extractions.push(quote! {
                            let #param_name = args.get(#param_name_str)
                                .and_then(|v| v.as_bool());
                        });
                    } else if is_struct {
                        // For Option<Struct>, deserialize from JSON
                        param_extractions.push(quote! {
                            let #param_name = args.get(#param_name_str)
                                .and_then(|v| serde_json::from_value::<#param_type>(v.clone()).ok());
                        });
                    }
                    fn_args.push(quote! { #param_name });
                } else {
                    // Required parameter
                    if param_type_name == "string" {
                        param_extractions.push(quote! {
                            let #param_name = actorus::validate_required_string!(args, #param_name_str).to_string();
                        });
                    } else if param_type_name == "number" {
                        // For required numbers, cast to the exact type
                        param_extractions.push(quote! {
                            let #param_name = actorus::validate_required_number!(args, #param_name_str) as #param_type;
                        });
                    } else if is_struct {
                        // For required struct, deserialize from JSON
                        param_extractions.push(quote! {
                            let #param_name = serde_json::from_value::<#param_type>(
                                args.get(#param_name_str)
                                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: {}", #param_name_str))?
                                    .clone()
                            )?;
                        });
                    } else {
                        param_extractions.push(quote! {
                            let #param_name = actorus::validate_required_string!(args, #param_name_str).to_string();
                        });
                    }
                    fn_args.push(quote! { #param_name });
                }
            }
        }
    }

    // Extract function parts
    let fn_sig = &input_fn.sig;
    let fn_block = &input_fn.block;
    let fn_vis = &input_fn.vis;

    // Generate the complete tool implementation
    let expanded = quote! {
        // Keep original function - suppress false unused warnings
        #[allow(dead_code, unused_variables)]
        #fn_vis #fn_sig {
            #fn_block
        }

        // Generate tool struct
        #[derive(Clone)]
        pub struct #struct_name;

        impl #struct_name {
            pub fn new() -> Self {
                Self
            }

            pub fn tool_metadata() -> actorus::tools::ToolMetadata {
                actorus::tools::ToolMetadata {
                    name: #tool_name.to_string(),
                    description: #tool_desc.to_string(),
                    parameters: vec![
                        #(#param_definitions),*
                    ],
                }
            }
        }

        #[async_trait::async_trait]
        impl actorus::tools::Tool for #struct_name {
            fn metadata(&self) -> actorus::tools::ToolMetadata {
                Self::tool_metadata()
            }

            fn validate(&self, args: &serde_json::Value) -> anyhow::Result<()> {
                // Auto-generated validation
                #(#param_extractions)*
                Ok(())
            }

            async fn execute(&self, args: serde_json::Value) -> anyhow::Result<actorus::tools::ToolResult> {
                self.validate(&args)?;

                // Extract parameters
                #(#param_extractions)*

                // Call original function
                let result = #fn_name(#(#fn_args),*).await?;

                actorus::tool_result!(success: result)
            }
        }
    };

    TokenStream::from(expanded)
}
