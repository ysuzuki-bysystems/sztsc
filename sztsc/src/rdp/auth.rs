use std::{
    io,
    process::{self, Stdio},
    string::FromUtf8Error,
};

use thiserror::Error;
use url::{ParseError, Url};

/// REF https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-rdpbcgr/e967ebeb-9e9f-443e-857a-5208802943c2

const CLIENT_ID: &'static str = "a85cf173-4192-42f8-81fa-777a763e6e2c";

const REDIRECT_URI: &'static str = "https://login.microsoftonline.com/common/oauth2/nativeclient";

pub(super) fn build_auth_code_url(scope: &str) -> Result<Url, ParseError> {
    // curl -s https://login.microsoftonline.com/common/v2.0/.well-known/openid-configuration|jq .authorization_endpoint
    let authorization_endpoint = "https://login.microsoftonline.com/common/oauth2/v2.0/authorize";
    // https://github.com/FreeRDP/FreeRDP/blob/477987f7e6002a600232ab1b1abd4f9fb32f73a9/libfreerdp/core/settings.c#L870

    url::Url::parse_with_params(
        authorization_endpoint,
        &[
            ("scope", scope.as_ref()),
            ("client_id", CLIENT_ID),
            ("response_type", "code"),
            ("redirect_uri", REDIRECT_URI),
        ],
    )
}

#[derive(Debug, Error)]
pub(super) enum GetCodeByWebviewError {
    #[error("{0}")]
    Io(#[from] io::Error),

    #[error("{0}")]
    Utf8(#[from] FromUtf8Error),

    #[error("exit without 0")]
    ExitWithoutZero,
}

pub(super) fn get_code_by_webview(bin: &str, url: &Url) -> Result<String, GetCodeByWebviewError> {
    let output = process::Command::new(bin)
        .arg(url.as_str())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .output()?;
    if !output.status.success() {
        return Err(GetCodeByWebviewError::ExitWithoutZero);
    }

    let stdout = String::from_utf8(output.stdout)?;
    Ok(stdout)
}

#[derive(Debug, Error)]
pub(super) enum GetTokenError {
    #[error("{0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("not ok")]
    NotOk,
}

#[derive(serde::Deserialize, Debug)]
pub struct TokenResponse {
    //token_type: String,
    //scope: String,
    //expires_in: u32,
    //ext_expires_in: u32,
    access_token: String,
}

pub(super) fn get_token(scope: &str, req_cnf: &str, code: &str) -> Result<String, GetTokenError> {
    let token_endpoint = "https://login.microsoftonline.com/common/oauth2/v2.0/token";

    let params: &[(&str, &str)] = &[
        ("client_id", CLIENT_ID),
        ("grant_type", "authorization_code"),
        ("code", code),
        ("scope", scope),
        ("req_cnf", req_cnf),
        ("redirect_uri", REDIRECT_URI),
    ];
    let client = reqwest::blocking::Client::new();
    let response = client.post(token_endpoint).form(params).send()?;
    if !response.status().is_success() {
        return Err(GetTokenError::NotOk);
    }

    let body = response.json::<TokenResponse>()?;
    Ok(body.access_token)
}
