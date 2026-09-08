use std::sync::OnceLock;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::dsl::{Document, DslError};
pub use crate::dsl::{InputField, InputKind};

pub const SOURCE: &str = include_str!("../schema/tuic.xml");
pub fn document() -> Result<&'static Document, DslError> {
	static DOCUMENT: OnceLock<Result<Document, DslError>> = OnceLock::new();
	DOCUMENT
		.get_or_init(|| Document::parse(SOURCE))
		.as_ref()
		.map_err(Clone::clone)
}
pub fn input_fields() -> &'static [InputField] {
	document().map(|d| d.fields.as_slice()).unwrap_or_default()
}
pub fn options(name: &str) -> &'static [(String, String)] {
	input_fields()
		.iter()
		.find(|f| f.key == name)
		.map(|f| f.options.as_slice())
		.unwrap_or_default()
}
pub fn collection_fields(name: &str) -> &'static [InputField] {
	document()
		.ok()
		.and_then(|d| d.collections.iter().find(|c| c.name == name))
		.map(|c| c.fields.as_slice())
		.unwrap_or_default()
}
pub fn version() -> &'static str {
	document().map(|d| d.target_version.as_str()).unwrap_or_default()
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct State {
	pub mode: String,
	pub format: String,
	pub host: String,
	pub port: String,
	pub listen: String,
	pub tls_mode: String,
	pub hostname: String,
	pub certificate: String,
	pub private_key: String,
	pub email: String,
	pub data_dir: String,
	pub sni: String,
	pub insecure: bool,
	pub local: String,
	pub local_auth: bool,
	pub local_username: String,
	pub local_password: String,
	pub log_level: String,
	pub zero_rtt: bool,
	pub controller: String,
	pub lazy: bool,
	pub reconnect: bool,
	pub initial_backoff: String,
	pub max_backoff: String,
	pub users: Vec<User>,
	pub active_user: usize,
	pub forwards: Vec<Forward>,
	#[serde(flatten)]
	pub extra: std::collections::BTreeMap<String, Value>,
}
impl State {
	pub fn initial() -> Result<Self, String> {
		serde_json::from_value(document().map_err(|e| e.to_string())?.defaults())
			.map_err(|_| "输入描述与页面状态不匹配。".into())
	}
	pub fn value(&self) -> Value {
		serde_json::to_value(self).unwrap_or(Value::Null)
	}
}
impl InputField {
	pub fn read(&self, s: &State) -> String {
		self.read_value(&s.value())
	}
	pub fn write(&self, s: &mut State, value: &str) -> bool {
		let mut data = s.value();
		if !self.write_value(&mut data, value) {
			return false;
		}
		if let Ok(next) = serde_json::from_value(data) {
			*s = next;
			true
		} else {
			false
		}
	}
	pub fn visible(&self, s: &State) -> bool {
		let data = s.value();
		document().is_ok_and(|d| self.visible_in(d, &data, &data).unwrap_or(false))
	}
}
pub fn has_server(s: &State) -> bool {
	condition("server", s)
}
pub fn has_client(s: &State) -> bool {
	condition("client", s)
}
pub fn acme(s: &State) -> bool {
	condition("acme", s)
}
fn condition(name: &str, s: &State) -> bool {
	let root = s.value();
	document().and_then(|d| d.condition(name, &root, &root)).unwrap_or(false)
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct User {
	#[serde(default)]
	pub id: u64,
	pub uuid: String,
	pub password: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Forward {
	#[serde(default)]
	pub id: u64,
	pub protocol: String,
	pub listen: String,
	pub remote: String,
	pub timeout: String,
}
impl Forward {
	pub fn initial() -> Result<Self, String> {
		serde_json::from_value(
			document()
				.map_err(|e| e.to_string())?
				.collection_defaults("forwards")
				.map_err(|e| e.to_string())?,
		)
		.map_err(|_| "转发描述与页面状态不匹配。".into())
	}
}

impl State {
	pub fn remove_user(&mut self, id: u64) {
		if self.users.len() <= 1 {
			return;
		}
		if let Some(index) = self.users.iter().position(|u| u.id == id) {
			self.users.remove(index);
			if self.active_user == index {
				self.active_user = 0;
			} else if self.active_user > index {
				self.active_user -= 1;
			}
		}
	}
}

pub fn collection_visible(name: &str, s: &State) -> bool {
	document().is_ok_and(|d| {
		d.collections
			.iter()
			.find(|c| c.name == name)
			.is_some_and(|c| c.visible_in(d, &s.value()).unwrap_or(false))
	})
}
