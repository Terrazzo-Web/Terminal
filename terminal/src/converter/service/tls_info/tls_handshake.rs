#![expect(unused)]

use scopeguard::guard;
use tls_parser::TlsCertificateContents;
use tls_parser::TlsCertificateRequestContents;
use tls_parser::TlsCipherSuiteID;
use tls_parser::TlsClientHelloContents;
use tls_parser::TlsCompressionID;
use tls_parser::TlsExtension;
use tls_parser::TlsHelloRetryRequestContents;
use tls_parser::TlsMessage;
use tls_parser::TlsMessageHandshake;
use tls_parser::TlsNewSessionTicketContent;
use tls_parser::TlsPlaintext;
use tls_parser::TlsRecordHeader;
use tls_parser::TlsServerHelloContents;
use tls_parser::TlsServerHelloV13Draft18Contents;
use tls_parser::TlsServerKeyExchangeContents;
use tls_parser::TlsVersion;
use tls_parser::parse_tls_extensions;
use tls_parser::parse_tls_plaintext;

use super::indented_writer::Indented;
use crate::converter::api::Language;
use crate::converter::service::AddConversionFn;

pub fn add_tls_handshake(mut buffer: &[u8], add: &mut impl AddConversionFn) {
    tracing::debug!("Adding TLS handshake info");
    let mut writer = super::indented_writer::Writer::new();
    let mut writer = guard(writer, |w| {
        add(Language::new("TLS handshake"), w.to_string())
    });
    while let Ok((rest, plaintext)) = tls_parser::parse_tls_plaintext(&buffer) {
        buffer = rest;
        let TlsPlaintext { hdr, msg } = plaintext;
        {
            let mut header = writer.write("Header").indent();
            let TlsRecordHeader {
                record_type,
                version,
                len: _,
            } = hdr;
            header.debug(record_type).writeln();
            header.debug(version).writeln();
        }
        {
            let mut message = guard(&mut writer, |w| {
                w.write("]").writeln();
            });
            let mut message = message.write("Message [").indent();
            for msg in msg {
                match msg {
                    TlsMessage::Handshake(msg) => {
                        write_handshake(&mut message, msg);
                    }
                    TlsMessage::ChangeCipherSpec => {
                        message.write("ChangeCipherSpec");
                    }
                    TlsMessage::Alert(msg) => {}
                    TlsMessage::ApplicationData(msg) => {}
                    TlsMessage::Heartbeat(msg) => {}
                }
            }
        }
    }
}

fn write_handshake(message: &mut Indented<'_>, msg: TlsMessageHandshake<'_>) {
    let mut handshake = message.write("Handshake").indent();
    match msg {
        TlsMessageHandshake::HelloRequest => {
            handshake.write("HelloRequest");
        }
        TlsMessageHandshake::ClientHello(TlsClientHelloContents {
            version,
            random,
            session_id,
            ciphers,
            comp: compression,
            ext: extensions,
        }) => {
            let mut w = handshake.write("ClientHello").indent();
            write_hello(
                w,
                version,
                random,
                session_id,
                ciphers,
                compression,
                extensions,
            );
        }
        TlsMessageHandshake::ServerHello(TlsServerHelloContents {
            version,
            random,
            session_id,
            cipher,
            compression,
            ext: extensions,
        }) => {
            let mut w = handshake.write("ServerHello").indent();
            write_hello(
                w,
                version,
                random,
                session_id,
                vec![cipher],
                vec![compression],
                extensions,
            );
        }
        TlsMessageHandshake::ServerHelloV13Draft18(TlsServerHelloV13Draft18Contents {
            version,
            random,
            cipher,
            ext,
        }) => {}
        TlsMessageHandshake::NewSessionTicket(TlsNewSessionTicketContent {
            ticket_lifetime_hint,
            ticket,
        }) => {}
        TlsMessageHandshake::EndOfEarlyData => {}
        TlsMessageHandshake::HelloRetryRequest(TlsHelloRetryRequestContents {
            version,
            cipher,
            ext,
        }) => {}
        TlsMessageHandshake::Certificate(TlsCertificateContents { cert_chain }) => {}
        TlsMessageHandshake::ServerKeyExchange(TlsServerKeyExchangeContents { parameters }) => {}
        TlsMessageHandshake::CertificateRequest(TlsCertificateRequestContents {
            cert_types,
            sig_hash_algs,
            unparsed_ca,
        }) => {}
        TlsMessageHandshake::ServerDone(items) => {}
        TlsMessageHandshake::CertificateVerify(items) => {}
        TlsMessageHandshake::ClientKeyExchange(msg) => {}
        TlsMessageHandshake::Finished(items) => {}
        TlsMessageHandshake::CertificateStatus(msg) => {}
        TlsMessageHandshake::NextProtocol(msg) => {}
        TlsMessageHandshake::KeyUpdate(_) => {}
    }
}

fn write_hello(
    mut w: Indented<'_>,
    version: tls_parser::TlsVersion,
    random: &[u8],
    session_id: Option<&[u8]>,
    ciphers: Vec<tls_parser::TlsCipherSuiteID>,
    compression: Vec<tls_parser::TlsCompressionID>,
    extensions: Option<&[u8]>,
) {
    w.write("Version: ").debug(version).writeln();
    w.write("Random: ").print(hex(random)).writeln();
    if let Some(session_id) = session_id {
        w.write("Session ID: ").print(hex(session_id)).writeln();
    }
    if !ciphers.is_empty() {
        w.write("Ciphers: ").debug(ciphers).writeln();
    }
    if !compression.is_empty() {
        w.write("Compression: ").debug(compression).writeln();
    }
    if let Some(extensions) = extensions {
        let Ok((_rest, extensions)) = parse_tls_extensions(extensions) else {
            return;
        };
        write_extensions(&mut w.write("Extensions").indent(), &extensions);
    }
}

fn write_extensions(w: &mut Indented<'_>, extensions: &[TlsExtension<'_>]) {
    for extension in extensions {
        write_extension(w, extension);
    }
}

fn write_extension(mut w: &mut Indented<'_>, extension: &TlsExtension<'_>) {
    let mut w = guard(w, |w| {
        w.writeln();
    });
    match extension {
        TlsExtension::SNI(items) => {
            let mut w = w.write("SNI").indent();
            for (sni_type, sni) in items {
                w.debug(sni_type)
                    .write(": ")
                    .write(&String::from_utf8_lossy(sni))
                    .writeln();
            }
        }
        TlsExtension::MaxFragmentLength(v) => {
            w.write("MaxFragmentLength: ").print(v);
        }
        TlsExtension::StatusRequest(v) => {
            w.write("StatusRequest: ");
            if let Some((t, s)) = v {
                w.debug(t).write(" = ").write(&String::from_utf8_lossy(s));
            } else {
                w.write("None");
            }
        }
        TlsExtension::EllipticCurves(named_groups) => {
            let mut w = w.write("EllipticCurves").indent();
            for named_group in named_groups {
                w.debug(named_group).writeln();
            }
        }
        TlsExtension::EcPointFormats(data) => {
            w.write("EcPointFormats");
            w.write(&format!(" ({})", data.len()));
        }
        TlsExtension::SignatureAlgorithms(items) => {
            w.debug(extension);
        }
        TlsExtension::RecordSizeLimit(limit) => {
            w.write("RecordSizeLimit: ").print(limit);
        }
        TlsExtension::SessionTicket(data) => {
            w.write("SessionTicket");
            w.write(&format!(" ({})", data.len()));
        }
        TlsExtension::KeyShareOld(data) => {
            w.write("KeyShareOld");
            w.write(&format!(" ({})", data.len()));
        }
        TlsExtension::KeyShare(data) => {
            w.write("KeyShare");
            w.write(&format!(" ({})", data.len()));
        }
        TlsExtension::PreSharedKey(data) => {
            w.write("PreSharedKey");
            w.write(&format!(" ({})", data.len()));
        }
        TlsExtension::EarlyData(len) => {
            w.write("EarlyData");
            if let Some(len) = len {
                w.write(&format!(" ({len})"));
            }
        }
        TlsExtension::SupportedVersions(tls_versions) => {
            w.write("SupportedVersions: ").debug(tls_versions);
        }
        TlsExtension::Cookie(data) => {
            w.write("Cookie");
            w.write(&format!(" ({})", data.len()));
        }
        TlsExtension::PskExchangeModes(data) => {
            w.write("PskExchangeModes");
            w.write(&format!(" ({})", data.len()));
        }
        TlsExtension::Heartbeat(h) => {
            w.write("Heartbeat").write(&format!(" ({h})"));
        }
        TlsExtension::ALPN(items) => {
            let mut w = w.write("ALPN").indent();
            for alpn in items {
                w.write("- ")
                    .write(&String::from_utf8_lossy(alpn))
                    .writeln();
            }
        }
        TlsExtension::SignedCertificateTimestamp(_data) => {
            w.write("SignedCertificateTimestamp");
        }
        TlsExtension::Padding(data) => {
            let mut w = w.write("Padding").indent();
            w.write(&format!(" ({})", data.len()));
        }
        TlsExtension::EncryptThenMac => {
            w.write("EncryptThenMac");
        }
        TlsExtension::ExtendedMasterSecret => {
            w.write("ExtendedMasterSecret");
        }
        TlsExtension::OidFilters(_oid_filters) => {
            w.write("OidFilters");
        }
        TlsExtension::PostHandshakeAuth => {
            w.write("PostHandshakeAuth");
        }
        TlsExtension::NextProtocolNegotiation => {
            w.write("NextProtocolNegotiation");
        }
        TlsExtension::RenegotiationInfo(data) => {
            w.write("RenegotiationInfo");
            w.write(&format!(" ({})", data.len()));
        }
        TlsExtension::EncryptedServerName {
            ciphersuite,
            group,
            key_share,
            record_digest,
            encrypted_sni,
        } => {
            let mut w = w.write("EncryptedServerName").indent();
            w.write("Cipher suite: ").debug(ciphersuite).writeln();
            w.write("Group: ").debug(group).writeln();
            w.write(&format!(
                "key_share:{}b record_digest:{}b encrypted_sni:{}b",
                key_share.len(),
                record_digest.len(),
                encrypted_sni.len()
            ));
        }
        TlsExtension::Grease(grease, data) => {
            w.write("Grease: ").print(grease);
            w.write(&format!(" ({})", data.len()));
        }
        TlsExtension::Unknown(tls_extension_type, data) => {
            w.write("Unknown: ").debug(tls_extension_type);
            w.write(&format!(" ({})", data.len()));
        }
    }
}

fn hex(data: &[u8]) -> String {
    data.iter()
        .map(|b| format!("{:02X}", b))
        .collect::<String>()
}
