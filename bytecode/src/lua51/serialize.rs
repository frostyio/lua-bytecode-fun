use super::{Writer, Header, Proto, Constants, instruction::Opcode};

fn dump_header(writer: &mut Writer, header: &Header) {
	writer.bytes(b"\x1BLua".to_vec());
	writer.byte(b'\x51');
	writer.byte(0);
	writer.byte(1);
	writer.byte(header.0);
	writer.byte(header.1);
	writer.byte(header.2);
	writer.byte(header.3);
	writer.byte(0);
}

fn dump_vector<I>(writer: &mut Writer, list: I, n: u32, header: &Header, dump: fn(&mut Writer, header: &Header, I::Item)) where I: Iterator {
	writer.int(n, header.0);
	list.enumerate().for_each(|(_i, v)| dump(writer, header, v));
}

fn dump_chunk(writer: &mut Writer, header: &Header, proto: &Proto) {
	writer.string(&proto.source, header.1);
	writer.int(proto.line_defined, header.0);
	writer.int(proto.last_line_defined, header.0);
	writer.byte(proto.nupvals);
	writer.byte(proto.nparams);
	writer.byte(proto.is_vararg_flag);
	writer.byte(proto.max_stack_size);

	// instructions
	dump_vector(writer, 
		proto.instructions.iter(), proto.instructions.len() as u32, header, 
		|writer, header, instr| {
			if let Opcode::NOP = instr.0 {
				println!("almost serialzied a NOP! skipping");
			} else {
				writer.int(instr.serialize(), header.2);
			}
	});

	// constants
	dump_vector(writer, 
		proto.constants.iter(), proto.constants.len() as u32, header, 
		|writer, header, kst| {
			match kst {
				Constants::Nil => writer.byte(0),
				&Constants::Boolean(b) => {writer.byte(1); writer.byte(b as u8)},
				Constants::String(s) => {writer.byte(4); writer.string(s, header.1)},
				&Constants::Number(n) => {writer.byte(3); writer.number(n, header.0)}
			}
	});

	// protos
	dump_vector(writer, proto.prototypes.iter(), proto.prototypes.len() as u32, header, |writer, header, proto| 
		dump_chunk(writer, header, proto));

	// source lines
	if let Some(source_lines) = &proto.source_lines {
		dump_vector(writer, source_lines.iter(), source_lines.len() as u32, header, |writer, header, l| {
			writer.int(*l, header.0)
		});
	} else {
		writer.int(0, header.0);
	}

	// locals
	if let Some(locals) = &proto.locals {
		dump_vector(writer, locals.iter(), locals.len() as u32, header, |writer, header, local| {
			writer.string(&local.0, header.1);
			writer.int(local.1, header.0);
			writer.int(local.2, header.0);
		});
	} else {
		writer.int(0, header.0);
	}

	// upvalues
	if let Some(upvals) = &proto.upvals {
		dump_vector(writer, upvals.iter(), upvals.len() as u32, header, |writer, header, upval| {
			writer.string(upval, header.1);
		});
	} else {
		writer.int(0, header.0);
	}
}

pub fn serialize_bytecode(header: &Header, proto: &Proto) -> Vec<u8> {
	let mut writer = Writer::new();

	dump_header(&mut writer, header);
	dump_chunk(&mut writer, header, proto);

	writer.as_bytes().into()
}