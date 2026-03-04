#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SendCodexInputPlan {
    ForwardTurnRun { prompt: String },
    RenderStatus,
    RejectUnsupportedSlash { message: String },
}
