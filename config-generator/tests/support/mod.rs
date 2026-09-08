//! Typed legacy fixtures are a test oracle, never part of the generator runtime.
#![allow(dead_code, unused_imports)]
use config_generator::{dsl::InputField, schema::document};
pub use config_generator::{
	model::serialize,
	schema::{input_fields, options},
	validation::endpoint,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
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

pub fn build_configs(s: &State) -> Result<Value, String> {
	config_generator::model::build_configs(document().map_err(|e| e.to_string())?, &s.value())
}
pub fn redact_configs(v: &Value) -> Result<Value, String> {
	document().map_err(|e| e.to_string())?.redact(v).map_err(|e| e.to_string())
}
pub fn validate(s: &State) -> config_generator::validation::Errors {
	document().map(|d| d.validate(&s.value())).unwrap_or_default()
}
pub trait Binding {
	fn read(&self, s: &State) -> String;
	fn write(&self, s: &mut State, value: &str) -> bool;
	fn visible(&self, s: &State) -> bool;
}
impl Binding for InputField {
	fn read(&self, s: &State) -> String {
		self.read_value(&s.value())
	}
	fn write(&self, s: &mut State, value: &str) -> bool {
		let mut v = s.value();
		if !self.write_value(&mut v, value) {
			return false;
		}
		if let Ok(next) = serde_json::from_value(v) {
			*s = next;
			true
		} else {
			false
		}
	}
	fn visible(&self, s: &State) -> bool {
		document().is_ok_and(|d| self.visible_in(d, &s.value(), &s.value()).unwrap_or(false))
	}
}
