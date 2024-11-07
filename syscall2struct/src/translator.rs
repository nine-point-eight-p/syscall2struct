use std::path::Path;

use codegen::{Scope, Struct};
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

        // Generate syscall structs
        for func in self.parsed.functions() {
            let nr = match self.parsed.consts().find_sysno(&func.name.name, &ARCH) {
                // Use the syscall number for the arch if available
                Some(nr) => nr,
                // Use the default syscall number if not specified
                None => self
                    .parsed
                    .consts()
                    .find_sysno_for_any(&func.name.name)
                    .iter()
                    .find(|c| c.arch.is_empty())
                    .map(|c| c.as_uint().unwrap() as usize)
                    .expect(&format!("Syscall number not found: {}", func.name.name)),
            };
            scope.push_struct(self.translate_syscall(nr, func));
        }

        scope.to_string()
    }

    fn generate_import(&self, scope: &mut Scope) {
        scope.import("serde", "Deserialize");
        scope.import("heapless", "Vec");
        scope.import("syscalls::raw", "*");
        scope.import("syscall2struct_derive", "*");
        scope.import("syscall2struct_helpers", "*");
        scope.import("uuid", "Uuid");
    }

    fn translate_syscall(&self, nr: usize, function: &ParserFunction) -> Struct {
        let struct_name = function.name.name.to_case(Case::Pascal);

        let mut s = Struct::new(&struct_name);
        s.vis("pub");
        s.derive("Debug").derive("Deserialize");
        s.attr(format!("sysno({nr})"));

        // Select trait after we decide whether the syscall has mutable arguments
        let mut has_mut = false;

        // Generate a field for each argument
        for arg in function.args() {
            let parse_type = self.get_underlying_type(arg);
            let arg_name = arg.name.name.to_case(Case::Snake);

            let field = s.new_field(&arg_name, parse_type).vis("pub");
            if arg.arg_type().is_ptr() {
                if arg.direction() == Direction::In {
                    field.annotation("#[in_ptr]");
                } else {
                    field.annotation("#[out_ptr]");
                    has_mut = true;
                }
            }
        }
        // Generate a field for return value (if any)
        if function.output != ArgType::Void {
            s.field("id", "Uuid").vis("pub").allow("#[ret_val]");
        }

        // Select trait
        if has_mut {
            s.derive("MakeSyscallMut");
        } else {
            s.derive("MakeSyscall");
        }

        s
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
            // Resource type
            ArgType::Ident(ident) => {
                let ident_type = self.parsed.identifier_to_ident_type(ident).unwrap();
                let arg_type = match ident_type {
                    IdentType::Resource => self.parsed.resource_to_basic_type(ident).unwrap(),
                    _ => unimplemented!("Unsupported identifier type: {:?}", ident_type),
                };
                let fake_arg = Argument::new_fake(arg_type, Vec::new());
                self.get_underlying_type(&fake_arg)
            }
            _ => unimplemented!("Unsupported argument type: {:?}", arg.arg_type()),
        }
    }
}
