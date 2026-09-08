fn main() {
	println!("cargo:rerun-if-env-changed=CONFIG_SCHEMA");
	let root = std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap_or_default());
	let source = std::env::var_os("CONFIG_SCHEMA")
		.map(std::path::PathBuf::from)
		.unwrap_or_else(|| root.join("schema/config.xml"));
	let source = if source.is_absolute() { source } else { root.join(source) };
	println!("cargo:rerun-if-changed={}", source.display());
	println!("cargo:rustc-env=CONFIG_SCHEMA_PATH={}", source.display());
}
