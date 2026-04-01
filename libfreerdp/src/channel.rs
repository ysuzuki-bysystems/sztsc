use super::CliprdrClientContext;
use super::DispClientContext;

pub enum Channel {
    Disp(DispClientContext),
    Cliprdr(CliprdrClientContext),
}

pub enum ChannelName {
    Disp,
    Cliprdr,
}
