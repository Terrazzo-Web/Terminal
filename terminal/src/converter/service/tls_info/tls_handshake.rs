use scopeguard::guard;
use tls_parser::TlsCertificateContents;
use tls_parser::TlsCertificateRequestContents;
use tls_parser::TlsClientHelloContents;
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
use tls_parser::parse_tls_extensions;

use super::indented_writer::Indented;
use super::indented_writer::Writer;
use crate::converter::api::Language;
use crate::converter::service::AddConversionFn;

pub fn add_tls_handshake(name: &'static str, mut buffer: &[u8], add: &mut impl AddConversionFn) {
    tracing::debug!("Adding TLS handshake info");
    let writer = Writer::new();
    let mut writer = guard(writer, |w| add(Language::new(name), w.to_string()));
    while let Ok((rest, plaintext)) = tls_parser::parse_tls_plaintext(&buffer) {
        buffer = rest;
        write_tls_plaintext(&mut *writer, plaintext);
    }
}

fn write_tls_plaintext(w: &mut Writer, plaintext: TlsPlaintext<'_>) {
    let TlsPlaintext {
        hdr: header,
        msg: messages,
    } = plaintext;
    {
        let mut w = w.write("Header").indent();
        let TlsRecordHeader {
            record_type,
            version,
            len: _,
        } = header;
        w.debug(record_type).writeln();
        w.debug(version).writeln();
    }
    {
        let mut w = guard(w, |w| {
            w.write("]").writeln();
        });
        let mut w = w.write("Message [").indent();
        for message in messages {
            write_tls_message(&mut w, message);
        }
    }
}

fn write_tls_message(w: &mut Indented<'_>, message: TlsMessage<'_>) {
    match message {
        TlsMessage::Handshake(handshake) => {
            write_handshake(w, handshake);
        }
        TlsMessage::ChangeCipherSpec => {
            w.write("ChangeCipherSpec");
        }
        TlsMessage::Alert(alert) => {
            w.write("Alert: ").debug(alert);
        }
        TlsMessage::ApplicationData(_data) => {
            w.write("ApplicationData");
        }
        TlsMessage::Heartbeat(heartbeat) => {
            w.write("Heartbeat: ").debug(heartbeat.heartbeat_type);
        }
    }
}

fn write_handshake(w: &mut Indented<'_>, handshake: TlsMessageHandshake<'_>) {
    let mut w = w.write("Handshake").indent();
    match handshake {
        TlsMessageHandshake::HelloRequest => {
            w.write("HelloRequest");
        }
        TlsMessageHandshake::ClientHello(TlsClientHelloContents {
            version,
            random,
            session_id,
            ciphers,
            comp: compression,
            ext: extensions,
        }) => {
            let w = w.write("ClientHello").indent();
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
            let w = w.write("ServerHello").indent();
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
            version: _,
            random: _,
            cipher: _,
            ext: _,
        }) => {
            w.write("ServerHelloV13Draft18");
        }
        TlsMessageHandshake::NewSessionTicket(TlsNewSessionTicketContent {
            ticket_lifetime_hint,
            ticket,
        }) => {
            w.write("NewSessionTicket");
            w.write(&format!(
                ": hint={} ticket={}",
                ticket_lifetime_hint,
                hex(ticket)
            ));
        }
        TlsMessageHandshake::EndOfEarlyData => {
            w.write("EndOfEarlyData");
        }
        TlsMessageHandshake::HelloRetryRequest(TlsHelloRetryRequestContents {
            version: _,
            cipher: _,
            ext: _,
        }) => {
            w.write("HelloRetryRequest");
        }
        TlsMessageHandshake::Certificate(TlsCertificateContents { cert_chain: _ }) => {
            w.write("Certificate");
        }
        TlsMessageHandshake::ServerKeyExchange(TlsServerKeyExchangeContents { parameters: _ }) => {
            w.write("ServerKeyExchange");
        }
        TlsMessageHandshake::CertificateRequest(TlsCertificateRequestContents {
            cert_types: _,
            sig_hash_algs: _,
            unparsed_ca: _,
        }) => {
            w.write("CertificateRequest");
        }
        TlsMessageHandshake::ServerDone(_) => {
            w.write("ServerDone");
        }
        TlsMessageHandshake::CertificateVerify(_) => {
            w.write("CertificateVerify");
        }
        TlsMessageHandshake::ClientKeyExchange(_) => {
            w.write("ClientKeyExchange");
        }
        TlsMessageHandshake::Finished(_) => {
            w.write("Finished");
        }
        TlsMessageHandshake::CertificateStatus(_) => {
            w.write("CertificateStatus");
        }
        TlsMessageHandshake::NextProtocol(_) => {
            w.write("NextProtocol");
        }
        TlsMessageHandshake::KeyUpdate(_) => {
            w.write("KeyUpdate");
        }
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

fn write_extension(w: &mut Indented<'_>, extension: &TlsExtension<'_>) {
    let mut w = guard(w, |w| {
        w.writeln();
    });
    match extension {
        TlsExtension::SNI(items) => {
            let mut w = w.write("SNI").indent();
            for (sni_type, sni) in items {
                w.print(sni_type)
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
        TlsExtension::SignatureAlgorithms(_) => {
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
