use darling::{ast, util, FromDeriveInput, FromField, FromMeta, ToTokens};
use proc_macro2::TokenStream;
use quote::quote;
use std::collections::BTreeMap;
use syn::{DeriveInput, GenericArgument, Ident, PathArguments, Type};

#[derive(FromDeriveInput, Debug)]
#[darling(attributes(effect), supports(struct_named))]
struct EffectStructReceiver {
    ident: Ident,
    name: Option<Type>,
    app: Option<Type>,
    data: ast::Data<util::Ignored, EffectFieldReceiver>,
}

#[derive(FromField, Debug)]
#[darling(attributes(effect))]
pub struct EffectFieldReceiver {
    ident: Option<Ident>,
    ty: Type,
}

impl ToTokens for EffectStructReceiver {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let ident = &self.ident;

        let name = match self.name {
            Some(ref name) => quote!(#name),
            None => {
                let x = Type::from_string("Effect").unwrap();
                quote!(#x)
            }
        };

        let app = match self.app {
            Some(ref app) => quote!(#app),
            None => {
                let x = Type::from_string("App").unwrap();
                quote!(#x)
            }
        };

        let fields = self
            .data
            .as_ref()
            .take_struct()
            .expect("Should never be enum")
            .fields;

        let fields: BTreeMap<Ident, (Ident, Type)> = fields
            .iter()
            .map(|f| {
                (
                    f.ident.as_ref().unwrap().to_owned(),
                    split_event_type(&f.ty),
                )
            })
            .collect();

        let events = fields.values().map(|(_, t2)| t2).collect::<Vec<_>>();
        if !events.windows(2).all(|win| {
            let t0 = win[0];
            let t1 = win[1];
            quote!(#t0).to_string() == quote!(#t1).to_string()
        }) {
            panic!("all fields should be generic over the same event type");
        }
        let event = events[0];

        let (variants, fields): (Vec<_>, Vec<_>) = fields.iter()
            .map(|(field_name, (variant, event))| {
                (
                    quote! { #variant(<#variant<#event> as ::crux_core::capability::Capability<#event>>::Operation) },
                    quote! { #field_name: #variant::new(context.with_effect(#name::#variant)) },
                )
            })
            .unzip();

        tokens.extend(quote! {
            #[derive(Clone, ::serde::Serialize, ::serde::Deserialize, Debug, PartialEq, Eq)]
            pub enum #name {
                #(#variants ,)*
            }

            impl ::crux_core::WithContext<#app, #name> for #ident {
                fn new_with_context(context: ::crux_core::capability::CapabilityContext<#name, #event>) -> #ident {
                    #ident {
                        #(#fields ,)*
                    }
                }
            }
        })
    }
}

pub(crate) fn effect_impl(input: &DeriveInput) -> TokenStream {
    let input = match EffectStructReceiver::from_derive_input(input) {
        Ok(v) => v,
        Err(e) => {
            return e.write_errors();
        }
    };

    quote!(#input)
}

fn split_event_type(ty: &Type) -> (Ident, Type) {
    match ty {
        Type::Path(p) if p.qself.is_none() => {
            // Get the first segment of the path (there should be only one)
            let path_segment = &p.path.segments.first().unwrap();
            let t1 = &path_segment.ident;
            let type_params = &path_segment.arguments;
            // It should have only one angle-bracketed param
            let generic_arg = match type_params {
                PathArguments::AngleBracketed(params) => params.args.first(),
                _ => None,
            };
            // This argument must be a type
            match generic_arg {
                Some(GenericArgument::Type(t2)) => Some((t1.clone(), t2.clone())),
                _ => None,
            }
        }
        _ => None,
    }
    .expect("capabilities should be generic over a single event type")
}

#[cfg(test)]
mod tests {
    use darling::{FromDeriveInput, ToTokens};
    use quote::quote;
    use syn::parse_str;

    use crate::effect::EffectStructReceiver;

    #[test]
    fn defaults() {
        let input = r#"
            #[derive(Effect)]
            pub struct Capabilities {
                pub render: Render<Event>,
            }
        "#;
        let input = parse_str(input).unwrap();
        let input = EffectStructReceiver::from_derive_input(&input).unwrap();

        let actual = quote!(#input);

        insta::assert_snapshot!(pretty_print(&actual), @r###"
        #[derive(Clone, ::serde::Serialize, ::serde::Deserialize, Debug, PartialEq, Eq)]
        pub enum Effect {
            Render(<Render<Event> as ::crux_core::capability::Capability<Event>>::Operation),
        }
        impl ::crux_core::WithContext<App, Effect> for Capabilities {
            fn new_with_context(
                context: ::crux_core::capability::CapabilityContext<Effect, Event>,
            ) -> Capabilities {
                Capabilities {
                    render: Render::new(context.with_effect(Effect::Render)),
                }
            }
        }
        "###);
    }

    #[test]
    fn full() {
        let input = r#"
            #[derive(Effect)]
            #[effect(name = "MyEffect", app = "MyApp")]
            pub struct MyCapabilities {
                pub http: Http<MyEvent>,
                pub key_value: KeyValue<MyEvent>,
                pub platform: Platform<MyEvent>,
                pub render: Render<MyEvent>,
                pub time: Time<MyEvent>,
            }
        "#;
        let input = parse_str(input).unwrap();
        let input = EffectStructReceiver::from_derive_input(&input).unwrap();

        let actual = quote!(#input);

        insta::assert_snapshot!(pretty_print(&actual), @r###"
        #[derive(Clone, ::serde::Serialize, ::serde::Deserialize, Debug, PartialEq, Eq)]
        pub enum MyEffect {
            Http(<Http<MyEvent> as ::crux_core::capability::Capability<MyEvent>>::Operation),
            KeyValue(
                <KeyValue<MyEvent> as ::crux_core::capability::Capability<MyEvent>>::Operation,
            ),
            Platform(
                <Platform<MyEvent> as ::crux_core::capability::Capability<MyEvent>>::Operation,
            ),
            Render(<Render<MyEvent> as ::crux_core::capability::Capability<MyEvent>>::Operation),
            Time(<Time<MyEvent> as ::crux_core::capability::Capability<MyEvent>>::Operation),
        }
        impl ::crux_core::WithContext<MyApp, MyEffect> for MyCapabilities {
            fn new_with_context(
                context: ::crux_core::capability::CapabilityContext<MyEffect, MyEvent>,
            ) -> MyCapabilities {
                MyCapabilities {
                    http: Http::new(context.with_effect(MyEffect::Http)),
                    key_value: KeyValue::new(context.with_effect(MyEffect::KeyValue)),
                    platform: Platform::new(context.with_effect(MyEffect::Platform)),
                    render: Render::new(context.with_effect(MyEffect::Render)),
                    time: Time::new(context.with_effect(MyEffect::Time)),
                }
            }
        }
        "###);
    }

    #[test]
    #[should_panic]
    fn should_panic_when_multiple_event_types() {
        let input = r#"
            #[derive(Effect)]
            pub struct Capabilities {
                pub render: Render<MyEvent>,
                pub time: Time<YourEvent>,
            }
        "#;
        let input = parse_str(input).unwrap();
        let input = EffectStructReceiver::from_derive_input(&input).unwrap();

        let mut actual = quote!();
        input.to_tokens(&mut actual);
    }

    fn pretty_print(ts: &proc_macro2::TokenStream) -> String {
        let file = syn::parse_file(&ts.to_string()).unwrap();
        prettyplease::unparse(&file)
    }
}
