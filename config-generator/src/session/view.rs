use serde::Serialize;

use super::*;
use crate::{
	dsl::{Collection, InputKind, Notice, Section},
	validation::Errors,
};

#[derive(Serialize)]
pub struct FieldView {
	pub key: String,
	pub path: String,
	pub label: String,
	pub hint: String,
	pub placeholder: String,
	pub kind: &'static str,
	pub options: Vec<(String, String)>,
	pub value: String,
	pub visible: bool,
	pub error: String,
	pub generated: bool,
}

#[derive(Serialize)]
pub struct RowView {
	pub id: String,
	pub number: usize,
	pub visible: bool,
	pub fields: Vec<FieldView>,
}

#[derive(Serialize)]
pub struct CollectionView {
	pub name: String,
	pub label: String,
	pub hint: String,
	pub add_label: String,
	pub generate_label: String,
	pub generated: bool,
	pub visible: bool,
	pub editable: bool,
	pub removable: bool,
	pub selector: Option<FieldView>,
	pub rows: Vec<RowView>,
}

#[derive(Serialize)]
pub struct NoticeView {
	pub text: String,
	pub visible: bool,
}

#[derive(Serialize)]
pub struct SectionView {
	pub name: String,
	pub label: String,
	pub detail: String,
	pub collapsed: bool,
	pub visible: bool,
	pub fields: Vec<FieldView>,
	pub collections: Vec<CollectionView>,
	pub notices: Vec<NoticeView>,
}

#[derive(Serialize)]
pub struct Branding {
	pub title: String,
	pub brand: String,
	pub mark: String,
	pub eyebrow: String,
	pub description: String,
	pub reference: String,
	pub export_hint: String,
}

#[derive(Serialize)]
pub struct OutputView {
	pub name: String,
	pub label: String,
	pub visible: bool,
}

#[derive(Serialize)]
pub struct ExportFile {
	pub filename: String,
	pub text: String,
}

#[derive(Serialize)]
pub struct Snapshot {
	pub ui: Branding,
	pub sections: Vec<SectionView>,
	pub mode: Option<FieldView>,
	pub format: Option<FieldView>,
	pub notices: Vec<NoticeView>,
	pub errors: Errors,
	pub outputs: Vec<OutputView>,
	pub selected: String,
	pub filename: String,
	pub command: String,
	pub preview: String,
	pub valid: bool,
}

impl Session {
	pub fn snapshot(&self, requested: &str, reveal: bool) -> Snapshot {
		let doc = &self.doc;
		let ui = &doc.ui;
		let root = &self.state.data;
		let errors = doc.validate(root);
		let selected = self.selected(requested);
		let preview = build_configs(doc, root).and_then(|configs| {
			let configs = if reveal {
				configs
			} else {
				doc.redact(&configs).map_err(|e| e.to_string())?
			};
			let export = selected.ok_or("请选择输出。")?;
			serialize(configs.get(&export.name).ok_or("请选择输出。")?, self.format())
		});
		Snapshot {
			ui: Branding {
				title: ui.title.clone(),
				brand: ui.brand.clone(),
				mark: ui.mark.clone(),
				eyebrow: ui.eyebrow.clone(),
				description: ui.description.clone(),
				reference: ui.reference.clone(),
				export_hint: ui.export_hint.clone(),
			},
			sections: ui
				.sections
				.iter()
				.map(|section| self.section_view(section, &errors))
				.collect(),
			mode: self.top_field(&ui.mode_field, &errors),
			format: self.top_field(&ui.format_field, &errors),
			notices: self.notices(&ui.notices),
			outputs: doc
				.exports
				.iter()
				.map(|e| OutputView {
					name: e.name.clone(),
					label: e.label.clone(),
					visible: doc.shown(&e.when, root),
				})
				.collect(),
			selected: selected.map(|e| e.name.clone()).unwrap_or_default(),
			filename: selected.map(|e| e.filename(self.format())).unwrap_or_default(),
			command: selected.map(|e| e.command(self.format())).unwrap_or_default(),
			valid: preview.is_ok(),
			preview: preview.unwrap_or_else(|_| "填写左侧配置，预览将在校验通过后显示。".into()),
			errors,
		}
	}

	fn top_field(&self, key: &Option<String>, errors: &Errors) -> Option<FieldView> {
		key.as_ref()
			.and_then(|k| self.doc.fields.iter().find(|f| &f.key == k))
			.map(|f| self.field_view(f, &self.state.data, f.key.clone(), errors))
	}

	fn section_view(&self, section: &Section, errors: &Errors) -> SectionView {
		let doc = &self.doc;
		let regular = |f: &&InputField| {
			f.section == section.name
				&& doc.ui.mode_field.as_ref() != Some(&f.key)
				&& doc.ui.format_field.as_ref() != Some(&f.key)
				&& !doc.collections.iter().any(|c| c.selected_by.as_ref() == Some(&f.key))
		};
		SectionView {
			name: section.name.clone(),
			label: section.label.clone(),
			detail: section.detail.clone(),
			collapsed: section.collapsed,
			visible: doc.shown(&section.when, &self.state.data),
			fields: doc
				.fields
				.iter()
				.filter(regular)
				.map(|f| self.field_view(f, &self.state.data, f.key.clone(), errors))
				.collect(),
			collections: doc
				.collections
				.iter()
				.filter(|c| c.section == section.name)
				.map(|c| self.collection_view(c, errors))
				.collect(),
			notices: self.notices(&section.notices),
		}
	}

	fn collection_view(&self, c: &Collection, errors: &Errors) -> CollectionView {
		let root = &self.state.data;
		let ids = self.state.rows(&c.name);
		let mut selector = self.top_field(&c.selected_by, errors);
		if let Some(f) = &mut selector {
			f.options = ids
				.iter()
				.enumerate()
				.map(|(i, _)| (i.to_string(), format!("{} {}", c.label, i + 1)))
				.collect();
			f.kind = "select";
			f.visible = self.doc.shown(&c.select_when, root) && ids.len() >= 2;
		}
		let rows = ids
			.iter()
			.enumerate()
			.filter_map(|(i, id)| {
				self.state.row(&c.name, *id).map(|row| RowView {
					id: id.to_string(),
					number: i + 1,
					visible: c.row_visible(&self.doc, root, i).unwrap_or(false),
					fields: c
						.fields
						.iter()
						.map(|f| self.field_view(f, row, format!("{}.{i}.{}", c.name, f.key), errors))
						.collect(),
				})
			})
			.collect();
		CollectionView {
			name: c.name.clone(),
			label: c.label.clone(),
			hint: c.hint.clone(),
			add_label: c.add_label.clone(),
			generate_label: c.generate_label.clone(),
			generated: c.fields.iter().any(|f| f.generator.is_some()),
			visible: c.visible_in(&self.doc, root).unwrap_or(false),
			editable: c.editable(&self.doc, root),
			removable: c.editable(&self.doc, root) && ids.len() > c.min_items,
			selector,
			rows,
		}
	}

	fn field_view(&self, f: &InputField, row: &Value, path: String, errors: &Errors) -> FieldView {
		FieldView {
			key: f.key.clone(),
			label: f.label.clone(),
			hint: f.hint.clone(),
			placeholder: f.placeholder.clone(),
			kind: match f.kind {
				InputKind::Text => "text",
				InputKind::Password => "password",
				InputKind::Number => "number",
				InputKind::Email => "email",
				InputKind::Toggle => "toggle",
				InputKind::Select => "select",
			},
			options: f.options.clone(),
			value: f.read_value(row),
			visible: f.visible_in(&self.doc, &self.state.data, row).unwrap_or(false),
			error: errors.get(&path).cloned().unwrap_or_default(),
			path,
			generated: f.generator.is_some(),
		}
	}

	fn notices(&self, notices: &[Notice]) -> Vec<NoticeView> {
		notices
			.iter()
			.map(|n| NoticeView {
				text: n.text.clone(),
				visible: self.doc.shown(&n.when, &self.state.data),
			})
			.collect()
	}
}
