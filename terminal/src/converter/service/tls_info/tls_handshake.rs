use scopeguard::guard;
use tls_parser::SignatureScheme;
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
use tls_parser::TlsRawRecord;
use tls_parser::TlsRecordType;
use tls_parser::TlsServerHelloContents;
use tls_parser::TlsServerHelloV13Draft18Contents;
use tls_parser::TlsServerKeyExchangeContents;
use tls_parser::TlsVersion;
use tls_parser::parse_tls_extensions;
use tls_parser::parse_tls_raw_record;
use tls_parser::parse_tls_record_with_header;
use tracing::debug;
use x509_parser::asn1_rs::FromDer;
use x509_parser::prelude::X509Certificate;

use super::indented_writer::Writer;
use crate::converter::api::Language;
use crate::converter::service::AddConversionFn;

pub fn add_tls_handshake(name: &'static str, mut buffer: &[u8], add: &mut impl AddConversionFn) {
    debug!("Adding TLS handshake info");
    let writer = Writer::new();
    let mut w = guard(writer, |w| add(Language::new(name), w.to_string()));

    loop {
        if buffer.is_empty() {
            break;
        }
        let raw_record = match parse_tls_raw_record(buffer) {
            Ok((rest, raw_record)) => {
                buffer = rest;
                raw_record
            }
            Err(error) => {
                w.write("TlsRawRecord ERROR");
                if cfg!(feature = "debug") {
                    w.write(": ").print(error);
                }
                break;
            }
        };
        write_tls_record(&mut w, raw_record);
    }
}

fn write_tls_record(w: &mut Writer, raw_record: TlsRawRecord<'_>) {
    if !cfg!(feature = "debug") {
        match raw_record.hdr.record_type {
            TlsRecordType::ChangeCipherSpec | TlsRecordType::ApplicationData => return,
            _ => (),
        }
    }
    let mut w = w
        .write("Record: ")
        .debug(raw_record.hdr.record_type)
        .indent();

    let mut buffer = raw_record.data;
    loop {
        if buffer.is_empty() {
            break;
        }
        let messages = match parse_tls_record_with_header(buffer, &raw_record.hdr) {
            Ok((rest, messages)) => {
                buffer = rest;
                messages
            }
            Err(error) => {
                w.write("TlsMessages ERROR");
                if cfg!(feature = "debug") {
                    w.write(": ").print(error);
                }
                break;
            }
        };
        for message in messages {
            write_tls_message(&mut w, message);
            w.writeln();
        }
    }
}

fn write_tls_message(w: &mut Writer, message: TlsMessage<'_>) {
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

fn write_handshake(w: &mut Writer, handshake: TlsMessageHandshake<'_>) {
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
            let mut w = w.write("ClientHello").indent();
            write_hello(
                &mut *w,
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
            let mut w = w.write("ServerHello").indent();
            write_hello(
                &mut *w,
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
        TlsMessageHandshake::Certificate(TlsCertificateContents { cert_chain }) => {
            let mut w = w.write("Certificate").indent();
            for certificate in cert_chain {
                match X509Certificate::from_der(certificate.data) {
                    Ok((_rest, certificate)) => {
                        w.print(certificate.subject()).writeln();
                    }
                    Err(error) => {
                        w.print(error).writeln();
                    }
                }
            }
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
    w: &mut Writer,
    version: TlsVersion,
    random: &[u8],
    session_id: Option<&[u8]>,
    ciphers: Vec<TlsCipherSuiteID>,
    compression: Vec<TlsCompressionID>,
    extensions: Option<&[u8]>,
) {
    w.write("Version: ").debug(version).writeln();
    w.write("Random: ").print(hex(random)).writeln();
    if let Some(session_id) = session_id {
        w.write("Session ID: ").print(hex(session_id)).writeln();
    }
    if !ciphers.is_empty() {
        let mut w = w.write("Ciphers").indent();
        for cipher in ciphers {
            w.debug(cipher).writeln();
        }
    }
    if !compression.is_empty() {
        let mut w = w.write("Compression").indent();
        for compression in compression {
            w.print(compression).writeln();
        }
    }
    if let Some(extensions) = extensions {
        let Ok((_rest, extensions)) = parse_tls_extensions(extensions) else {
            return;
        };
        write_extensions(&mut w.write("Extensions").indent(), &extensions);
    }
}

fn write_extensions(w: &mut Writer, extensions: &[TlsExtension<'_>]) {
    for extension in extensions {
        write_extension(w, extension);
    }
}

fn write_extension(w: &mut Writer, extension: &TlsExtension<'_>) {
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
            let mut w = w.write("StatusRequest").indent();
            if let Some((t, s)) = v {
                w.write("type='").debug(t).write("'").writeln();
                w.write("data='")
                    .write(&String::from_utf8_lossy(s))
                    .write("'");
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
        TlsExtension::SignatureAlgorithms(algs) => {
            let mut w = w.write("SignatureSchemes").indent();
            for alg in algs {
                w.print(SignatureScheme(*alg)).writeln();
            }
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
            let mut w = w.write("SupportedVersions").indent();
            for tls_version in tls_versions {
                w.print(tls_version).writeln();
            }
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
