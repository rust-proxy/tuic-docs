use config_generator::{
	dsl::{Collection, Document, InputField, InputKind, Notice, Section},
	model::{build_configs, serialize},
	schema::{self, State},
	validation::Errors,
};
use leptos::prelude::*;
use serde_json::Value;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

fn generate(fields: &[InputField], row: &mut Value) -> Result<(), String> {
	if !fields.iter().any(|f| f.generator.is_some()) {
		return Ok(());
	}
	let error = || "当前浏览器无法安全生成随机值，请使用 HTTPS 或 localhost，或手动填写。".to_owned();
	let crypto = web_sys::window().ok_or_else(error)?.crypto().map_err(|_| error())?;
	let mut next = row.clone();
	for field in fields {
		if let Some(generator) = &field.generator {
			let mut bytes = vec![0; generator.byte_count()];
			crypto.get_random_values_with_u8_array(&mut bytes).map_err(|_| error())?;
			next[&field.key] = generator.encode(&bytes).map_err(|e| e.to_string())?.into();
		}
	}
	*row = next;
	Ok(())
}

fn download(text: &str, filename: &str) -> Result<(), JsValue> {
	let window = web_sys::window().ok_or_else(|| JsValue::from_str("No window"))?;
	let document = window.document().ok_or_else(|| JsValue::from_str("No document"))?;
	let parts = js_sys::Array::new();
	parts.push(&JsValue::from_str(text));
	let options = web_sys::BlobPropertyBag::new();
	options.set_type("text/plain;charset=utf-8");
	let blob = web_sys::Blob::new_with_str_sequence_and_options(&parts, &options)?;
	let url = web_sys::Url::create_object_url_with_blob(&blob)?;
	let result = (|| {
		let anchor = document.create_element("a")?.dyn_into::<web_sys::HtmlAnchorElement>()?;
		anchor.set_href(&url);
		anchor.set_download(filename);
		let body = document.body().ok_or_else(|| JsValue::from_str("No body"))?;
		body.append_child(&anchor)?;
		anchor.click();
		anchor.remove();
		Ok(())
	})();
	// Let the browser consume the URL before releasing its blob.
	let cleanup = wasm_bindgen::closure::Closure::once_into_js(move || {
		let _ = web_sys::Url::revoke_object_url(&url);
	});
	window.set_timeout_with_callback_and_timeout_and_arguments_0(cleanup.unchecked_ref(), 1000)?;
	result
}

fn focus_error(key: &str) {
	if let Some(document) = web_sys::window().and_then(|w| w.document())
		&& let Some(element) = document.get_element_by_id(&format!("cg-{key}"))
	{
		if let Ok(Some(details)) = element.closest("details")
			&& let Ok(details) = details.dyn_into::<web_sys::HtmlDetailsElement>()
		{
			details.set_open(true);
		}
		if let Ok(element) = element.dyn_into::<web_sys::HtmlElement>() {
			let _ = element.focus();
		}
	}
}

#[derive(Clone, Copy)]
struct Binding {
	collection: Option<&'static Collection>,
	id: u64,
}
impl Binding {
	fn index(self, s: &State) -> Option<usize> {
		self.collection.and_then(|c| s.index(&c.name, self.id))
	}
	fn path(self, s: &State, key: &str) -> String {
		match self.collection {
			Some(c) => format!("{}.{}.{key}", c.name, self.index(s).unwrap_or_default()),
			None => key.into(),
		}
	}
	fn row(self, s: &State) -> &Value {
		self.collection.and_then(|c| s.row(&c.name, self.id)).unwrap_or(&s.data)
	}
	fn write(self, s: &mut State, doc: &Document, field: &InputField, value: &str) {
		if let Some(c) = self.collection {
			if let Some(row) = s.row_mut(&c.name, self.id) {
				field.write_value(row, value);
			}
		} else {
			doc.write_field(&mut s.data, &field.key, value);
		}
	}
}
#[component]
fn Field(
	state: RwSignal<State>,
	errors: Memo<Errors>,
	doc: &'static Document,
	binding: Binding,
	field: &'static InputField,
) -> impl IntoView {
	let key = field.key.as_str();
	let id = move || state.with(|s| format!("cg-{}", binding.path(s, key)));
	let error = move || errors.with(|e| state.with(|s| e.get(&binding.path(s, key)).cloned().unwrap_or_default()));
	let value = move || state.with(|s| field.read_value(binding.row(s)));
	let described = move || format!("{}-hint {}-error", id(), id());
	let invalid = move || (!error().is_empty()).to_string();
	let input=match field.kind {
  InputKind::Toggle=>view! { <input id=id type="checkbox" aria-describedby=described aria-invalid=invalid
   prop:checked=move || value()=="true" on:change=move |ev| state.update(|s| binding.write(s,doc,field,if event_target_checked(&ev) {"true"} else {"false"}))/>
  }.into_any(),
  InputKind::Select=>view! { <select id=id aria-describedby=described aria-invalid=invalid prop:value=value
   on:change=move |ev| state.update(|s| binding.write(s,doc,field,&event_target_value(&ev)))>
   {field.options.iter().map(|(value,label)| view! { <option value=value.as_str()>{label.as_str()}</option> }).collect_view()}
  </select> }.into_any(),
  _=>view! { <input id=id type=match field.kind {InputKind::Password=>"password",InputKind::Email=>"email",_=>"text"}
   placeholder=field.placeholder.as_str() autocomplete="off" spellcheck="false" inputmode=(field.kind==InputKind::Number).then_some("numeric")
   aria-describedby=described aria-invalid=invalid prop:value=value
   on:input=move |ev| state.update(|s| binding.write(s,doc,field,&event_target_value(&ev)))/>
  }.into_any(),
 };
	let generated = RwSignal::new(String::new());
	view! {
	 <div class=if field.kind==InputKind::Toggle {"cg-field cg-toggle"} else {"cg-field"}
	  hidden=move || state.with(|s| !field.visible_in(doc,&s.data,binding.row(s)).unwrap_or(false))>
	  <label for=id>{field.label.as_str()}</label>{input}
	  <span id=move || format!("{}-hint",id()) class="cg-hint">{field.hint.as_str()}</span>
	  <span id=move || format!("{}-error",id()) class="cg-error">{error}</span>
	  {if binding.collection.is_none() && field.generator.is_some() { Some(view! {
	   <button type="button" on:click=move |_| state.update(|s| { let mut candidate = s.data.clone();
	  match generate(std::slice::from_ref(field), &mut candidate) {
	   Ok(()) => { doc.write_field(&mut s.data, &field.key, &field.read_value(&candidate)); generated.set(String::new()); },
	   Err(error) => generated.set(error),
	  } })>"生成随机值"</button>
	   <span role="status">{move || generated.get()}</span>
	  }) } else {None}}
	 </div>
	}
}
#[component]
fn Notices(state: RwSignal<State>, doc: &'static Document, notices: &'static [Notice]) -> impl IntoView {
	notices
		.iter()
		.map(
			|n| view! { <p class="cg-callout" hidden=move || state.with(|s| !doc.shown(&n.when,&s.data))>{n.text.as_str()}</p> },
		)
		.collect_view()
}
#[component]
fn CollectionRow(
	state: RwSignal<State>,
	errors: Memo<Errors>,
	status: RwSignal<String>,
	doc: &'static Document,
	collection: &'static Collection,
	id: u64,
) -> impl IntoView {
	let binding = Binding {
		collection: Some(collection),
		id,
	};
	let number = move || state.with(|s| binding.index(s).unwrap_or_default() + 1);
	view! {
	 <div class="cg-user" hidden=move || state.with(|s| !collection.row_visible(doc,&s.data,binding.index(s).unwrap_or_default()).unwrap_or(false))>
	  <div class="cg-row-title"><strong>{move || format!("{} {}",collection.label,number())}</strong>
	   <button type="button" hidden=!collection.fields.iter().any(|f| f.generator.is_some()) on:click=move |_| state.update(|s| {
		if let Some(row)=s.row_mut(&collection.name,id) { status.set(match generate(&collection.fields,row) {Ok(())=>"已更新随机值。".into(),Err(e)=>e}); }
	   })>{collection.generate_label.as_str()}</button>
	   <button type="button" hidden=move || state.with(|s| !collection.editable(doc,&s.data) || s.rows(&collection.name).len()<=collection.min_items)
		aria-label=move || format!("移除{} {}",collection.label,number()) on:click=move |_| state.update(|s| {s.remove(doc,&collection.name,id);})>"移除"</button>
	  </div>
	  {collection.fields.iter().map(|field| view! { <Field state errors doc binding field/> }).collect_view()}
	 </div>
	}
}
#[component]
fn CollectionEditor(
	state: RwSignal<State>,
	errors: Memo<Errors>,
	status: RwSignal<String>,
	doc: &'static Document,
	collection: &'static Collection,
) -> impl IntoView {
	view! {
	 <div hidden=move || state.with(|s| !collection.visible_in(doc,&s.data).unwrap_or(false))>
	  <h3>{collection.label.as_str()}</h3>
	  <For each=move || state.with(|s| s.rows(&collection.name)) key=|id| *id children=move |id| view! { <CollectionRow state errors status doc collection id/> }/>
	  <button type="button" hidden=move || state.with(|s| !collection.editable(doc,&s.data)) on:click=move |_| state.update(|s| {
	   match s.add(doc,&collection.name) {
		Ok(id)=> {if let Some(row)=s.row_mut(&collection.name,id) && let Err(e)=generate(&collection.fields,row) {status.set(e);}},
		Err(e)=>status.set(e.to_string()),
	   }
	  })>{collection.add_label.as_str()}</button>
	  {collection.selected_by.as_ref().map(|key| {
	   let label=doc.fields.iter().find(|f| &f.key==key).map(|f| f.label.as_str()).unwrap_or(key.as_str());
	   view! { <div class="cg-field" hidden=move || state.with(|s| !doc.shown(&collection.select_when,&s.data) || s.rows(&collection.name).len()<2)>
		<label for=format!("cg-{key}")>{label}</label>
		<select id=format!("cg-{key}") prop:value=move || state.with(|s| s.data[key].to_string())
		 on:change=move |ev| state.update(|s| {doc.write_field(&mut s.data,key,&event_target_value(&ev));})>
		 {move || state.with(|s| s.rows(&collection.name).iter().enumerate().map(|(i,_)| view! {
		  <option value=i.to_string() selected=s.data[key].as_u64()==Some(i as u64)>{format!("{} {}",collection.label,i+1)}</option>
		 }).collect_view())}
		</select>
	   </div> }
	  })}
	  <p class="cg-hint">{collection.hint.as_str()}</p>
	 </div>
	}
}
#[component]
fn FormSection(
	state: RwSignal<State>,
	errors: Memo<Errors>,
	status: RwSignal<String>,
	doc: &'static Document,
	section: &'static Section,
	number: usize,
) -> impl IntoView {
	let contents = view! {
	 {doc.fields.iter().filter(|f| f.section==section.name && doc.ui.mode_field.as_ref()!=Some(&f.key) && doc.ui.format_field.as_ref()!=Some(&f.key) && !doc.collections.iter().any(|c| c.selected_by.as_ref()==Some(&f.key))).map(|field| view! { <Field state errors doc binding=Binding{collection:None,id:0} field/> }).collect_view()}
	 {doc.collections.iter().filter(|c| c.section==section.name).map(|collection| view! { <CollectionEditor state errors status doc collection/> }).collect_view()}
	 <Notices state doc notices=&section.notices/>
	};
	if section.collapsed {
		view! { <details class="cg-section cg-advanced" hidden=move || state.with(|s| !doc.shown(&section.when,&s.data))><summary>{section.label.as_str()}<span>{section.detail.as_str()}</span></summary>{contents}</details> }.into_any()
	} else {
		view! { <section class="cg-section" hidden=move || state.with(|s| !doc.shown(&section.when,&s.data))><h2><span class="cg-step">{format!("{:02}",number+1)}</span>{section.label.as_str()}</h2>{contents}</section> }.into_any()
	}
}
#[component]
pub fn App() -> impl IntoView {
	let doc = match schema::document() {
		Ok(doc) => doc,
		Err(e) => {
			return view! { <main class="cg-shell"><h1>"配置描述加载失败"</h1><p role="alert">{e.to_string()}</p></main> }
				.into_any();
		}
	};
	let status = RwSignal::new(String::new());
	let mut initial = State::new(doc);
	if let Err(e) = generate(&doc.fields, &mut initial.data) {
		status.set(e);
	}
	for c in &doc.collections {
		for id in initial.rows(&c.name) {
			if let Some(row) = initial.row_mut(&c.name, id)
				&& let Err(e) = generate(&c.fields, row)
			{
				status.set(e);
			}
		}
	}
	if let Some(document) = web_sys::window().and_then(|w| w.document()) {
		document.set_title(&format!("{} {}", doc.ui.brand, doc.ui.title));
	}
	let state = RwSignal::new(initial);
	let errors = Memo::new(move |_| state.with(|s| doc.validate(&s.data)));
	let configs = Memo::new(move |_| state.with(|s| build_configs(doc, &s.data)));
	let output = RwSignal::new(String::new());
	let side = Memo::new(move |_| {
		state.with(|s| {
			doc.exports
				.iter()
				.find(|e| e.name == output.get() && doc.shown(&e.when, &s.data))
				.or_else(|| doc.exports.iter().find(|e| doc.shown(&e.when, &s.data)))
				.map(|e| e.name.clone())
				.unwrap_or_default()
		})
	});
	let format = move || {
		state.with(|s| {
			doc.ui
				.format_field
				.as_ref()
				.and_then(|k| s.data[k].as_str())
				.unwrap_or("json")
				.to_owned()
		})
	};
	let reveal = RwSignal::new(false);
	let dark = RwSignal::new(
		web_sys::window()
			.and_then(|w| w.match_media("(prefers-color-scheme: dark)").ok().flatten())
			.is_some_and(|m| m.matches()),
	);
	let text = Memo::new(move |_| {
		configs.get().and_then(|c| {
			c.get(side.get())
				.ok_or_else(|| "请选择输出。".into())
				.and_then(|v| serialize(v, &format()))
		})
	});
	let preview = move || {
		configs
			.get()
			.and_then(|c| {
				let c = if reveal.get() {
					c
				} else {
					doc.redact(&c).map_err(|e| e.to_string())?
				};
				c.get(side.get())
					.ok_or_else(|| "请选择输出。".into())
					.and_then(|v| serialize(v, &format()))
			})
			.unwrap_or_else(|_| "填写左侧配置，预览将在校验通过后显示。".into())
	};
	let filename = move || {
		doc.exports
			.iter()
			.find(|e| e.name == side.get())
			.map(|e| e.filename(&format()))
			.unwrap_or_default()
	};
	let command = move || {
		doc.exports
			.iter()
			.find(|e| e.name == side.get())
			.map(|e| e.command(&format()))
			.unwrap_or_default()
	};
	let unavailable = move || text.with(Result::is_err);
	view! {
  <div class="cg-shell" data-theme=move || if dark.get() {"dark"} else {"light"}>
   <header class="cg-header"><a class="cg-brand" href="./" aria-label=format!("{}首页",doc.ui.title)><span class="cg-mark">{doc.ui.mark.as_str()}</span>{doc.ui.brand.as_str()}<span class="cg-brand-sub">"配置工具"</span></a>
    <nav><a href=doc.ui.reference.as_str() hidden=doc.ui.reference.is_empty() target="_blank" rel="noopener noreferrer">"配置说明 ↗"</a>
     <button type="button" class="cg-theme" aria-label="切换主题" aria-pressed=move || dark.get().to_string() on:click=move |_| dark.update(|v| *v = !*v)>{move || if dark.get() {"浅色"} else {"深色"}}</button>
    </nav>
   </header>
   <main id="config-generator" data-ready="true">
    <div class="cg-heading"><p class="cg-eyebrow">{doc.ui.eyebrow.as_str()}</p><h1>{doc.ui.title.as_str()}</h1><p>{doc.ui.description.as_str()}</p>
     <p class="cg-privacy"><span class="cg-dot"></span>"在浏览器本地处理 · 不保存输入 · 不上传凭据"</p>
    </div>
    {doc.ui.mode_field.as_ref().and_then(|key| doc.fields.iter().find(|f| &f.key==key)).map(|field| view! {
     <div class="cg-modes" role="group" aria-label=field.label.as_str()>
      {field.options.iter().map(|(mode,label)| view! { <button type="button" data-mode=mode.as_str() aria-pressed=move || state.with(|s| field.read_value(&s.data)==*mode).to_string()
       on:click=move |_| state.update(|s| {doc.write_field(&mut s.data,&field.key,mode);})>{label.as_str()}</button> }).collect_view()}
     </div>
    })}
    <div class="cg-workspace"><form class="cg-form" autocomplete="off" on:submit=|ev| ev.prevent_default()>
     {doc.ui.sections.iter().enumerate().map(|(number,section)| view! { <FormSection state errors status doc section number/> }).collect_view()}
    </form>
    <aside class="cg-output" aria-label="生成结果">
     <div class="cg-output-top"><strong>"配置预览"</strong><span class="cg-validity" role="status" data-valid=move || (!unavailable()).to_string()>{move || if unavailable() {format!("{} 项待填写或修正",errors.get().len())} else {"可导出".into()}}</span></div>
     <div class="cg-toolbar"><div class="cg-sides" role="group" aria-label="配置预览类型">
      {doc.exports.iter().map(|export| view! { <button type="button" hidden=move || state.with(|s| !doc.shown(&export.when,&s.data)) aria-pressed=move || (side.get()==export.name).to_string() on:click=move |_| output.set(export.name.clone())>{export.label.as_str()}</button> }).collect_view()}
     </div>
     {doc.ui.format_field.as_ref().and_then(|key| doc.fields.iter().find(|f| &f.key==key)).map(|field| view! {
      <select id=format!("cg-{}",field.key) aria-label=field.label.as_str() prop:value=format on:change=move |ev| state.update(|s| {doc.write_field(&mut s.data,&field.key,&event_target_value(&ev));})>
       {field.options.iter().map(|(value,label)| view! { <option value=value.as_str()>{label.as_str()}</option> }).collect_view()}
      </select>
     })}</div>
     <div class="cg-filebar"><strong>{filename}</strong><label for="cg-reveal"><input id="cg-reveal" type="checkbox" prop:checked=move || reveal.get() on:change=move |ev| reveal.set(event_target_checked(&ev))/>" 显示密码"</label></div>
     <div class="cg-errors" hidden=move || errors.get().is_empty()><p>"完成以下字段后即可生成："</p><ul>
      {move || errors.get().into_iter().map(|(key,message)| view! { <li><button type="button" on:click=move |_| focus_error(&key)>{message}</button></li> }).collect_view()}
     </ul></div>
     <pre class="cg-code" tabindex="0" aria-label="配置内容"><code id="cg-preview-code">{preview}</code></pre>
     <div class="cg-actions"><button type="button" disabled=unavailable on:click=move |_| {
      if let Ok(text)=text.get_untracked() {leptos::task::spawn_local(async move {
       let result=match web_sys::window() {Some(w)=>JsFuture::from(w.navigator().clipboard().write_text(&text)).await,None=>Err(JsValue::NULL)};
       status.set(if result.is_ok() {"已复制完整配置（含明文凭据）。"} else {"浏览器未允许复制，请使用下载配置。"}.into());
      });}
     }>"复制配置"</button>
     <button type="button" class="cg-primary" disabled=unavailable on:click=move |_| {
      if let Ok(text)=text.get_untracked() {let filename=filename();status.set(if download(&text,&filename).is_ok() {format!("已下载 {filename}（含明文凭据）。")} else {"下载失败，请尝试复制配置。".into()});}
     }>"下载配置"</button></div>
     <p class="cg-hint">{doc.ui.export_hint.as_str()}</p><code class="cg-command">{command}</code>
     <Notices state doc notices=&doc.ui.notices/>
    </aside></div>
    <p class="cg-status" role="status" aria-live="polite">{move || status.get()}</p>
    <footer>{doc.ui.brand.as_str()}" 配置工具"<span>"本地生成，按需导出。"</span></footer>
   </main>
  </div>
 }.into_any()
}
