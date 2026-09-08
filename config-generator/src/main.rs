#[cfg(target_arch = "wasm32")]
mod app;

#[cfg(target_arch = "wasm32")]
fn main() {
	leptos::mount::mount_to_body(app::App);
	if let Some(element) = web_sys::window()
		.and_then(|w| w.document())
		.and_then(|d| d.get_element_by_id("cg-loading"))
	{
		element.remove();
	}
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
	eprintln!("This is a browser application. Run trunk serve --config config-generator/Trunk.toml.");
}
