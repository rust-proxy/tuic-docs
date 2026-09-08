//! Ephemeral parser fixtures and independent format round trips. Never commit outputs.
use std::{error::Error, fs, path::PathBuf};

use serde_json::json;
use tuic_config_generator::{
	model::{build_configs, serialize},
	schema::{Forward, State, User, options},
};

fn user(id: u64) -> User {
	User {
		id,
		uuid: uuid::Uuid::new_v4().to_string(),
		password: uuid::Uuid::new_v4().simple().to_string(),
	}
}

fn main() -> Result<(), Box<dyn Error>> {
	let args: Vec<_> = std::env::args().collect();
	if args.get(1).is_some_and(|v| v == "--compare") {
		let cases: serde_json::Value = serde_json::from_slice(&fs::read(args.get(2).ok_or("comparison file required")?)?)?;
		let cases = cases.as_array().ok_or("expected comparison array")?;
		for (i, case) in cases.iter().enumerate() {
			let state: State = serde_json::from_value(case["state"].clone())?;
			if build_configs(&state)? != case["expected"] {
				return Err(format!("Migration mismatch in case {i}").into());
			}
		}
		println!("{} migration cases matched", cases.len());
		return Ok(());
	}
	let dir = PathBuf::from(args.get(1).ok_or("fixture directory required")?);
	fs::create_dir_all(&dir)?;
	fs::write(dir.join("certificate.fixture"), "parser fixture")?;
	fs::write(dir.join("key.fixture"), "parser fixture")?;
	let mut cases = vec![];
	let mut count = 0;
	for (controller, _) in options("controller") {
		for tls_mode in ["certificate", "acme", "self"] {
			let mut s = State {
				host: "localhost".into(),
				hostname: "tuic.example.com".into(),
				tls_mode: tls_mode.into(),
				controller: controller.clone(),
				insecure: tls_mode == "self",
				email: "admin@example.com".into(),
				certificate: dir.join("certificate.fixture").to_string_lossy().into(),
				private_key: dir.join("key.fixture").to_string_lossy().into(),
				data_dir: dir.join("acme-cache").to_string_lossy().into(),
				users: vec![user(0), user(1)],
				active_user: 1,
				..State::initial()?
			};
			if *controller != "bbr" {
				s.host = "[2001:db8::1]".into();
				s.users[1].password.push_str(
					" quotes \" \\f \u{c}\n\r\t\u{8}\0\u{7f}\u{85}\u{2028}\u{2029} 中文 😀 # ]\n[users]\n- yes: null",
				);
				s.local_auth = true;
				s.local_username = "generator-test".into();
				s.local_password = "local \" \\ 中文".into();
				s.zero_rtt = true;
				s.forwards = vec![
					Forward {
						protocol: "tcp".into(),
						listen: "127.0.0.1:8080".into(),
						remote: "example.com:80".into(),
						..Forward::initial()?
					},
					Forward {
						id: 1,
						protocol: "udp".into(),
						listen: "[::1]:8053".into(),
						remote: "[2001:db8::53]:53".into(),
						timeout: "45".into(),
					},
				];
			}
			let configs = build_configs(&s)?;
			for side in ["server", "client"] {
				let config = &configs[side];
				let mut formats = serde_json::Map::new();
				for (format, _) in options("format") {
					let text = serialize(config, format)?;
					fs::write(dir.join(format!("{controller}-{tls_mode}-{side}.{format}")), &text)?;
					formats.insert(format.clone(), text.into());
					count += 1;
				}
				cases
					.push(json!({ "name": format!("{controller}/{tls_mode}/{side}"), "expected": config, "formats": formats }));
			}
		}
	}
	fs::write(dir.join("roundtrip.json"), serde_json::to_vec(&cases)?)?;
	println!("Generated {count} ephemeral parser fixtures");
	Ok(())
}
