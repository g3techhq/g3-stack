//! Makes the public build-time ids visible to the client.
//!
//! The browser client reads `GOOGLE_OAUTH_CLIENT_ID` and `APPLE_SERVICES_ID`
//! with `option_env!`, so they must be in the environment of `dx build`. In
//! development that environment is a shell that has not loaded `.env`, so this
//! reads `.env` for the names below when the shell does not already have them.
//!
//! Only public identifiers belong in the list. A secret named here would be
//! compiled into the WASM bundle for anyone to read.
//!
//! A dependency such as g3-auth is built before this script runs and sees only
//! the process environment, so pass these values to its components as props
//! (`GoogleSignIn { client_id }`, `AppleSignIn { services_id }`) instead of
//! relying on the component to find them.

use std::{env, fs};

const PUBLIC_BUILD_VARIABLES: [&str; 2] = ["GOOGLE_OAUTH_CLIENT_ID", "APPLE_SERVICES_ID"];

fn main() {
    println!("cargo:rerun-if-changed=.env");
    for name in PUBLIC_BUILD_VARIABLES {
        println!("cargo:rerun-if-env-changed={name}");
    }

    let Ok(contents) = fs::read_to_string(".env") else {
        return;
    };

    for name in PUBLIC_BUILD_VARIABLES {
        if env::var(name).is_ok_and(|value| !value.trim().is_empty()) {
            continue;
        }
        let Some(value) = contents.lines().find_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let (key, value) = line.split_once('=')?;
            (key.trim() == name).then(|| {
                value
                    .trim()
                    .trim_matches(|character| matches!(character, '\'' | '"'))
            })
        }) else {
            continue;
        };
        if !value.is_empty() {
            println!("cargo:rustc-env={name}={value}");
        }
    }
}
