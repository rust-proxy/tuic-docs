use wasm_bindgen::prelude::*;

use crate::{schema, session::Session};

fn random(bytes: &mut [u8]) -> Result<(), String> {
	let error = || "当前浏览器无法安全生成随机值，请使用 HTTPS 或 localhost，或手动填写。".to_owned();
	web_sys::window()
		.ok_or_else(error)?
		.crypto()
		.map_err(|_| error())?
		.get_random_values_with_u8_array(bytes)
		.map_err(|_| error())?;
	Ok(())
}

fn js_error(error: impl std::fmt::Display) -> JsValue {
	JsValue::from_str(&error.to_string())
}

#[wasm_bindgen]
pub struct Engine {
	session: Session,
}

#[wasm_bindgen]
impl Engine {
	#[wasm_bindgen(constructor)]
	pub fn new() -> Result<Engine, JsValue> {
		Ok(Self {
			session: Session::new(schema::document().map_err(js_error)?.clone()),
		})
	}

	pub fn initialize(&mut self) -> Result<(), JsValue> {
		self.session.initialize(&mut random).map_err(js_error)
	}

	pub fn dispatch(&mut self, action: &str) -> Result<(), JsValue> {
		let action = serde_json::from_str(action).map_err(|_| js_error("无效的编辑操作。"))?;
		self.session.dispatch(action, &mut random).map_err(js_error)
	}

	pub fn snapshot(&self, selected: &str, reveal: bool) -> Result<String, JsValue> {
		serde_json::to_string(&self.session.snapshot(selected, reveal)).map_err(|_| js_error("无法读取界面状态。"))
	}

	pub fn export(&self, selected: &str) -> Result<String, JsValue> {
		serde_json::to_string(&self.session.export(selected).map_err(js_error)?).map_err(|_| js_error("无法导出配置。"))
	}
}
