use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{Data, DeriveInput, Ident, LitInt, Type};

#[proc_macro_derive(MakeSyscall, attributes(sysno, in_ptr, ret_val))]
pub fn make_syscall(item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as DeriveInput);
    impl_make_syscall(input, false)
}

#[proc_macro_derive(MakeSyscallMut, attributes(sysno, in_ptr, out_ptr, ret_val))]
pub fn make_syscall_mut(item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as DeriveInput);
    impl_make_syscall(input, true)
}

fn impl_make_syscall(input: DeriveInput, mutable: bool) -> TokenStream {
    let trait_name = if mutable {
        "MakeSyscallMut"
    } else {
        "MakeSyscall"
    };
    let trait_ident = Ident::new(trait_name, Span::call_site());

    let struct_name = &input.ident;
    let data = match input.data {
        Data::Struct(data) => data,
        _ => panic!("{trait_name} can only be used on structs"),
    };

    // Get syscall number
    let sysno: LitInt = input
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("sysno"))
        .map(|attr| {
            attr.parse_args()
                .expect("Invalid syscall number, expected i32")
        })
        .expect(&format!(
            "{trait_name} requires a `sysno` attribute on the struct"
        ));
    let sysno: i32 = sysno
        .base10_parse()
        .expect("Invalid syscall number, expected i32");

    // Prepare arguments of the syscall
    let mut assignment = Vec::new();
    let mut idx = 0;
    for field in &data.fields {
        let field_name = field
            .ident
            .as_ref()
            .expect(&format!("{trait_name} can only be used with named fields"));
        let arg_name = Ident::new(&format!("arg{}", idx), Span::call_site());

        let is_in_ptr = field
            .attrs
            .iter()
            .any(|attr| attr.path().is_ident("in_ptr"));
        let is_out_ptr = field
            .attrs
            .iter()
            .any(|attr| attr.path().is_ident("out_ptr"));
        let is_ret_val = field
            .attrs
            .iter()
            .any(|attr| attr.path().is_ident("ret_val"));
        let is_resource = match &field.ty {
            Type::Path(path) => path
                .path
                .segments
                .iter()
                .last()
                .map(|segment| segment.ident == "SyscallResult")
                .unwrap_or(false),
            _ => false,
        };

        if is_ret_val {
            continue;
        }
        if is_resource && (is_in_ptr || is_out_ptr) {
            panic!("Syscalls returning pointers are not supported");
        }
        if is_in_ptr && is_out_ptr {
            panic!("Field {field_name} cannot be both in_ptr and out_ptr");
        }

        let line = if is_resource {
            quote! {
                let #arg_name = match self.#field_name {
                    SyscallResult::Ref(id) => results.get(&id).expect("Syscall result not found"),
                    SyscallResult::Value(val) => val as usize,
                };
            }
        } else if is_in_ptr {
            quote! { let #arg_name = self.#field_name.as_ptr(); }
        } else if is_out_ptr {
            quote! { let mut #arg_name = self.#field_name.as_mut_ptr(); }
        } else {
            quote! { let #arg_name = self.#field_name; }
        };

        assignment.push(line);
        idx += 1;
    }

    // Generate the syscall function call
    let syscall = Ident::new(&format!("syscall{}", idx), Span::call_site());
    let ref_self = if mutable {
        quote! { &mut self }
    } else {
        quote! { &self }
    };
    let args = (0..idx).map(|i| Ident::new(&format!("arg{}", i), Span::call_site()));
    let impl_block = quote! {
        impl #trait_ident for #struct_name {
            const NR: i32 = #sysno;

            fn call(#ref_self, results: &ResultContainer) -> isize {
                #(#assignment)*
                unsafe {
                    #syscall(Self::NR.into(), #(#args as usize),*) as isize
                }
            }
        }
    };

    impl_block.into()
}
