#![allow(dead_code)]

use rustls::{server::AllowAnyAuthenticatedClient, ServerConfig, ServerConnection};
use std::{
    convert::{TryFrom, TryInto},
    io,
    sync::Arc,
};
use tls_client::{
    internal::msgs::{
        codec::Reader,
        message::{Message, OpaqueMessage, PlainMessage},
    },
    Certificate, ClientConfig, ClientConnection, Error, PrivateKey, RootCertStore,
    RustCryptoBackend,
};

macro_rules! embed_files {
    (
        $(
            ($name:ident, $keytype:expr, $path:expr);
        )+
    ) => {
        $(
            const $name: &'static [u8] = include_bytes!(
                concat!("../../test-ca/", $keytype, "/", $path));
        )+

        pub fn bytes_for(keytype: &str, path: &str) -> &'static [u8] {
            match (keytype, path) {
                $(
                    ($keytype, $path) => $name,
                )+
                _ => panic!("unknown keytype {} with path {}", keytype, path),
            }
        }
    }
}

embed_files! {
    (ECDSA_CA_CERT, "ecdsa", "ca.cert");
    (ECDSA_CA_DER, "ecdsa", "ca.der");
    (ECDSA_CA_KEY, "ecdsa", "ca.key");
    (ECDSA_CLIENT_CERT, "ecdsa", "client.cert");
    (ECDSA_CLIENT_CHAIN, "ecdsa", "client.chain");
    (ECDSA_CLIENT_FULLCHAIN, "ecdsa", "client.fullchain");
    (ECDSA_CLIENT_KEY, "ecdsa", "client.key");
    (ECDSA_CLIENT_REQ, "ecdsa", "client.req");
    (ECDSA_END_CERT, "ecdsa", "end.cert");
    (ECDSA_END_CHAIN, "ecdsa", "end.chain");
    (ECDSA_END_FULLCHAIN, "ecdsa", "end.fullchain");
    (ECDSA_END_KEY, "ecdsa", "end.key");
    (ECDSA_END_REQ, "ecdsa", "end.req");
    (ECDSA_INTER_CERT, "ecdsa", "inter.cert");
    (ECDSA_INTER_KEY, "ecdsa", "inter.key");
    (ECDSA_INTER_REQ, "ecdsa", "inter.req");
    (ECDSA_NISTP256_PEM, "ecdsa", "nistp256.pem");
    (ECDSA_NISTP384_PEM, "ecdsa", "nistp384.pem");

    (EDDSA_CA_CERT, "eddsa", "ca.cert");
    (EDDSA_CA_DER, "eddsa", "ca.der");
    (EDDSA_CA_KEY, "eddsa", "ca.key");
    (EDDSA_CLIENT_CERT, "eddsa", "client.cert");
    (EDDSA_CLIENT_CHAIN, "eddsa", "client.chain");
    (EDDSA_CLIENT_FULLCHAIN, "eddsa", "client.fullchain");
    (EDDSA_CLIENT_KEY, "eddsa", "client.key");
    (EDDSA_CLIENT_REQ, "eddsa", "client.req");
    (EDDSA_END_CERT, "eddsa", "end.cert");
    (EDDSA_END_CHAIN, "eddsa", "end.chain");
    (EDDSA_END_FULLCHAIN, "eddsa", "end.fullchain");
    (EDDSA_END_KEY, "eddsa", "end.key");
    (EDDSA_END_REQ, "eddsa", "end.req");
    (EDDSA_INTER_CERT, "eddsa", "inter.cert");
    (EDDSA_INTER_KEY, "eddsa", "inter.key");
    (EDDSA_INTER_REQ, "eddsa", "inter.req");

    (RSA_CA_CERT, "rsa", "ca.cert");
    (RSA_CA_DER, "rsa", "ca.der");
    (RSA_CA_KEY, "rsa", "ca.key");
    (RSA_CLIENT_CERT, "rsa", "client.cert");
    (RSA_CLIENT_CHAIN, "rsa", "client.chain");
    (RSA_CLIENT_FULLCHAIN, "rsa", "client.fullchain");
    (RSA_CLIENT_KEY, "rsa", "client.key");
    (RSA_CLIENT_REQ, "rsa", "client.req");
    (RSA_CLIENT_RSA, "rsa", "client.rsa");
    (RSA_END_CERT, "rsa", "end.cert");
    (RSA_END_CHAIN, "rsa", "end.chain");
    (RSA_END_FULLCHAIN, "rsa", "end.fullchain");
    (RSA_END_KEY, "rsa", "end.key");
    (RSA_END_REQ, "rsa", "end.req");
    (RSA_END_RSA, "rsa", "end.rsa");
    (RSA_INTER_CERT, "rsa", "inter.cert");
    (RSA_INTER_KEY, "rsa", "inter.key");
    (RSA_INTER_REQ, "rsa", "inter.req");
}

pub fn version_eq(left: tls_client::ProtocolVersion, right: rustls::ProtocolVersion) -> bool {
    left.as_str() == right.as_str()
}

pub fn send(client: &mut ClientConnection, server: &mut ServerConnection) -> usize {
    let mut buf = [0u8; 262144];
    let mut total = 0;

    while client.wants_write() {
        let sz = {
            let into_buf: &mut dyn io::Write = &mut &mut buf[..];
            client.write_tls(into_buf).unwrap()
        };
        total += sz;
        if sz == 0 {
            return total;
        }

        let mut offs = 0;
        loop {
            let from_buf: &mut dyn io::Read = &mut &buf[offs..sz];
            offs += server.read_tls(from_buf).unwrap();
            if sz == offs {
                break;
            }
        }
    }

    total
}

pub fn receive(server: &mut ServerConnection, client: &mut ClientConnection) -> usize {
    let mut buf = [0u8; 262144];
    let mut total = 0;

    while server.wants_write() {
        let sz = {
            let into_buf: &mut dyn io::Write = &mut &mut buf[..];
            server.write_tls(into_buf).unwrap()
        };
        total += sz;
        if sz == 0 {
            return total;
        }

        let mut offs = 0;
        loop {
            let from_buf: &mut dyn io::Read = &mut &buf[offs..sz];
            offs += client.read_tls(from_buf).unwrap();
            if sz == offs {
                break;
            }
        }
    }

    total
}

pub fn transfer_eof(conn: &mut ClientConnection) {
    let empty_buf = [0u8; 0];
    let empty_cursor: &mut dyn io::Read = &mut &empty_buf[..];
    let sz = conn.read_tls(empty_cursor).unwrap();
    assert_eq!(sz, 0);
}

pub fn transfer_eof_rustls(conn: &mut ServerConnection) {
    let empty_buf = [0u8; 0];
    let empty_cursor: &mut dyn io::Read = &mut &empty_buf[..];
    let sz = conn.read_tls(empty_cursor).unwrap();
    assert_eq!(sz, 0);
}

pub enum Altered {
    /// message has been edited in-place (or is unchanged)
    InPlace,
    /// send these raw bytes instead of the message.
    Raw(Vec<u8>),
}

pub fn send_altered<F>(
    client: &mut ClientConnection,
    filter: F,
    server: &mut rustls::Connection,
) -> usize
where
    F: Fn(&mut Message) -> Altered,
{
    let mut buf = [0u8; 262144];
    let mut total = 0;

    while client.wants_write() {
        let sz = {
            let into_buf: &mut dyn io::Write = &mut &mut buf[..];
            client.write_tls(into_buf).unwrap()
        };
        total += sz;
        if sz == 0 {
            return total;
        }

        let mut reader = Reader::init(&buf[..sz]);
        while reader.any_left() {
            let message = OpaqueMessage::read(&mut reader).unwrap();
            let mut message = Message::try_from(message.into_plain_message()).unwrap();
            let message_enc = match filter(&mut message) {
                Altered::InPlace => PlainMessage::from(message)
                    .into_unencrypted_opaque()
                    .encode(),
                Altered::Raw(data) => data,
            };

            let message_enc_reader: &mut dyn io::Read = &mut &message_enc[..];
            let len = server.read_tls(message_enc_reader).unwrap();
            assert_eq!(len, message_enc.len());
        }
    }

    total
}

pub fn receive_altered<F>(
    server: &mut rustls::Connection,
    filter: F,
    client: &mut ClientConnection,
) -> usize
where
    F: Fn(&mut Message) -> Altered,
{
    let mut buf = [0u8; 262144];
    let mut total = 0;

    while server.wants_write() {
        let sz = {
            let into_buf: &mut dyn io::Write = &mut &mut buf[..];
            server.write_tls(into_buf).unwrap()
        };
        total += sz;
        if sz == 0 {
            return total;
        }

        let mut reader = Reader::init(&buf[..sz]);
        while reader.any_left() {
            let message = OpaqueMessage::read(&mut reader).unwrap();
            let mut message = Message::try_from(message.into_plain_message()).unwrap();
            let message_enc = match filter(&mut message) {
                Altered::InPlace => PlainMessage::from(message)
                    .into_unencrypted_opaque()
                    .encode(),
                Altered::Raw(data) => data,
            };

            let message_enc_reader: &mut dyn io::Read = &mut &message_enc[..];
            let len = client.read_tls(message_enc_reader).unwrap();
            assert_eq!(len, message_enc.len());
        }
    }

    total
}

#[derive(Clone, Copy, PartialEq)]
pub enum KeyType {
    Rsa,
    Ecdsa,
    Ed25519,
}

pub static ALL_KEY_TYPES: [KeyType; 3] = [KeyType::Rsa, KeyType::Ecdsa, KeyType::Ed25519];

impl KeyType {
    fn bytes_for(&self, part: &str) -> &'static [u8] {
        match self {
            KeyType::Rsa => bytes_for("rsa", part),
            KeyType::Ecdsa => bytes_for("ecdsa", part),
            KeyType::Ed25519 => bytes_for("eddsa", part),
        }
    }

    pub fn get_chain(&self) -> Vec<Certificate> {
        rustls_pemfile::certs(&mut io::BufReader::new(self.bytes_for("end.fullchain")))
            .unwrap()
            .iter()
            .map(|v| Certificate(v.clone()))
            .collect()
    }

    pub fn get_chain_rustls(&self) -> Vec<rustls::Certificate> {
        rustls_pemfile::certs(&mut io::BufReader::new(self.bytes_for("end.fullchain")))
            .unwrap()
            .iter()
            .map(|v| rustls::Certificate(v.clone()))
            .collect()
    }

    pub fn get_key(&self) -> PrivateKey {
        PrivateKey(
            rustls_pemfile::pkcs8_private_keys(&mut io::BufReader::new(self.bytes_for("end.key")))
                .unwrap()[0]
                .clone(),
        )
    }

    pub fn get_key_rustls(&self) -> rustls::PrivateKey {
        rustls::PrivateKey(
            rustls_pemfile::pkcs8_private_keys(&mut io::BufReader::new(self.bytes_for("end.key")))
                .unwrap()[0]
                .clone(),
        )
    }

    pub fn get_client_chain(&self) -> Vec<Certificate> {
        rustls_pemfile::certs(&mut io::BufReader::new(self.bytes_for("client.fullchain")))
            .unwrap()
            .iter()
            .map(|v| Certificate(v.clone()))
            .collect()
    }

    pub fn get_client_chain_rustls(&self) -> Vec<rustls::Certificate> {
        rustls_pemfile::certs(&mut io::BufReader::new(self.bytes_for("client.fullchain")))
            .unwrap()
            .iter()
            .map(|v| rustls::Certificate(v.clone()))
            .collect()
    }

    fn get_client_key(&self) -> PrivateKey {
        PrivateKey(
            rustls_pemfile::pkcs8_private_keys(&mut io::BufReader::new(
                self.bytes_for("client.key"),
            ))
            .unwrap()[0]
                .clone(),
        )
    }

    fn get_client_key_rustls(&self) -> rustls::PrivateKey {
        rustls::PrivateKey(
            rustls_pemfile::pkcs8_private_keys(&mut io::BufReader::new(
                self.bytes_for("client.key"),
            ))
            .unwrap()[0]
                .clone(),
        )
    }
}

pub fn finish_server_config(
    kt: KeyType,
    conf: rustls::ConfigBuilder<ServerConfig, rustls::WantsVerifier>,
) -> ServerConfig {
    conf.with_no_client_auth()
        .with_single_cert(kt.get_chain_rustls(), kt.get_key_rustls())
        .unwrap()
}

pub fn make_server_config(kt: KeyType) -> rustls::ServerConfig {
    finish_server_config(kt, ServerConfig::builder().with_safe_defaults())
}

pub fn make_server_config_with_versions(
    kt: KeyType,
    versions: &[&'static rustls::SupportedProtocolVersion],
) -> ServerConfig {
    finish_server_config(
        kt,
        ServerConfig::builder()
            .with_safe_default_cipher_suites()
            .with_safe_default_kx_groups()
            .with_protocol_versions(versions)
            .unwrap(),
    )
}

pub fn make_server_config_with_kx_groups(
    kt: KeyType,
    kx_groups: &[&'static rustls::SupportedKxGroup],
) -> ServerConfig {
    finish_server_config(
        kt,
        ServerConfig::builder()
            .with_safe_default_cipher_suites()
            .with_kx_groups(kx_groups)
            .with_safe_default_protocol_versions()
            .unwrap(),
    )
}

pub fn get_client_root_store(kt: KeyType) -> rustls::RootCertStore {
    let roots = kt.get_chain_rustls();
    let mut client_auth_roots = rustls::RootCertStore::empty();
    for root in roots {
        client_auth_roots.add(&root).unwrap();
    }
    client_auth_roots
}

pub fn make_server_config_with_mandatory_client_auth(kt: KeyType) -> ServerConfig {
    let client_auth_roots = get_client_root_store(kt);

    let client_auth = AllowAnyAuthenticatedClient::new(client_auth_roots);

    ServerConfig::builder()
        .with_safe_defaults()
        .with_client_cert_verifier(client_auth)
        .with_single_cert(kt.get_chain_rustls(), kt.get_key_rustls())
        .unwrap()
}

pub fn finish_client_config(
    kt: KeyType,
    config: tls_client::ConfigBuilder<tls_client::WantsVerifier>,
) -> ClientConfig {
    let mut root_store = RootCertStore::empty();
    let mut rootbuf = io::BufReader::new(kt.bytes_for("ca.cert"));
    root_store.add_parsable_certificates(&rustls_pemfile::certs(&mut rootbuf).unwrap());

    config
        .with_root_certificates(root_store)
        .with_no_client_auth()
}

pub fn finish_client_config_with_creds(
    kt: KeyType,
    config: tls_client::ConfigBuilder<tls_client::WantsVerifier>,
) -> ClientConfig {
    let mut root_store = RootCertStore::empty();
    let mut rootbuf = io::BufReader::new(kt.bytes_for("ca.cert"));
    root_store.add_parsable_certificates(&rustls_pemfile::certs(&mut rootbuf).unwrap());

    config
        .with_root_certificates(root_store)
        .with_single_cert(kt.get_client_chain(), kt.get_client_key())
        .unwrap()
}

pub fn make_client_config(kt: KeyType) -> ClientConfig {
    finish_client_config(kt, ClientConfig::builder().with_safe_defaults())
}

pub fn make_client_config_with_kx_groups(
    kt: KeyType,
    kx_groups: &[&'static tls_client::SupportedKxGroup],
) -> ClientConfig {
    let builder = ClientConfig::builder()
        .with_safe_default_cipher_suites()
        .with_kx_groups(kx_groups)
        .with_safe_default_protocol_versions()
        .unwrap();
    finish_client_config(kt, builder)
}

pub fn make_client_config_with_versions(
    kt: KeyType,
    versions: &[&'static tls_client::SupportedProtocolVersion],
) -> ClientConfig {
    let builder = ClientConfig::builder()
        .with_safe_default_cipher_suites()
        .with_safe_default_kx_groups()
        .with_protocol_versions(versions)
        .unwrap();
    finish_client_config(kt, builder)
}

pub fn make_client_config_with_auth(kt: KeyType) -> ClientConfig {
    finish_client_config_with_creds(kt, ClientConfig::builder().with_safe_defaults())
}

pub fn make_client_config_with_versions_with_auth(
    kt: KeyType,
    versions: &[&'static tls_client::SupportedProtocolVersion],
) -> ClientConfig {
    let builder = ClientConfig::builder()
        .with_safe_default_cipher_suites()
        .with_safe_default_kx_groups()
        .with_protocol_versions(versions)
        .unwrap();
    finish_client_config_with_creds(kt, builder)
}

pub async fn make_pair(kt: KeyType) -> (ClientConnection, ServerConnection) {
    make_pair_for_configs(make_client_config(kt), make_server_config(kt)).await
}

pub async fn make_pair_for_configs(
    client_config: ClientConfig,
    server_config: ServerConfig,
) -> (ClientConnection, ServerConnection) {
    make_pair_for_arc_configs(&Arc::new(client_config), &Arc::new(server_config)).await
}

pub async fn make_pair_for_arc_configs(
    client_config: &Arc<ClientConfig>,
    server_config: &Arc<ServerConfig>,
) -> (ClientConnection, ServerConnection) {
    let mut client = ClientConnection::new(
        Arc::clone(client_config),
        Box::new(RustCryptoBackend::new()),
        dns_name("localhost"),
    )
    .unwrap();
    client.start().await.unwrap();
    (
        client,
        ServerConnection::new(Arc::clone(server_config)).unwrap(),
    )
}

pub async fn do_handshake(
    client: &mut ClientConnection,
    server: &mut ServerConnection,
) -> (usize, usize) {
    let (mut to_client, mut to_server) = (0, 0);
    while server.is_handshaking() || client.is_handshaking() {
        to_server += send(client, server);
        server.process_new_packets().unwrap();
        to_client += receive(server, client);
        client.process_new_packets().await.unwrap();
    }
    (to_server, to_client)
}

#[derive(PartialEq, Debug)]
pub enum ErrorFromPeer {
    Client(Error),
    Server(rustls::Error),
}

pub async fn do_handshake_until_error(
    client: &mut ClientConnection,
    server: &mut ServerConnection,
) -> Result<(), ErrorFromPeer> {
    while server.is_handshaking() || client.is_handshaking() {
        send(client, server);
        server
            .process_new_packets()
            .map_err(ErrorFromPeer::Server)?;
        receive(server, client);
        client
            .process_new_packets()
            .await
            .map_err(ErrorFromPeer::Client)?;
    }

    Ok(())
}

pub async fn do_handshake_until_both_error(
    client: &mut ClientConnection,
    server: &mut ServerConnection,
) -> Result<(), Vec<ErrorFromPeer>> {
    match do_handshake_until_error(client, server).await {
        Err(server_err @ ErrorFromPeer::Server(_)) => {
            let mut errors = vec![server_err];
            receive(server, client);
            let client_err = client
                .process_new_packets()
                .await
                .map_err(ErrorFromPeer::Client)
                .expect_err("client didn't produce error after server error");
            errors.push(client_err);
            Err(errors)
        }

        Err(client_err @ ErrorFromPeer::Client(_)) => {
            let mut errors = vec![client_err];
            send(client, server);
            let server_err = server
                .process_new_packets()
                .map_err(ErrorFromPeer::Server)
                .expect_err("server didn't produce error after client error");
            errors.push(server_err);
            Err(errors)
        }

        Ok(()) => Ok(()),
    }
}

pub fn dns_name(name: &'static str) -> tls_client::ServerName {
    name.try_into().unwrap()
}

pub struct FailsReads {
    errkind: io::ErrorKind,
}

impl FailsReads {
    pub fn new(errkind: io::ErrorKind) -> Self {
        FailsReads { errkind }
    }
}

impl io::Read for FailsReads {
    fn read(&mut self, _b: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::from(self.errkind))
    }
}

pub fn version_compat(v: Option<rustls::ProtocolVersion>) -> Option<tls_client::ProtocolVersion> {
    if let Some(v) = v {
        Some(tls_client::ProtocolVersion::from(v.get_u16()))
    } else {
        None
    }
}
