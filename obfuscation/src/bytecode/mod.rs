use ir::{Context};
use bytecode::lua51::{luac, deserialize_bytecode};

mod control_flow;

pub struct VMConfig {
    max_stack_size: usize
}

// assuming all registers are u8 currently; will change in the future
// type U8 = u8;
pub enum VM {
    Lua51
}
impl VM {
    pub fn get(&self) -> VMConfig {
        match self {
            Self::Lua51 => VMConfig {
                max_stack_size: 250
            }
        }
    }
}

pub struct Options {
    pub flatten_control_flow: bool,
    pub scramble_opcodes: bool,
    pub target_vm: VM
}

pub struct Obfuscate {
    ctx: Option<Context>,
    options: Options,
    includes: Vec<String>
}

impl Obfuscate {
    pub fn new(options: Options) -> Self {
        Self {
            ctx: None,
            options,
            includes: vec![]
        }
    }

    pub fn include(&mut self, file: &str) {
        self.includes.push(file.to_string());
    }

    pub fn obfuscate(&mut self, ctx: Context) {
		if self.options.flatten_control_flow {
			let flattened = control_flow::flatten(&ctx, &ctx.chunk, &self.options);
            self.ctx = Some(flattened);
		}

        // add includes to protos
        if let Some(ctx) = &mut self.ctx {
            for include in &self.includes {
                let bytecode = luac::load(format!("obfuscation/src/bytecode/includes/{}.lua", include).as_str());
                let (_, proto) = deserialize_bytecode(&bytecode);
                ctx.chunk.prototypes.push(proto);
            }
        }

        if let Some(ctx) = &self.ctx {
            println!("\n-- obfuscated view --");
            ctx.view();
        }
    }

    pub fn get(self) -> Option<Context> {
        self.ctx
    }
}