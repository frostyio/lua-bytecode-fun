use ir::{Context};
use bytecode::lua51::{luac, deserialize_bytecode};

mod control_flow;

pub struct Options {
    pub flatten_control_flow: bool,
    pub scramble_opcodes: bool
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
			let mut flatten = control_flow::Flatten::new(ctx);
            flatten.flatten();
            self.include("pop");
            self.include("push");
            self.ctx = Some(flatten.get());
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