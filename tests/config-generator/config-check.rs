//! Real parser checks and a loopback-only SOCKS5 -> QUIC -> TCP echo smoke
//! test. Built in .cache by check-rust.py; the TUIC checkout is read-only.
use std::{
	net::{IpAddr, Ipv4Addr, SocketAddr},
	path::{Path, PathBuf},
	time::Duration,
};

use eyre::{Result, ensure};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

async fn server_config(path: PathBuf) -> Result<tuic_server::config::Config> {
	tuic_server::config::parse_config(
		tuic_server::config::Cli {
			config: Some(path),
			dir: None,
			init: false,
		},
		tuic_server::config::EnvState::default(),
	)
	.await
}
fn client_config(path: PathBuf) -> Result<tuic_client::Config> {
	tuic_client::Config::parse(
		tuic_client::config::Cli { config: Some(path) },
		tuic_client::config::EnvState::default(),
	)
}

#[tokio::main]
async fn main() -> Result<()> {
	let dir = PathBuf::from(
		std::env::args()
			.nth(1)
			.ok_or_else(|| eyre::eyre!("fixture directory required"))?,
	);
	let mut count = 0;
	for item in std::fs::read_dir(&dir)? {
		let path = item?.path();
		let name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default();
		if name.contains("-server.") {
			let _ = server_config(path).await?;
			count += 1;
		} else if name.contains("-client.") {
			let _ = client_config(path)?;
			count += 1;
		}
	}
	ensure!(count == 72, "expected 72 parser fixtures");
	println!("Real TUIC parsers accepted {count} generated configurations");
	tokio::time::timeout(Duration::from_secs(30), smoke(&dir)).await??;
	Ok(())
}

async fn smoke(dir: &Path) -> Result<()> {
	let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
	let mut cfg = server_config(dir.join("bbr-self-server.toml")).await?;
	cfg.server = SocketAddr::from((Ipv4Addr::LOCALHOST, 0));
	// Only the in-memory test configuration allows the loopback echo target.
	cfg.experimental.drop_loopback = false;
	cfg.experimental.drop_private = false;
	let server = tuic_server::run(cfg).await?;
	let mut cfg = client_config(dir.join("bbr-self-client.toml"))?;
	cfg.relay.server = ("localhost".to_owned(), server.local_addr.port());
	cfg.relay.ip = Some(IpAddr::V4(Ipv4Addr::LOCALHOST));
	cfg.local.server = SocketAddr::from((Ipv4Addr::LOCALHOST, 0));
	let client = tuic_client::run(cfg).await?;

	let echo = tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await?;
	let target = echo.local_addr()?;
	let echo_task = tokio::spawn(async move {
		let (mut conn, _) = echo.accept().await?;
		let mut data = [0u8; 16];
		conn.read_exact(&mut data).await?;
		conn.write_all(&data).await?;
		Ok::<_, std::io::Error>(())
	});
	let mut socks = tokio::net::TcpStream::connect(client.socks5_addr).await?;
	socks.write_all(&[5, 1, 0]).await?;
	let mut greeting = [0u8; 2];
	socks.read_exact(&mut greeting).await?;
	ensure!(greeting == [5, 0], "SOCKS5 greeting failed");
	let mut request = vec![5, 1, 0, 1, 127, 0, 0, 1];
	request.extend_from_slice(&target.port().to_be_bytes());
	socks.write_all(&request).await?;
	let mut header = [0u8; 4];
	socks.read_exact(&mut header).await?;
	ensure!(header[0] == 5 && header[1] == 0, "SOCKS5 connect failed");
	let size = match header[3] {
		1 => 6,
		4 => 18,
		_ => return Err(eyre::eyre!("unexpected SOCKS5 address")),
	};
	socks.read_exact(&mut vec![0u8; size]).await?;
	let message = *b"tuic-docs-smoke!";
	socks.write_all(&message).await?;
	let mut response = [0u8; 16];
	socks.read_exact(&mut response).await?;
	ensure!(response == message, "echo payload mismatch");
	echo_task.await??;
	client.shutdown().await;
	server.shutdown().await;
	println!("Loopback SOCKS5 / TUIC / TCP echo passed");
	Ok(())
}
