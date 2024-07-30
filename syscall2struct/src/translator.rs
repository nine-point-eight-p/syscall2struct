use std::path::Path;

use codegen::{Function, Impl, Scope, Struct};
use convert_case::{Case, Casing};
use syzlang_parser::parser::{
    Arch, ArgOpt, ArgType, Argument, Consts, Direction, Function as ParserFunction, IdentType,
    Parsed, Statement,
};

// Currently only support RISC-V 64-bit
const ARCH: Arch = Arch::Riscv64;

/// A translator that converts syscall description to Rust code
pub struct SyscallTranslator {
    parsed: Parsed,
}

impl SyscallTranslator {
    /// Create a new translator from the description and constants file
    pub fn new(desc_path: &Path, const_path: &Path) -> Self {
        let stmts = Statement::from_file(desc_path).unwrap();
        let mut consts = Consts::new(Vec::new());
        consts.create_from_file(const_path).unwrap();
        let parsed = Parsed::new(consts, stmts).unwrap();
        Self { parsed }
    }

    /// Translate the syscall description to Rust code
    pub fn translate(&self) -> String {
        let mut scope = Scope::new();

        // Generate imports
        self.generate_import(&mut scope);

        // Generate syscall functions
        for func in self.parsed.functions() {
            let nr = self
                .parsed
                .consts()
                .find_sysno(&func.name.name, &ARCH)
                .expect(&format!("Syscall number not found: {}", func.name.name));
            let (s, i) = self.translate_syscall(nr, func);
            scope.push_struct(s);
            scope.push_impl(i);
        }

        format!("#![no_std]\n{}", scope.to_string())
    }

    fn generate_import(&self, scope: &mut Scope) {
        scope.import("serde", "Serialize");
        scope.import("serde", "Deserialize");

        for arg_num in 0..7 {
            scope.import("syscalls::raw", format!("syscall{}", arg_num).as_str());
        }

        scope.import("syscall2struct_helpers", "*");
    }

    fn translate_syscall(&self, nr: usize, function: &ParserFunction) -> (Struct, Impl) {
        let struct_name = function.name.name.to_case(Case::Pascal);

        let mut s = Struct::new(&struct_name);
        s.vis("pub");
        s.derive("Debug").derive("Serialize").derive("Deserialize");

        let mut i = Impl::new(struct_name);
        // Select trait after we decide whether the syscall has mutable arguments
        i.associate_const("NR", "usize", nr.to_string(), "");

        // The function for impl, will be added at the end
        let mut func = Function::new("call");
        func.ret("usize");

        let mut has_ref = false;
        let mut has_mut = false;

        for (idx, arg) in function.args().enumerate() {
            let parse_type = self.get_underlying_type(arg);
            if parse_type.starts_with("&") {
                has_ref = true;
            }

            let arg_name = arg.name.name.to_case(Case::Snake);
            let mutable = arg.arg_type().is_ptr() && arg.direction() == Direction::Out;
            if mutable {
                has_mut = true;
            }

            // Generate a struct field
            let field = s.new_field(&arg_name, parse_type).vis("pub");
            if mutable {
                field.annotation("#[serde(skip)]");
            }

            // Generate a line for the function
            if arg.arg_type().is_ptr() {
                if mutable {
                    func.line(format!(
                        "let arg{} = (&mut self.{}).as_mut_ptr();",
                        idx, arg_name
                    ));
                } else {
                    func.line(format!("let arg{} = (&self.{}).as_ptr();", idx, arg_name));
                }
            } else {
                func.line(format!("let arg{} = self.{};", idx, arg_name));
            }
        }

        // Make syscall
        let mut syscall = format!("syscall{}({}.into(), ", function.args().len(), nr);
        for idx in 0..function.args().len() {
            syscall += format!("arg{} as usize, ", idx).as_str();
        }
        syscall += ")";
        func.line(format!("unsafe {{ {} }}", syscall));

        // Add lifetime generics if needed
        if has_ref {
            s.generic("'a");
            i.generic("'a").target_generic("'a");
        }

        // Select trait
        if has_mut {
            i.impl_trait("MakeSyscallMut");
            func.arg_mut_self();
        } else {
            i.impl_trait("MakeSyscall");
            func.arg_ref_self();
        }

        // Add function to impl
        i.push_fn(func);

        (s, i)
    }

    fn get_underlying_type(&self, arg: &Argument) -> String {
        let arg_type = arg.arg_type();
        match arg_type {
            // Integer
            ArgType::Int8 => "i8".to_string(),
            ArgType::Int16 => "i16".to_string(),
            ArgType::Int32 => "i32".to_string(),
            ArgType::Int64 => "i64".to_string(),
            ArgType::Intptr => "isize".to_string(),
            // String
            ArgType::String | ArgType::StringConst => "str".to_string(),
            ArgType::Ident(ident) if ident.name == "filename" => "str".to_string(),
            // Pointer
            ArgType::Ptr | ArgType::Ptr64 => {
                let subarg =
                    ArgOpt::get_subarg(&arg.opts).expect("Pointer type without underlying type");
                assert!(
                    !subarg.arg_type().is_ptr(),
                    "Nested pointer type is not supported"
                );

                let ty = self.get_underlying_type(&subarg);
                if ty == "str" || ty == "[u8]" {
                    format!("&'a {}", ty) // Hold reference for dynamic-size types
                } else {
                    ty // Hold value for fixed-size types
                }
            }
            // Custom type
            ArgType::Ident(ident) => {
                let ident_type = self.parsed.identifier_to_ident_type(ident).unwrap();
                let arg_type = match ident_type {
                    IdentType::Resource => self.parsed.get_resource(ident).unwrap().arg_type(),
                    _ => unimplemented!(),
                };
                let fake_subarg = Argument::new_fake(arg_type.clone(), Vec::new());
                self.get_underlying_type(&fake_subarg)
            }
            _ => unimplemented!("Unsupported argument type: {:?}", arg.arg_type()),
        }
    }
}
