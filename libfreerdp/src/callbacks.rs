use thiserror::Error;

use super::Freerdp;
use super::RdpContext;

#[derive(Debug, Error)]
pub enum CallbackError {
    #[error("{0}")]
    Any(String),
}

pub type CallbackResult<T> = ::std::result::Result<T, CallbackError>;

pub trait Callbacks {
    fn pre_connect(&mut self, context: &mut Freerdp) -> CallbackResult<()>;
    fn post_connect(&mut self, context: &mut Freerdp) -> CallbackResult<()>;
    fn post_disconnect(&mut self, instance: &mut Freerdp) -> CallbackResult<()>;

    fn verify_x509_certificate(
        &mut self,
        instance: &mut Freerdp,
        data: &[u8],
        hostname: &str,
        port: u16,
        flags: u32,
    ) -> CallbackResult<()>;
    fn get_access_token_aad(
        &mut self,
        instance: &mut Freerdp,
        scope: &str,
        req_cnf: &str,
    ) -> CallbackResult<String>;

    fn begin_paint(&mut self, instance: &mut RdpContext) -> CallbackResult<()>;
    fn end_paint(&mut self, instance: &mut RdpContext) -> CallbackResult<()>;
}
