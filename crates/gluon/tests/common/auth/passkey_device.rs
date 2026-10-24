//! Real cryptographic authenticator fixtures. SoftPasskey signs ECDSA responses
//! and stores private credential keys. `true` simulates device-local UV only,
//! never the relying party's signature verification or acceptance result.
//! The backend is called directly so adversarial tests can sign an incorrect RP
//! hash without a browser-side RP/origin check masking server-side verification.
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde_json::{Value, json};
use webauthn_authenticator_rs::{
    AuthenticatorBackend,
    prelude::{CreationChallengeResponse, RequestChallengeResponse, Url},
    softpasskey::SoftPasskey,
};

pub const ORIGIN: &str = "https://app.example.test";

// A fresh packed self-attestation, with a new key but a caller-selected ID,
// models an authenticator colliding with another user's registered credential.
pub fn register_with_id(options: Value, id: &str) -> Value {
    use openssl::{bn::BigNum, bn::BigNumContext, ec::{EcGroup, EcKey}, hash::MessageDigest,
        nid::Nid, pkey::PKey, sign::Signer};
    use serde_cbor_2::value::Value as Cbor;
    use sha2::{Digest, Sha256};
    use std::collections::BTreeMap;

    let key = EcKey::generate(&EcGroup::from_curve_name(Nid::X9_62_PRIME256V1).unwrap()).unwrap();
    let (mut x, mut y, mut ctx) = (BigNum::new().unwrap(), BigNum::new().unwrap(), BigNumContext::new().unwrap());
    key.public_key().affine_coordinates_gfp(key.group(), &mut x, &mut y, &mut ctx).unwrap();
    let cose = Cbor::Map(BTreeMap::from([
        (Cbor::Integer(1), Cbor::Integer(2)),
        (Cbor::Integer(3), Cbor::Integer(-7)),
        (Cbor::Integer(-1), Cbor::Integer(1)),
        (Cbor::Integer(-2), Cbor::Bytes(x.to_vec_padded(32).unwrap())),
        (Cbor::Integer(-3), Cbor::Bytes(y.to_vec_padded(32).unwrap())),
    ]));
    let client = serde_json::to_vec(&json!({
        "type": "webauthn.create", "challenge": options["publicKey"]["challenge"],
        "origin": ORIGIN, "crossOrigin": false,
    })).unwrap();
    let id_bytes = URL_SAFE_NO_PAD.decode(id).unwrap();
    let mut auth_data = Sha256::digest(options["publicKey"]["rp"]["id"].as_str().unwrap().as_bytes()).to_vec();
    auth_data.push(0x45); // UP, UV and attested credential data.
    auth_data.extend_from_slice(&[0; 20]); // Counter and AAGUID.
    auth_data.extend_from_slice(&u16::try_from(id_bytes.len()).unwrap().to_be_bytes());
    auth_data.extend_from_slice(&id_bytes);
    auth_data.extend(serde_cbor_2::to_vec(&cose).unwrap());
    let key = PKey::from_ec_key(key).unwrap();
    let mut signer = Signer::new(MessageDigest::sha256(), &key).unwrap();
    signer.update(&auth_data).unwrap();
    signer.update(&Sha256::digest(&client)).unwrap();
    let attestation = Cbor::Map(BTreeMap::from([
        (Cbor::Text("fmt".into()), Cbor::Text("packed".into())),
        (Cbor::Text("authData".into()), Cbor::Bytes(auth_data)),
        (Cbor::Text("attStmt".into()), Cbor::Map(BTreeMap::from([
            (Cbor::Text("alg".into()), Cbor::Integer(-7)),
            (Cbor::Text("sig".into()), Cbor::Bytes(signer.sign_to_vec().unwrap())),
        ]))),
    ]));
    json!({"id": id, "rawId": id, "type": "public-key", "extensions": {},
        "response": {"clientDataJSON": URL_SAFE_NO_PAD.encode(client),
            "attestationObject": URL_SAFE_NO_PAD.encode(serde_cbor_2::to_vec(&attestation).unwrap())}})
}
pub struct Device {
    authenticator: SoftPasskey,
    user_handle: Option<Value>,
    credential_id: Option<Value>,
}
impl Device {
    pub fn new() -> Self {
        Self {
            authenticator: SoftPasskey::new(true),
            user_handle: None,
            credential_id: None,
        }
    }
    pub fn register(&mut self, options: Value) -> Value {
        self.register_at(options, ORIGIN)
    }
    pub fn register_at(&mut self, options: Value, origin: &str) -> Value {
        let user_handle = options["publicKey"]["user"]["id"].clone();
        let challenge: CreationChallengeResponse =
            serde_json::from_value(options).expect("WebAuthn registration options");
        let response = serde_json::to_value(
            self.authenticator
                .perform_register(Url::parse(origin).unwrap(), challenge.public_key, 60_000)
                .expect("real software authenticator registration"),
        )
        .unwrap();
        self.user_handle = Some(user_handle);
        self.credential_id = Some(response["id"].clone());
        response
    }
    pub fn authenticate(&mut self, options: Value) -> Value {
        self.authenticate_at(options, ORIGIN)
    }
    pub fn authenticate_at(&mut self, mut options: Value, origin: &str) -> Value {
        let allow = &options["publicKey"]["allowCredentials"];
        let discoverable = allow.is_null() || allow.as_array().is_some_and(|list| list.is_empty());
        if discoverable {
            // SoftPasskey requires an allow list. Simulate device-local credential
            // selection without changing the server's options, RP, or challenge.
            options["publicKey"]["allowCredentials"] = json!([{
                "type": "public-key",
                "id": self.credential_id.as_ref().expect("registered credential"),
            }]);
        }
        let challenge: RequestChallengeResponse =
            serde_json::from_value(options).expect("WebAuthn authentication options");
        let mut response = serde_json::to_value(
            self.authenticator
                .perform_auth(Url::parse(origin).unwrap(), challenge.public_key, 60_000)
                .expect("real software authenticator assertion"),
        )
        .unwrap();
        if discoverable {
            // userHandle is returned by a discoverable authenticator, but is not
            // part of the signed authenticatorData + clientDataJSON hash message.
            response["response"]["userHandle"] = self
                .user_handle
                .as_ref()
                .expect("registered user handle")
                .clone();
        }
        response
    }
}
pub fn corrupt_signature(response: &mut Value) {
    let mut bytes = URL_SAFE_NO_PAD
        .decode(response["response"]["signature"].as_str().unwrap())
        .unwrap();
    *bytes.last_mut().unwrap() ^= 1;
    response["response"]["signature"] = URL_SAFE_NO_PAD.encode(bytes).into();
}
pub fn client_data(response: &Value) -> Value {
    serde_json::from_slice(
        &URL_SAFE_NO_PAD
            .decode(response["response"]["clientDataJSON"].as_str().unwrap())
            .unwrap(),
    )
    .unwrap()
}
