//! Framework-independent UI session. The browser submits actions, never a second DSL implementation.
mod view;
use serde::Deserialize;
use serde_json::Value;
pub use view::*;

use crate::{
	dsl::{Document, InputField},
	model::{build_configs, serialize},
	schema::State,
};

pub type Random = dyn FnMut(&mut [u8]) -> Result<(), String>;

/// Row identities cross the JS boundary as strings to preserve all u64 values.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Action {
	Set {
		field: String,
		value: String,
	},
	SetRow {
		collection: String,
		id: String,
		field: String,
		value: String,
	},
	Add {
		collection: String,
	},
	Remove {
		collection: String,
		id: String,
	},
	Generate {
		field: String,
	},
	GenerateRow {
		collection: String,
		id: String,
	},
}

pub struct Session {
	doc: Document,
	state: State,
}

impl Session {
	pub fn new(doc: Document) -> Self {
		let state = State::new(&doc);
		Self { doc, state }
	}

	/// Commit initialization only after every random request succeeds.
	pub fn initialize(&mut self, random: &mut Random) -> Result<(), String> {
		let mut next = self.state.clone();
		generate(&self.doc.fields, &mut next.data, random)?;
		for c in &self.doc.collections {
			for id in next.rows(&c.name) {
				if let Some(row) = next.row_mut(&c.name, id) {
					generate(&c.fields, row, random)?;
				}
			}
		}
		self.state = next;
		Ok(())
	}

	pub fn dispatch(&mut self, action: Action, random: &mut Random) -> Result<(), String> {
		// A failed edit/random request must not leave a partially updated row or selection.
		let mut next = self.state.clone();
		match action {
			Action::Set { field, value } => {
				if !self.doc.write_field(&mut next.data, &field, &value) {
					return Err("无法更新字段。".into());
				}
			}
			Action::SetRow {
				collection,
				id,
				field,
				value,
			} => {
				let c = self.collection(&collection)?;
				let field = c.fields.iter().find(|f| f.key == field).ok_or("输入字段不存在。")?;
				let row = next.row_mut(&collection, row_id(&id)?).ok_or("输入行不存在。")?;
				if !field.write_value(row, &value) {
					return Err("无法更新字段。".into());
				}
			}
			Action::Add { collection } => {
				let c = self.collection(&collection)?;
				let id = next.add(&self.doc, &collection).map_err(|e| e.to_string())?;
				let row = next.row_mut(&collection, id).ok_or("输入行不存在。")?;
				generate(&c.fields, row, random)?;
			}
			Action::Remove { collection, id } => {
				if !next.remove(&self.doc, &collection, row_id(&id)?) {
					return Err("当前输入行不可移除。".into());
				}
			}
			Action::Generate { field } => {
				let field = self.doc.fields.iter().find(|f| f.key == field).ok_or("输入字段不存在。")?;
				if field.generator.is_none() {
					return Err("字段未声明随机生成器。".into());
				}
				let mut candidate = next.data.clone();
				generate(std::slice::from_ref(field), &mut candidate, random)?;
				self.doc
					.write_field(&mut next.data, &field.key, &field.read_value(&candidate));
			}
			Action::GenerateRow { collection, id } => {
				let c = self.collection(&collection)?;
				let row = next.row_mut(&collection, row_id(&id)?).ok_or("输入行不存在。")?;
				generate(&c.fields, row, random)?;
			}
		}
		self.state = next;
		Ok(())
	}

	fn collection(&self, name: &str) -> Result<&crate::dsl::Collection, String> {
		self.doc
			.collections
			.iter()
			.find(|c| c.name == name)
			.ok_or_else(|| "输入集合不存在。".into())
	}

	fn selected(&self, requested: &str) -> Option<&crate::dsl::Export> {
		self.doc
			.exports
			.iter()
			.find(|e| e.name == requested && self.doc.shown(&e.when, &self.state.data))
			.or_else(|| self.doc.exports.iter().find(|e| self.doc.shown(&e.when, &self.state.data)))
	}

	fn format(&self) -> &str {
		self.doc
			.ui
			.format_field
			.as_ref()
			.and_then(|k| self.state.data[k].as_str())
			.unwrap_or("json")
	}

	/// Raw text is obtained explicitly for copy/download, independently of preview redaction.
	pub fn export(&self, requested: &str) -> Result<ExportFile, String> {
		let export = self.selected(requested).ok_or("请选择输出。")?;
		let configs = build_configs(&self.doc, &self.state.data)?;
		let config = configs.get(&export.name).ok_or("请选择输出。")?;
		Ok(ExportFile {
			filename: export.filename(self.format()),
			text: serialize(config, self.format())?,
		})
	}
}

fn row_id(id: &str) -> Result<u64, String> {
	id.parse().map_err(|_| "输入行标识无效。".into())
}

fn generate(fields: &[InputField], row: &mut Value, random: &mut Random) -> Result<(), String> {
	for field in fields {
		if let Some(generator) = &field.generator {
			let mut bytes = vec![0; generator.byte_count()];
			random(&mut bytes)?;
			row[&field.key] = generator.encode(&bytes).map_err(|e| e.to_string())?.into();
		}
	}
	Ok(())
}
