use super::super::SourceId;

#[derive(Debug)]
pub(crate) enum TrashMoveMessage {
    SetTotal(usize),
    Progress {
        completed: usize,
        detail: Option<String>,
    },
    Finished(TrashMoveFinished),
}

#[derive(Clone, Debug)]
pub(crate) struct TrashMoveFinished {
    pub(crate) total: usize,
    pub(crate) moved: usize,
    pub(crate) cancelled: bool,
    pub(crate) errors: Vec<String>,
    pub(crate) affected_sources: Vec<SourceId>,
}
