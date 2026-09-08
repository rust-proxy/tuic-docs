//! Optional embedded application description; all engine APIs accept any Document.
use std::{collections::BTreeMap, sync::OnceLock};

use serde_json::Value;

use crate::dsl::{Document, DslError};
pub use crate::dsl::{InputField, InputKind};

pub const SOURCE: &str = include_str!(env!("CONFIG_SCHEMA_PATH"));
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

/// UI identity is separate from input data: user schemas may freely use a field named id.
#[derive(Clone, Debug, PartialEq)]
pub struct State {
	pub data: Value,
	ids: BTreeMap<String, Vec<u64>>,
	next_id: u64,
}
impl State {
	pub fn new(doc: &Document) -> Self {
		let mut state = Self {
			data: doc.defaults(),
			ids: BTreeMap::new(),
			next_id: 0,
		};
		for c in &doc.collections {
			let count = state.data[&c.name].as_array().map_or(0, Vec::len);
			let ids = (0..count)
				.map(|_| {
					let id = state.next_id;
					state.next_id += 1;
					id
				})
				.collect();
			state.ids.insert(c.name.clone(), ids);
		}
		state
	}
	pub fn rows(&self, name: &str) -> Vec<u64> {
		self.ids.get(name).cloned().unwrap_or_default()
	}
	pub fn index(&self, name: &str, id: u64) -> Option<usize> {
		self.ids.get(name)?.iter().position(|v| *v == id)
	}
	pub fn row(&self, name: &str, id: u64) -> Option<&Value> {
		self.data.get(name)?.get(self.index(name, id)?)
	}
	pub fn row_mut(&mut self, name: &str, id: u64) -> Option<&mut Value> {
		let i = self.index(name, id)?;
		self.data.get_mut(name)?.get_mut(i)
	}
	pub fn add(&mut self, doc: &Document, name: &str) -> Result<u64, DslError> {
		let collection = doc
			.collections
			.iter()
			.find(|c| c.name == name)
			.ok_or_else(|| DslError("输入集合不存在".into()))?;
		if !collection.visible_in(doc, &self.data)? || !collection.editable(doc, &self.data) {
			return Err(DslError("当前集合不可添加项目".into()));
		}
		let row = doc.collection_defaults(name)?;
		let rows = self
			.data
			.get_mut(name)
			.and_then(Value::as_array_mut)
			.ok_or_else(|| DslError("输入集合不存在".into()))?;
		let id = self.next_id;
		self.next_id += 1;
		rows.push(row);
		self.ids.entry(name.into()).or_default().push(id);
		Ok(id)
	}
	pub fn remove(&mut self, doc: &Document, name: &str, id: u64) -> bool {
		let Some(c) = doc.collections.iter().find(|c| c.name == name) else {
			return false;
		};
		if !c.editable(doc, &self.data) {
			return false;
		}
		let Some(i) = self.index(name, id) else {
			return false;
		};
		let Some(rows) = self.data.get_mut(name).and_then(Value::as_array_mut) else {
			return false;
		};
		if rows.len() <= c.min_items {
			return false;
		}
		rows.remove(i);
		if let Some(ids) = self.ids.get_mut(name) {
			ids.remove(i);
		}
		if let Some(key) = &c.selected_by
			&& let Some(selected) = self.data[key].as_u64()
		{
			self.data[key] = if selected == i as u64 {
				0
			} else if selected > i as u64 {
				selected - 1
			} else {
				selected
			}
			.into();
		}
		true
	}
}
