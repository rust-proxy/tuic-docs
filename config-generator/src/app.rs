use leptos::prelude::*;
use tuic_config_generator::{
	model::{build_configs, redact_configs, serialize},
	schema::{self, Forward, InputField, InputKind, State, User, input_fields},
	validation::{Errors, endpoint, validate},
};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

fn credentials(id: u64) -> Result<User, String> {
	let error = || "当前浏览器无法安全生成凭据，请使用 HTTPS 或 localhost，或手动填写。".to_owned();
	let crypto = web_sys::window().ok_or_else(error)?.crypto().map_err(|_| error())?;
	let mut bytes = [0u8; 40];
	crypto.get_random_values_with_u8_array(&mut bytes).map_err(|_| error())?;
	let mut uuid_bytes = [0u8; 16];
	uuid_bytes.copy_from_slice(&bytes[..16]);
	uuid_bytes[6] = (uuid_bytes[6] & 0x0f) | 0x40;
	uuid_bytes[8] = (uuid_bytes[8] & 0x3f) | 0x80;
	Ok(User {
		id,
		uuid: uuid::Uuid::from_bytes(uuid_bytes).hyphenated().to_string(),
		password: bytes[16..].iter().map(|b| format!("{b:02x}")).collect(),
	})
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

#[component]
fn Field(state: RwSignal<State>, errors: Memo<Errors>, field: &'static InputField) -> impl IntoView {
	let id = format!("cg-{}", field.key);
	let described = format!("{id}-hint {id}-error");
	let invalid = move || errors.with(|e| e.contains_key(field.key.as_str())).to_string();
	let input = match field.kind {
		InputKind::Toggle => view! {
			<input id=id.clone() type="checkbox" aria-describedby=described aria-invalid=invalid
				prop:checked=move || field.read(&state.get()) == "true"
				on:change=move |ev| { state.update(|s| { field.write(s, if event_target_checked(&ev) { "true" } else { "false" }); }); }/>
		}.into_any(),
		InputKind::Select => view! {
			<select id=id.clone() aria-describedby=described aria-invalid=invalid prop:value=move || field.read(&state.get())
				on:change=move |ev| state.update(|s| { field.write(s, &event_target_value(&ev)); if field.key == "tlsMode" { s.insecure = false; } })>
				{field.options.iter().map(|(value, label)| view! { <option value=value.as_str()>{label.as_str()}</option> }).collect_view()}
			</select>
		}.into_any(),
		_ => {
			let kind = match field.kind { InputKind::Password => "password", InputKind::Email => "email", _ => "text" };
			view! {
				<input id=id.clone() type=kind placeholder=field.placeholder.as_str() autocomplete="off" spellcheck="false"
					inputmode=matches!(field.kind, InputKind::Number).then_some("numeric") aria-describedby=described aria-invalid=invalid
					prop:value=move || field.read(&state.get()) on:input=move |ev| state.update(|s| { field.write(s, &event_target_value(&ev)); })/>
			}.into_any()
		}
	};
	view! {
		<div class=if matches!(field.kind, InputKind::Toggle) { "cg-field cg-toggle" } else { "cg-field" } hidden=move || !field.visible(&state.get())>
			<label for=id.clone()>{field.label.as_str()}</label>
			{input}
			<span id=format!("{id}-hint") class="cg-hint">{field.hint.as_str()}</span>
			<span id=format!("{id}-error") class="cg-error">{move || errors.with(|e| e.get(field.key.as_str()).cloned().unwrap_or_default())}</span>
		</div>
	}
}

#[component]
fn Fields(state: RwSignal<State>, errors: Memo<Errors>, section: &'static str) -> impl IntoView {
	input_fields()
		.iter()
		.filter(|f| f.section == section)
		.map(|field| view! { <Field state errors field/> })
		.collect_view()
}

#[derive(Clone, Copy)]
enum Row {
	User(u64),
	Forward(u64),
}
impl Row {
	fn index(self, s: &State) -> Option<usize> {
		match self {
			Self::User(id) => s.users.iter().position(|u| u.id == id),
			Self::Forward(id) => s.forwards.iter().position(|f| f.id == id),
		}
	}
	fn path(self, s: &State, key: &str) -> String {
		format!(
			"{}.{}.{key}",
			if matches!(self, Self::User(_)) { "users" } else { "forwards" },
			self.index(s).unwrap_or_default()
		)
	}
	fn collection(self) -> &'static str {
		if matches!(self, Self::User(_)) { "users" } else { "forwards" }
	}
	fn value(self, s: &State) -> serde_json::Value {
		self.index(s)
			.map(|i| s.value()[self.collection()][i].clone())
			.unwrap_or_default()
	}
	fn write(self, s: &mut State, field: &InputField, value: &str) {
		let Some(i) = self.index(s) else {
			return;
		};
		let mut data = s.value();
		if field.write_value(&mut data[self.collection()][i], value)
			&& let Ok(next) = serde_json::from_value(data)
		{
			*s = next;
		}
	}
}

#[component]
fn RowField(state: RwSignal<State>, errors: Memo<Errors>, row: Row, field: &'static InputField) -> impl IntoView {
	let key = field.key.as_str();
	let id = move || state.with(|s| format!("cg-{}", row.path(s, key)));
	let error = move || errors.with(|e| state.with(|s| e.get(&row.path(s, key)).cloned().unwrap_or_default()));
	let value = move || state.with(|s| field.read_value(&row.value(s)));
	let input = if field.kind == InputKind::Select {
		view! { <select id=id prop:value=value aria-invalid=move || (!error().is_empty()).to_string()
   aria-describedby=move || format!("{}-error", id()) on:change=move |ev| state.update(|s| row.write(s, field, &event_target_value(&ev)))>
   {field.options.iter().map(|(value, label)| view! { <option value=value.as_str()>{label.as_str()}</option> }).collect_view()}
  </select> }.into_any()
	} else {
		view! { <input id=id prop:value=value type=if field.kind == InputKind::Password { "password" } else { "text" } placeholder=field.placeholder.as_str()
   autocomplete="off" spellcheck="false" inputmode=(field.kind == InputKind::Number).then_some("numeric")
   aria-invalid=move || (!error().is_empty()).to_string() aria-describedby=move || format!("{}-error", id())
   on:input=move |ev| state.update(|s| row.write(s, field, &event_target_value(&ev)))/>
  }.into_any()
	};
	view! {
	 <div class="cg-field" hidden=move || state.with(|s| schema::document().is_ok_and(|d| !field.visible_in(d, &s.value(), &row.value(s)).unwrap_or(false)))>
	  <label for=id>{field.label.as_str()}</label>{input}<span class="cg-error" id=move || format!("{}-error", id())>{error}</span>
	 </div>
	}
}

#[component]
fn UserRow(state: RwSignal<State>, errors: Memo<Errors>, status: RwSignal<String>, id: u64) -> impl IntoView {
	let row = Row::User(id);
	let number = move || state.with(|s| row.index(s).unwrap_or_default() + 1);
	view! {
		<div class="cg-user" hidden=move || state.with(|s| !schema::has_server(s) && row.index(s) != Some(s.active_user))>
			<div class="cg-row-title"><strong>{move || format!("用户 {}", number())}</strong>
				<button type="button" on:click=move |_| match credentials(id) {
					Ok(user) => { state.update(|s| { if let Some(i) = row.index(s) { s.users[i] = user; } }); status.set("已更新用户凭据。".into()); },
					Err(error) => status.set(error),
				}>"生成凭据"</button>
				<button type="button" hidden=move || state.with(|s| !schema::has_server(s) || s.users.len() <= 1)
					aria-label=move || format!("移除用户 {}", number()) on:click=move |_| state.update(|s| s.remove_user(id))>"移除"</button>
			</div>
			{schema::collection_fields("users").iter().map(|field| view! { <RowField state errors row field/> }).collect_view()}
			<span class="cg-hint">"配对配置自动使用相同的 UUID 和密码。"</span>
		</div>
	}
}

#[component]
fn ForwardRow(state: RwSignal<State>, errors: Memo<Errors>, id: u64) -> impl IntoView {
	let row = Row::Forward(id);
	let number = move || state.with(|s| row.index(s).unwrap_or_default() + 1);
	view! {
		<div class="cg-user">
			<div class="cg-row-title"><strong>{move || format!("转发 {}", number())}</strong>
				<button type="button" aria-label=move || format!("移除转发 {}", number()) on:click=move |_| state.update(|s| s.forwards.retain(|f| f.id != id))>"移除"</button>
			</div>
			{schema::collection_fields("forwards").iter().map(|field| view! { <RowField state errors row field/> }).collect_view()}
		</div>
	}
}

#[component]
pub fn App() -> impl IntoView {
	let status = RwSignal::new(String::new());
	let mut initial = match State::initial() {
		Ok(state) => state,
		Err(error) => {
			return view! { <main class="cg-shell"><h1>"配置描述加载失败"</h1><p role="alert">{error}</p></main> }.into_any();
		}
	};
	match credentials(0) {
		Ok(user) => {
			if let Some(first) = initial.users.first_mut() {
				*first = user;
			}
		}
		Err(error) => status.set(error),
	}
	let state = RwSignal::new(initial);
	let next_id = RwSignal::new(1u64);
	let errors = Memo::new(move |_| validate(&state.get()));
	let configs = Memo::new(move |_| build_configs(&state.get()));
	let output_side = RwSignal::new("server".to_owned());
	let side = Memo::new(move |_| state.with(|s| if s.mode == "pair" { output_side.get() } else { s.mode.clone() }));
	let reveal = RwSignal::new(false);
	let dark = RwSignal::new(
		web_sys::window()
			.and_then(|w| w.match_media("(prefers-color-scheme: dark)").ok().flatten())
			.is_some_and(|m| m.matches()),
	);
	let text = Memo::new(move |_| configs.get().and_then(|c| serialize(&c[side.get()], &state.get().format)));
	let preview = move || {
		configs
			.get()
			.and_then(|c| {
				let c = if reveal.get() { c } else { redact_configs(&c)? };
				serialize(&c[side.get()], &state.get().format)
			})
			.unwrap_or_else(|_| "填写左侧配置，预览将在校验通过后显示。".into())
	};
	let filename = move || format!("{}.{}", side.get(), state.get().format);
	let unavailable = move || text.with(Result::is_err);
	let reference = "https://docs.ihsin.dev/tuic/tools/config-generator-reference/";
	view! {
		<div class="cg-shell" data-theme=move || if dark.get() { "dark" } else { "light" }>
			<header class="cg-header"><a class="cg-brand" href="./" aria-label="TUIC 配置生成器首页"><span class="cg-mark">"T"</span>"TUIC"<span class="cg-brand-sub">"配置工具"</span></a>
				<nav><a href=reference target="_blank" rel="noopener noreferrer">"配置说明 ↗"</a>
					<button type="button" class="cg-theme" aria-label="切换主题" aria-pressed=move || dark.get().to_string() on:click=move |_| dark.update(|v| *v = !*v)>{move || if dark.get() { "浅色" } else { "深色" }}</button>
				</nav>
			</header>
			<main id="config-generator" data-ready="true">
				<div class="cg-heading"><p class="cg-eyebrow">{format!("TUIC {} · QUINN", schema::version())}</p><h1>"配置文件生成器"</h1>
					<p>"填写连接信息，生成相互匹配的服务端与客户端配置。"</p>
					<p class="cg-privacy"><span class="cg-dot"></span>"在浏览器本地处理 · 不保存输入 · 不上传凭据"</p>
				</div>
				<div class="cg-modes" role="group" aria-label="生成模式">
					{schema::options("mode").iter().map(|(mode, label)| view! {
						<button type="button" data-mode=mode.as_str() aria-pressed=move || state.with(|s| s.mode == *mode).to_string()
							on:click=move |_| { state.update(|s| s.mode = mode.clone()); output_side.set(if *mode == "client" { "client" } else { "server" }.into()); }>{label.as_str()}</button>
					}).collect_view()}
				</div>
				<div class="cg-workspace">
					<form class="cg-form" autocomplete="off" on:submit=|ev| ev.prevent_default()>
						<section class="cg-section"><h2><span class="cg-step">"01"</span>"连接信息"</h2><Fields state errors section="addresses"/></section>
						<section class="cg-section"><h2><span class="cg-step">"02"</span>"用户认证"</h2>
							<For each=move || state.with(|s| s.users.iter().map(|u| u.id).collect::<Vec<_>>()) key=|id| *id children=move |id| view! { <UserRow state errors status id/> }/>
							<button type="button" hidden=move || !schema::has_server(&state.get()) on:click=move |_| {
								let id = next_id.get_untracked(); next_id.update(|id| *id += 1);
								let user = credentials(id).unwrap_or_else(|error| { status.set(error); User { id, ..User::default() } });
								state.update(|s| s.users.push(user));
							}>"＋ 添加用户"</button>
							<div class="cg-field" hidden=move || state.with(|s| !schema::has_client(s) || s.users.len() < 2)>
								<label for="cg-activeUser">"客户端使用的用户"</label>
								<select id="cg-activeUser" prop:value=move || state.get().active_user.to_string() on:change=move |ev| {
									if let Ok(i) = event_target_value(&ev).parse() { state.update(|s| s.active_user = i); }
								}>{move || state.with(|s| s.users.iter().enumerate().map(|(i, _)| view! { <option value=i.to_string() selected=i == s.active_user>{format!("用户 {}", i + 1)}</option> }).collect_view())}</select>
							</div>
						</section>
						<section class="cg-section"><h2><span class="cg-step">"03"</span>"TLS 与证书"</h2><Fields state errors section="tls"/>
							<p class="cg-callout" hidden=move || !schema::acme(&state.get())>"域名需指向服务器，ACME HTTP-01 验证使用 TCP 80；同时开放 TUIC 的 UDP 监听端口。"</p>
							<p class="cg-callout" hidden=move || state.with(|s| !schema::has_server(s) || s.tls_mode != "self")>"自签名仅用于受控测试。配对测试需明确跳过证书校验。"</p>
						</section>
						<section class="cg-section" hidden=move || !schema::has_client(&state.get())><h2><span class="cg-step">"04"</span>"本地代理"</h2><Fields state errors section="local"/></section>
						<details class="cg-section cg-advanced"><summary>"高级选项"<span>"日志 · 重连 · 端口转发"</span></summary>
							<Fields state errors section="transport"/>
							<div hidden=move || !schema::collection_visible("forwards", &state.get())><h3>"TCP / UDP 端口转发"</h3>
								<For each=move || state.with(|s| s.forwards.iter().map(|f| f.id).collect::<Vec<_>>()) key=|id| *id children=move |id| view! { <ForwardRow state errors id/> }/>
								<button type="button" on:click=move |_| { let id = next_id.get_untracked(); next_id.update(|id| *id += 1); match Forward::initial() { Ok(forward) => state.update(|s| s.forwards.push(Forward { id, ..forward })), Err(error) => status.set(error) } }>"＋ 添加转发"</button>
								<p class="cg-hint">"服务端默认拦截回环和私有目标地址。"</p>
							</div>
							<p class="cg-hint">"客户端 UDP 模式、自定义信任证书和拥塞控制暂未接入运行逻辑，本生成器不输出这些选项。"</p>
						</details>
					</form>
					<aside class="cg-output" aria-label="生成结果">
						<div class="cg-output-top"><strong>"配置预览"</strong><span class="cg-validity" role="status" data-valid=move || (!unavailable()).to_string()>{move || if unavailable() { format!("{} 项待填写或修正", errors.get().len()) } else { "可导出".into() }}</span></div>
						<div class="cg-toolbar"><div class="cg-sides" role="group" aria-label="配置预览类型">
							{[("server", "服务端"), ("client", "客户端")].into_iter().map(|(value, label)| view! {
								<button type="button" hidden=move || state.with(|s| s.mode != "pair" && s.mode != value) aria-pressed=move || (side.get() == value).to_string() on:click=move |_| output_side.set(value.into())>{label}</button>
							}).collect_view()}
						</div><select id="cg-format" aria-label="输出格式" prop:value=move || state.get().format on:change=move |ev| state.update(|s| s.format = event_target_value(&ev))>
							{schema::options("format").iter().map(|(value, label)| view! { <option value=value.as_str()>{label.as_str()}</option> }).collect_view()}
						</select></div>
						<div class="cg-filebar"><strong>{filename}</strong><label for="cg-reveal"><input id="cg-reveal" type="checkbox" prop:checked=move || reveal.get() on:change=move |ev| reveal.set(event_target_checked(&ev))/>" 显示密码"</label></div>
						<div class="cg-errors" hidden=move || errors.get().is_empty()><p>"完成以下字段后即可生成："</p><ul>
							{move || errors.get().into_iter().map(|(key, message)| view! { <li><button type="button" on:click=move |_| focus_error(&key)>{message}</button></li> }).collect_view()}
						</ul></div>
						<pre class="cg-code" tabindex="0" aria-label="配置内容"><code id="cg-preview-code">{preview}</code></pre>
						<div class="cg-actions">
							<button type="button" disabled=unavailable on:click=move |_| {
								if let Ok(text) = text.get_untracked() { leptos::task::spawn_local(async move {
									let result = match web_sys::window() { Some(w) => JsFuture::from(w.navigator().clipboard().write_text(&text)).await, None => Err(JsValue::NULL) };
									status.set(if result.is_ok() { "已复制完整配置（含明文凭据）。" } else { "浏览器未允许复制，请使用下载配置。" }.into());
								}); }
							}>"复制配置"</button>
							<button type="button" class="cg-primary" disabled=unavailable on:click=move |_| {
								if let Ok(text) = text.get_untracked() { let filename = filename(); status.set(if download(&text, &filename).is_ok() { format!("已下载 {filename}（含明文凭据）。") } else { "下载失败，请尝试复制配置。".into() }); }
							}>"下载配置"</button>
						</div>
						<p class="cg-hint">"复制和下载始终包含真实密码。配对模式请分别保存两份配置。"</p>
						<code class="cg-command">{move || format!("tuic-{} -c {}", side.get(), filename())}</code>
						<p class="cg-callout" hidden=move || state.with(|s| !(s.insecure && schema::has_client(s)))>"当前客户端配置跳过证书校验，仅适合受控测试。"</p>
						<p class="cg-callout" hidden=move || state.with(|s| !schema::has_client(s) || s.local_auth || endpoint(&s.local, true).is_none_or(|(h, _)| ["127.0.0.1", "::1"].contains(&h.as_str())))>"SOCKS5 正在监听非回环地址且未启用认证，请确认访问范围。"</p>
						<p class="cg-callout" hidden=move || !state.get().zero_rtt>"0-RTT 早期数据可能被重放。"</p>
						<p class="cg-hint">"此处仅检查配置字段。证书文件、DNS、防火墙和实际连接需要在部署环境验证。"</p>
					</aside>
				</div>
				<p class="cg-status" role="status" aria-live="polite">{move || status.get()}</p>
				<footer>"TUIC 配置工具"<span>"本地生成，按需导出。"</span></footer>
			</main>
		</div>
	}.into_any()
}
