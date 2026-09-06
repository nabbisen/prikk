//! `prikk trust maintainer list`/`check` output (RFC 138). Both wordings must satisfy §3.1's own
//! distinction: adopting a key means Prikk accepts its signatures on objects (Block/RefState); it
//! never lets that key move a ref. **Neither format ever says "required" or otherwise reports a
//! threshold as policy** -- `MaintainerTrustPolicy` holds a `Vec` and nothing else, trust is
//! any-of-N by construction, and this RFC adds no third site repeating the pre-existing
//! `policy: required=1` literal in the voice of a query (`main.rs`/`setup.rs` already do, out of
//! scope here).

use crate::stdout::println;
use prikk_store::{AdoptedMaintainerKey, MaintainerTrustPolicy};

use super::verification::escape_json_string;

const NOT_REF_AUTHORITY_NOTE: &str = "note: this is object trust, not ref authority -- an adopted \
     key's signatures are accepted on objects it signed, but adopting a key never lets it move a \
     ref by itself";

/// `prikk trust maintainer list`, prose. An empty policy is a successful, empty answer (RFC 138
/// control 3), never an error.
pub(crate) fn print_trust_list(policy: &MaintainerTrustPolicy) {
    if policy.keys.is_empty() {
        println!("no maintainer keys adopted");
        return;
    }
    println!("adopted maintainer keys, in adoption order:");
    for (index, key) in policy.keys.iter().enumerate() {
        println!(
            "  {}. {}  {}",
            index + 1,
            key.key_id,
            prikk_hash::to_hex(&key.public_key)
        );
    }
    println!("{NOT_REF_AUTHORITY_NOTE}");
}

/// `prikk trust maintainer list --format json`: `trust-list-v1`, named for the tool (`trust`), the
/// question (`list`), and versioned like `verify-report-v1` and this project's other
/// schema-versioned reports -- same idiom, not the same name (RFC 138 §2).
pub(crate) fn print_trust_list_json(policy: &MaintainerTrustPolicy) {
    let mut json = String::new();
    json.push_str("{\n");
    json.push_str("  \"schema_version\": \"trust-list-v1\",\n");
    json.push_str("  \"keys\": [");
    for (index, key) in policy.keys.iter().enumerate() {
        if index > 0 {
            json.push(',');
        }
        json.push_str("\n    {\"key_id\": ");
        json.push_str(&escape_json_string(&key.key_id));
        json.push_str(", \"public_key\": ");
        json.push_str(&escape_json_string(&prikk_hash::to_hex(&key.public_key)));
        json.push('}');
    }
    if !policy.keys.is_empty() {
        json.push_str("\n  ");
    }
    json.push_str("]\n}");
    println!("{json}");
}

/// `prikk trust maintainer check --key-id <ID>`, prose. Exits `0` whichever way the question
/// resolves (RFC 138 §3/RFC 121) -- the caller decides the exit code from the boolean this returns
/// unused here; this function only prints.
pub(crate) fn print_trust_check(key_id: &str, found: Option<&AdoptedMaintainerKey>) {
    match found {
        Some(key) => {
            println!(
                "trusted: {}  {}",
                key.key_id,
                prikk_hash::to_hex(&key.public_key)
            );
            println!("{NOT_REF_AUTHORITY_NOTE}");
        }
        None => println!("not trusted: {key_id}"),
    }
}

/// `prikk trust maintainer check --key-id <ID> --format json`: `trust-check-v1`.
pub(crate) fn print_trust_check_json(key_id: &str, found: Option<&AdoptedMaintainerKey>) {
    let mut json = String::new();
    json.push_str("{\n");
    json.push_str("  \"schema_version\": \"trust-check-v1\",\n");
    json.push_str(&format!("  \"key_id\": {},\n", escape_json_string(key_id)));
    json.push_str(&format!("  \"trusted\": {},\n", found.is_some()));
    match found {
        Some(key) => json.push_str(&format!(
            "  \"public_key\": {}\n",
            escape_json_string(&prikk_hash::to_hex(&key.public_key))
        )),
        None => json.push_str("  \"public_key\": null\n"),
    }
    json.push('}');
    println!("{json}");
}
