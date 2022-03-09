use std::{fs, process::Command, path::{Path}};

pub fn load(path: &str) -> Vec<u8> {
	let file = Path::new(&path);
	let file_name = file.file_stem().unwrap().to_str().unwrap();
	let name = format!("out/{}_c.out", &file_name);

	let mut command = Command::new("luac")
			.args(["-o", &name, path])
			.spawn()
			.expect("err");

	command.wait().expect("unable to run");
	
	fs::read(name).expect("unable to load file")
}