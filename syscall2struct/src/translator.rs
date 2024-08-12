use std::path::Path;

use codegen::{Function, Impl, Scope, Struct};
use convert_case::{Case, Casing};
use syzlang_parser::parser::{
    Arch, ArgOpt, ArgType, Argument, Consts, Direction, Function as ParserFunction, IdentType,
    Parsed, Statement,
};

// Currently only support RISC-V 64-bit
const ARCH: Arch = Arch::Riscv64;

const MAX_BUFFER_LENGTH: usize = 4096;
const MAX_ARRAY_LENGTH: usize = 10;

/// A translator that converts syscall description to Rust code
pub struct SyscallTranslator {
    parsed: Parsed,
}

impl SyscallTranslator {
    /// Create a new translator from the description and constants file
    pub fn new(desc_path: &Path, const_path: &Path) -> Self {
        let builtin = Statement::from_file(Path::new("desc/builtin.txt")).unwrap();
        let desc = Statement::from_file(desc_path).unwrap();
        let stmts = [builtin, desc].concat();

        let mut consts = Consts::new(Vec::new());
        consts.create_from_file(const_path).unwrap();

        let mut parsed = Parsed::new(consts, stmts).unwrap();
        parsed.postprocess().unwrap();

        Self { parsed }
    }

    /// Translate the syscall description to Rust code
    pub fn translate(&self) -> String {
        let mut scope = Scope::new();

        // Generate imports
        self.generate_import(&mut scope);

        // Generate syscall functions
        for func in self.parsed.functions() {
            let nr = if let Some(nr) = self.parsed.consts().find_sysno(&func.name.name, &ARCH) {
                // Use the syscall number for the arch if available
                nr
            } else {
                // Use the default syscall number if not specified
                self.parsed
                    .consts()
                    .find_sysno_for_any(&func.name.name)
                    .iter()
                    .find(|c| c.arch.is_empty())
                    .map(|c| c.as_uint().unwrap() as usize)
                    .expect(&format!("Syscall number not found: {}", func.name.name))
            };
            let (s, i) = self.translate_syscall(nr, func);
            scope.push_struct(s);
            scope.push_impl(i);
        }

        format!("#![no_std]\n{}", scope.to_string())
    }

    fn generate_import(&self, scope: &mut Scope) {
        scope.import("serde", "Serialize");
        scope.import("serde", "Deserialize");
        scope.import("heapless", "Vec");
        scope.import("syscalls::raw", "*");
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
        func.ret("isize");

        let mut has_mut = false;

        for (idx, arg) in function.args().enumerate() {
            let parse_type = self.get_underlying_type(arg);
            let is_buffer = parse_type.starts_with("Vec<u8");

            let arg_name = arg.name.name.to_case(Case::Snake);
            let mutable = arg.arg_type().is_ptr() && arg.direction() == Direction::Out;
            if mutable {
                has_mut = true;
            }

            // Generate a struct field
            let field = s.new_field(&arg_name, parse_type).vis("pub");
            if mutable {
                field.annotation("#[serde(skip)]");
                if is_buffer {
                    s.field(&format!("{}_len", &arg_name), "u64").vis("pub");
                }
            }

            // Generate a line for the function
            if arg.arg_type().is_ptr() {
                if mutable {
                    if is_buffer {
                        func.line(format!(
                            "if let(Pointer::Addr(data)) = self.{} {{ data.resize(self.{}_len as usize, 0).unwrap(); }}",
                            idx, arg_name
                        ));
                    }
                    func.line(format!("let arg{} = self.{}.as_mut_ptr();", idx, arg_name));
                } else {
                    func.line(format!("let arg{} = self.{}.as_ptr();", idx, arg_name));
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
        func.line(format!("unsafe {{ {} as isize }}", syscall));

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
            ArgType::Int8
            | ArgType::Int16
            | ArgType::Int32
            | ArgType::Int64
            | ArgType::Intptr
            | ArgType::Flags => "u64".to_string(),
            // String
            ArgType::String | ArgType::StringNoz => format!("Vec<u8, {}>", MAX_BUFFER_LENGTH),
            // Array
            ArgType::Array => {
                let subarg = ArgOpt::get_subarg(&arg.opts).unwrap();
                if subarg.argtype == ArgType::Int8 {
                    format!("Vec<u8, {}>", MAX_ARRAY_LENGTH)
                } else {
                    let ty = self.get_underlying_type(&subarg);
                    format!("Vec<{}, {}>", ty, MAX_ARRAY_LENGTH)
                }
            }
            // Pointer
            ArgType::Ptr | ArgType::Ptr64 => {
                let subarg = ArgOpt::get_subarg(&arg.opts).unwrap();
                assert!(
                    !subarg.arg_type().is_ptr(),
                    "Nested pointer type is not supported"
                );

                let ty = self.get_underlying_type(&subarg);
                format!("Pointer<{}>", ty)
            }
            // Custom type
            ArgType::Ident(ident) => {
                let ident_type = self.parsed.identifier_to_ident_type(ident).unwrap();
                let arg_type = match ident_type {
                    IdentType::Resource => self.parsed.get_resource(ident).unwrap().arg_type(),
                    _ => unimplemented!(),
                };
                let fake_arg = Argument::new_fake(arg_type.clone(), Vec::new());
                self.get_underlying_type(&fake_arg)
            }
            _ => unimplemented!("Unsupported argument type: {:?}", arg.arg_type()),
        }
    }
}
