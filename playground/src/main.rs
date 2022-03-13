use {
	bytecode,
	ir,
	obfuscation,
	std::{fs, env},
};
use clipboard::ClipboardProvider;
use clipboard::ClipboardContext;
mod tests;

fn main() {
	// get test file
	// let mut dir = env::current_dir().expect("unable to read directory");
	// dir.push("playground");
	// dir.push("examples");
	// dir.push("luac.out");
	// let test_out = fs::read(dir).expect("unable to read file");
	let test_out = bytecode::lua51::luac::load("playground/examples/test_file.lua");
	
	// deserialize
	let (header, proto) = bytecode::lua51::deserialize_bytecode(test_out.as_slice());
	println!("{:?}", proto);

	// serialize
	//let bytes = bytecode::lua51::serialize_bytecode(&header, &proto);

	// create ir
	let mut context = ir::Context::new(header, proto);
	context.map();

	// obfuscate
	let options = obfuscation::bytecode::Options {
		flatten_control_flow: true,
		scramble_opcodes: false,
		target_vm: obfuscation::bytecode::VM::Lua51
	};
	let mut obfuscate = obfuscation::bytecode::Obfuscate::new(options); 
	obfuscate.obfuscate(context);
	let p = obfuscate.get().unwrap();
	// println!("{:?}", p.chunk);
	let bytes = p.assemble();

	//
	let s = bytes.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(", ");
	let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
    ctx.set_contents(format!("{{{}}}", s)).unwrap();

	// write file
	let mut out = env::current_dir().expect("unable to read directory");
	out.push("out");
	out.push("test.out");

	// use bytecode::Hex;
	// println!("{}\n----\n{}", test_out.hex(None), bytes.hex(None));

	fs::write(out, bytes).expect("unable to write file");
	println!("wrote to file at out/test.out");

}